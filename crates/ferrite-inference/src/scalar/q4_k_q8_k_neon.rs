#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, q4_k::Q4_K_BLOCK_BYTES, q8_k::BlockQ8K, InferenceError};
use std::arch::aarch64::{
    int32x4_t, int8x16_t, uint8x16_t, vaddq_s32, vaddvq_s32, vandq_u8, vdupq_n_u8, vget_high_s8,
    vget_low_s8, vld1q_s8, vld1q_u8, vmull_s8, vpaddlq_s16, vreinterpretq_s8_u8, vshrq_n_u8,
};

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
    use super::neon_q4_k_q8_k_block_dot;
    use crate::scalar::{
        q4_k::Q4_K_BLOCK_VALUES, q4_k_q8_k::q4_k_q8_k_block_dot, q8_k::BlockQ8K, InferenceError,
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

    fn patterned_q4_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend_from_slice(&[3, 5, 7, 9, 11, 13, 15, 17, 2, 4, 6, 8]);
        for index in 0..128 {
            let low = (index as u8).wrapping_mul(3) & 0x0f;
            let high = (index as u8).wrapping_mul(5).wrapping_add(1) & 0x0f;
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
}
