#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q4_k::{q4_k_storage_bytes, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    q8_k::BlockQ8K,
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int32x4_t, int8x16_t, uint8x16_t, vaddq_s32, vaddvq_s32, vandq_u8, vdupq_n_u8, vget_high_s8,
    vget_low_s8, vld1q_s8, vld1q_u8, vmull_s8, vpaddlq_s16, vreinterpretq_s8_u8, vshrq_n_u8,
};

pub(in crate::scalar) fn neon_q4_k_q8_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    validate_neon_q4_k_q8_k_mul_vec(bytes, rows, cols, vector)?;
    let activation_blocks = BlockQ8K::quantize_blocks(vector)?;
    let blocks_per_row = cols / Q4_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row
        .checked_mul(Q4_K_BLOCK_BYTES)
        .ok_or_else(|| InferenceError::new("Q4_K row byte length overflow"))?;

    let values = bytes
        .par_chunks_exact(row_bytes)
        .map(|row| {
            row.chunks_exact(Q4_K_BLOCK_BYTES)
                .enumerate()
                .map(|(block_index, block)| {
                    neon_q4_k_q8_k_block_dot(block, &activation_blocks[block_index])
                })
                .collect::<Result<Vec<_>, InferenceError>>()
                .map(|parts| parts.iter().sum())
        })
        .collect::<Result<Vec<_>, InferenceError>>()?;
    debug_assert_eq!(values.len(), rows);

    Ok(values)
}

fn validate_neon_q4_k_q8_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(), InferenceError> {
    if cols == 0 {
        return Err(InferenceError::new("Q4_K Q8_K columns must not be zero"));
    }
    if !cols.is_multiple_of(Q4_K_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q4_K Q8_K columns {cols} must be divisible by {Q4_K_BLOCK_VALUES}"
        )));
    }
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
    let expected = q4_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q4_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    Ok(())
}

pub(in crate::scalar) fn neon_q4_k_q8_k_block_dot(
    block: &[u8],
    activation: &BlockQ8K,
) -> Result<f32, InferenceError> {
    if block.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q4_K block byte length {} does not match {Q4_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    // SAFETY: this module is compiled only for aarch64, where NEON is part of
    // the baseline architecture. `block` has been length-checked above, and
    // `BlockQ8K` always owns exactly one 256-value activation block.
    Ok(unsafe { neon_q4_k_q8_k_block_dot_unchecked(block, activation) })
}

#[target_feature(enable = "neon")]
unsafe fn neon_q4_k_q8_k_block_dot_unchecked(block: &[u8], activation: &BlockQ8K) -> f32 {
    let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    let scales = &block[4..16];
    let quants = &block[16..];
    let mask = vdupq_n_u8(0x0f);
    let mut weighted_sum = 0i32;
    let mut min_sum = 0i32;
    let mut scale_index = 0usize;
    let mut activation_offset = 0usize;

    for quant_chunk in quants.chunks_exact(32) {
        let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);

        let q8_low = activation.qs.as_ptr().add(activation_offset);
        let q8_high = activation.qs.as_ptr().add(activation_offset + 32);
        let low_dot = q4_nibble_dot_32(quant_chunk.as_ptr(), q8_low, mask, Q4Nibble::Low);
        let high_dot = q4_nibble_dot_32(quant_chunk.as_ptr(), q8_high, mask, Q4Nibble::High);

        weighted_sum += i32::from(scale_low) * low_dot + i32::from(scale_high) * high_dot;
        min_sum += i32::from(min_low)
            * (i32::from(activation.bsums[scale_index * 2])
                + i32::from(activation.bsums[scale_index * 2 + 1]));
        min_sum += i32::from(min_high)
            * (i32::from(activation.bsums[(scale_index + 1) * 2])
                + i32::from(activation.bsums[(scale_index + 1) * 2 + 1]));

        scale_index += 2;
        activation_offset += 64;
    }

    activation.d * (d * weighted_sum as f32 - dmin * min_sum as f32)
}

#[derive(Clone, Copy)]
enum Q4Nibble {
    Low,
    High,
}

#[target_feature(enable = "neon")]
unsafe fn q4_nibble_dot_32(
    q4: *const u8,
    q8: *const i8,
    mask: uint8x16_t,
    nibble: Q4Nibble,
) -> i32 {
    let q4_a = vld1q_u8(q4);
    let q4_b = vld1q_u8(q4.add(16));
    let q4_a = q4_nibble_lanes(q4_a, mask, nibble);
    let q4_b = q4_nibble_lanes(q4_b, mask, nibble);
    let q8_a = vld1q_s8(q8);
    let q8_b = vld1q_s8(q8.add(16));

    vaddvq_s32(vaddq_s32(dot_i8x16(q4_a, q8_a), dot_i8x16(q4_b, q8_b)))
}

#[target_feature(enable = "neon")]
unsafe fn q4_nibble_lanes(values: uint8x16_t, mask: uint8x16_t, nibble: Q4Nibble) -> int8x16_t {
    match nibble {
        Q4Nibble::Low => vreinterpretq_s8_u8(vandq_u8(values, mask)),
        Q4Nibble::High => vreinterpretq_s8_u8(vshrq_n_u8(values, 4)),
    }
}

#[target_feature(enable = "neon")]
unsafe fn dot_i8x16(left: int8x16_t, right: int8x16_t) -> int32x4_t {
    let products_low = vmull_s8(vget_low_s8(left), vget_low_s8(right));
    let products_high = vmull_s8(vget_high_s8(left), vget_high_s8(right));
    vaddq_s32(vpaddlq_s16(products_low), vpaddlq_s16(products_high))
}

fn q4_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{neon_q4_k_q8_k_block_dot, neon_q4_k_q8_k_mul_vec};
    use crate::scalar::{
        q4_k::Q4_K_BLOCK_VALUES,
        q4_k_q8_k::{q4_k_q8_k_block_dot, q4_k_q8_k_mul_vec},
        q8_k::BlockQ8K,
        InferenceError,
    };

    #[test]
    fn neon_q4_k_q8_k_block_dot_matches_scalar_q8_k_dot() -> Result<(), InferenceError> {
        let block = patterned_q4_k_block();
        let vector = patterned_activation();
        let activation = BlockQ8K::quantize(&vector)?;

        let actual = neon_q4_k_q8_k_block_dot(&block, &activation)?;
        let expected = q4_k_q8_k_block_dot(&block, &activation)?;

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    #[test]
    fn neon_q4_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales() -> Result<(), InferenceError>
    {
        let block = patterned_q4_k_block();

        for vector in [
            positive_dominant_activation(),
            negative_dominant_activation(),
        ] {
            let activation = BlockQ8K::quantize(&vector)?;
            let actual = neon_q4_k_q8_k_block_dot(&block, &activation)?;
            let expected = q4_k_q8_k_block_dot(&block, &activation)?;

            assert!(
                (actual - expected).abs() < 0.001,
                "actual={actual} expected={expected}"
            );
        }

        Ok(())
    }

    #[test]
    fn neon_q4_k_q8_k_mul_vec_matches_scalar_q8_k_adapter() -> Result<(), InferenceError> {
        let cols = Q4_K_BLOCK_VALUES * 2;
        let rows = 2;
        let vector = (0..cols)
            .map(|index| (index % 47) as f32 / 15.0 - 1.6)
            .collect::<Vec<_>>();
        let bytes = [
            patterned_q4_k_block_with_seed(0),
            patterned_q4_k_block_with_seed(1),
            patterned_q4_k_block_with_seed(2),
            patterned_q4_k_block_with_seed(3),
        ]
        .concat();

        let actual = neon_q4_k_q8_k_mul_vec(&bytes, rows, cols, &vector)?;
        let expected = q4_k_q8_k_mul_vec(&bytes, rows, cols, &vector)?;

        assert_eq!(actual.len(), rows);
        for (actual, expected) in actual.iter().zip(&expected) {
            assert!(
                (actual - expected).abs() < 0.001,
                "actual={actual} expected={expected}"
            );
        }
        Ok(())
    }

    #[test]
    fn neon_q4_k_q8_k_mul_vec_rejects_partial_block_columns() -> Result<(), InferenceError> {
        let bytes = patterned_q4_k_block();
        let vector = vec![1.0; Q4_K_BLOCK_VALUES / 2];

        let err = match neon_q4_k_q8_k_mul_vec(&bytes, 2, Q4_K_BLOCK_VALUES / 2, &vector) {
            Ok(_) => return Err(InferenceError::new("partial-block columns must fail")),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "Q4_K Q8_K columns 128 must be divisible by 256"
        );
        Ok(())
    }

    fn patterned_q4_k_block() -> Vec<u8> {
        patterned_q4_k_block_with_seed(0)
    }

    fn patterned_q4_k_block_with_seed(seed: u8) -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend(
            [3, 5, 7, 9, 11, 13, 15, 17, 2, 4, 6, 8]
                .into_iter()
                .map(|value| value + seed),
        );
        for index in 0..128 {
            let low = (index as u8).wrapping_mul(3).wrapping_add(seed) & 0x0f;
            let high = (index as u8)
                .wrapping_mul(5)
                .wrapping_add(1)
                .wrapping_add(seed)
                & 0x0f;
            block.push(low | (high << 4));
        }
        block
    }

    fn patterned_activation() -> [f32; Q4_K_BLOCK_VALUES] {
        let mut values = [0.0; Q4_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let wave = (index % 41) as f32 - 20.0;
            *value = wave / 13.0;
        }
        values
    }

    fn positive_dominant_activation() -> [f32; Q4_K_BLOCK_VALUES] {
        let mut values = patterned_activation();
        values[0] = 3.25;
        values[1] = -1.5;
        values
    }

    fn negative_dominant_activation() -> [f32; Q4_K_BLOCK_VALUES] {
        let mut values = patterned_activation();
        values[0] = -3.25;
        values[1] = 1.5;
        values
    }
}
