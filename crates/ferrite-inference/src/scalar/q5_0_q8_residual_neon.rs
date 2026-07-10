//! Experimental Q5_0 × two-pass residual-Q8 matvec for FEAT_DotProd CPUs.
#![allow(unsafe_code)]

use super::{
    neon_util::native_f16_bits_to_f32,
    q5_0::{Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
    q5_0_neon::q5_signed_offsets,
    q8_residual_activation::{BlockQ8Residual, Q8_RESIDUAL_PASSES},
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int32x4_t, int8x16_t, vaddq_s32, vaddq_s8, vaddvq_s32, vandq_u8, vcombine_s8, vdupq_n_s32,
    vdupq_n_u8, vget_high_s8, vget_low_s8, vgetq_lane_s32, vld1q_s8, vld1q_u8, vreinterpretq_s8_u8,
    vshrq_n_u8,
};
use std::arch::asm;

const ROW_PARALLEL_MIN_ROWS: usize = 512;
const ROW_PARALLEL_MIN_ROWS_PER_TASK: usize = 128;
const PAIRED_MATRICES_ROWS_PER_TASK: usize = 256;

pub(super) fn neon_q5_0_q8_residual_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q5_0_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid Q5_0 x residual-Q8 matvec shape",
        ));
    }
    let activation = BlockQ8Residual::quantize_blocks(vector)?;
    neon_q5_0_q8_residual_mul_vec_prequantized(bytes, rows, cols, &activation)
}

pub(super) fn neon_q5_0_q8_residual_mul_vec_prequantized(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    activation: &[BlockQ8Residual],
) -> Result<Vec<f32>, InferenceError> {
    if cols == 0
        || !cols.is_multiple_of(Q5_0_BLOCK_VALUES)
        || activation.len() != cols / Q5_0_BLOCK_VALUES
    {
        return Err(InferenceError::new(
            "invalid prequantized Q5_0 x residual-Q8 matvec shape",
        ));
    }
    let row_bytes = cols / Q5_0_BLOCK_VALUES * Q5_0_BLOCK_BYTES;
    if bytes.len() != rows * row_bytes {
        return Err(InferenceError::new(
            "invalid Q5_0 x residual-Q8 matrix byte length",
        ));
    }
    if rows >= 2 && std::arch::is_aarch64_feature_detected!("i8mm") {
        return Ok(neon_q5_0_q8_residual_mul_vec_i8mm(
            bytes, rows, row_bytes, activation,
        ));
    }
    let values = if rows >= ROW_PARALLEL_MIN_ROWS {
        bytes
            .par_chunks_exact(row_bytes)
            .with_min_len(ROW_PARALLEL_MIN_ROWS_PER_TASK)
            .map(|row| row_dot(row, activation))
            .collect()
    } else {
        bytes
            .chunks_exact(row_bytes)
            .map(|row| row_dot(row, activation))
            .collect()
    };
    Ok(values)
}

/// Multiplies same-shaped gate/up matrices after quantizing their shared
/// activation once. Each matrix retains the same block accumulation order as
/// [`neon_q5_0_q8_residual_mul_vec`].
pub(super) fn neon_q5_0_q8_residual_mul_vec_pair(
    left_bytes: &[u8],
    right_bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(Vec<f32>, Vec<f32>), InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q5_0_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid paired Q5_0 x residual-Q8 matvec shape",
        ));
    }
    let row_bytes = cols / Q5_0_BLOCK_VALUES * Q5_0_BLOCK_BYTES;
    let expected_bytes = rows * row_bytes;
    if left_bytes.len() != expected_bytes || right_bytes.len() != expected_bytes {
        return Err(InferenceError::new(
            "invalid paired Q5_0 x residual-Q8 matrix byte length",
        ));
    }

    let activation = BlockQ8Residual::quantize_blocks(vector)?;
    let mut left_values = vec![0.0; rows];
    let mut right_values = vec![0.0; rows];
    let use_i8mm = std::arch::is_aarch64_feature_detected!("i8mm");
    left_values
        .par_iter_mut()
        .zip(right_values.par_iter_mut())
        .zip(left_bytes.par_chunks_exact(row_bytes))
        .zip(right_bytes.par_chunks_exact(row_bytes))
        .with_min_len(PAIRED_MATRICES_ROWS_PER_TASK)
        .for_each(|(((left_out, right_out), left_row), right_row)| {
            (*left_out, *right_out) = if use_i8mm {
                // SAFETY: dispatch checked FEAT_I8MM and all slices have the
                // validated Q5_0/residual block layout.
                unsafe { row_dot_pair_i8mm(left_row, right_row, &activation) }
            } else {
                row_dot_pair(left_row, right_row, &activation)
            };
        });
    Ok((left_values, right_values))
}

fn neon_q5_0_q8_residual_mul_vec_i8mm(
    bytes: &[u8],
    rows: usize,
    row_bytes: usize,
    activation: &[BlockQ8Residual],
) -> Vec<f32> {
    let paired_rows = rows / 2 * 2;
    let paired_bytes = paired_rows * row_bytes;
    let mut values = vec![0.0; rows];
    let (paired_values, tail_values) = values.split_at_mut(paired_rows);
    if paired_rows >= ROW_PARALLEL_MIN_ROWS {
        paired_values
            .par_chunks_exact_mut(2)
            .zip(bytes[..paired_bytes].par_chunks_exact(row_bytes * 2))
            .with_min_len(ROW_PARALLEL_MIN_ROWS_PER_TASK / 2)
            .for_each(|(output, rows)| {
                let (left, right) = rows.split_at(row_bytes);
                // SAFETY: dispatch checked FEAT_I8MM and the caller validated
                // exact row and activation block sizes.
                let pair = unsafe { row_dot_pair_i8mm(left, right, activation) };
                output.copy_from_slice(&[pair.0, pair.1]);
            });
    } else {
        for (output, rows) in paired_values
            .chunks_exact_mut(2)
            .zip(bytes[..paired_bytes].chunks_exact(row_bytes * 2))
        {
            let (left, right) = rows.split_at(row_bytes);
            // SAFETY: dispatch checked FEAT_I8MM and the caller validated
            // exact row and activation block sizes.
            let pair = unsafe { row_dot_pair_i8mm(left, right, activation) };
            output.copy_from_slice(&[pair.0, pair.1]);
        }
    }
    if let Some(output) = tail_values.first_mut() {
        *output = row_dot(&bytes[paired_bytes..], activation);
    }
    values
}

fn row_dot(row: &[u8], activation: &[BlockQ8Residual]) -> f32 {
    row.chunks_exact(Q5_0_BLOCK_BYTES)
        .zip(activation)
        // SAFETY: validated exact-sized blocks and FEAT_DotProd dispatch.
        .map(|(weights, activation)| unsafe { block_dot(weights, activation) })
        .sum()
}

fn row_dot_pair(left_row: &[u8], right_row: &[u8], activation: &[BlockQ8Residual]) -> (f32, f32) {
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    for ((left, right), activation) in left_row
        .chunks_exact(Q5_0_BLOCK_BYTES)
        .zip(right_row.chunks_exact(Q5_0_BLOCK_BYTES))
        .zip(activation)
    {
        // SAFETY: the caller validated both matrices and the activation was
        // quantized in exact 32-value blocks.
        unsafe {
            left_sum += block_dot(left, activation);
            right_sum += block_dot(right, activation);
        }
    }
    (left_sum, right_sum)
}

#[target_feature(enable = "i8mm")]
unsafe fn row_dot_pair_i8mm(
    left_row: &[u8],
    right_row: &[u8],
    activation: &[BlockQ8Residual],
) -> (f32, f32) {
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    for ((left, right), activation) in left_row
        .chunks_exact(Q5_0_BLOCK_BYTES)
        .zip(right_row.chunks_exact(Q5_0_BLOCK_BYTES))
        .zip(activation)
    {
        // SAFETY: exact block sizes were established by the caller.
        let pair = unsafe { block_dot_pair_i8mm(left, right, activation) };
        left_sum += pair.0;
        right_sum += pair.1;
    }
    (left_sum, right_sum)
}

#[target_feature(enable = "i8mm")]
unsafe fn block_dot_pair_i8mm(
    left: &[u8],
    right: &[u8],
    activation: &BlockQ8Residual,
) -> (f32, f32) {
    // SAFETY: callers pass exact 22-byte weight blocks and a complete
    // residual activation block.
    unsafe {
        let (left_low, left_high) = decode_q5_weights(left);
        let (right_low, right_high) = decode_q5_weights(right);
        let pass_0 = activation.quants[0].as_ptr();
        let pass_1 = activation.quants[1].as_ptr();
        let pass_0_low = vld1q_s8(pass_0);
        let pass_1_low = vld1q_s8(pass_1);
        let pass_0_high = vld1q_s8(pass_0.add(16));
        let pass_1_high = vld1q_s8(pass_1.add(16));
        let mut sums = vdupq_n_s32(0);

        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_low_s8(left_low), vget_low_s8(right_low)),
            vcombine_s8(vget_low_s8(pass_0_low), vget_low_s8(pass_1_low)),
        );
        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_high_s8(left_low), vget_high_s8(right_low)),
            vcombine_s8(vget_high_s8(pass_0_low), vget_high_s8(pass_1_low)),
        );
        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_low_s8(left_high), vget_low_s8(right_high)),
            vcombine_s8(vget_low_s8(pass_0_high), vget_low_s8(pass_1_high)),
        );
        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_high_s8(left_high), vget_high_s8(right_high)),
            vcombine_s8(vget_high_s8(pass_0_high), vget_high_s8(pass_1_high)),
        );

        let left_dot = vgetq_lane_s32::<0>(sums) as f32 * activation.scales[0]
            + vgetq_lane_s32::<1>(sums) as f32 * activation.scales[1];
        let right_dot = vgetq_lane_s32::<2>(sums) as f32 * activation.scales[0]
            + vgetq_lane_s32::<3>(sums) as f32 * activation.scales[1];
        let left_scale = native_f16_bits_to_f32(u16::from_le_bytes([left[0], left[1]]));
        let right_scale = native_f16_bits_to_f32(u16::from_le_bytes([right[0], right[1]]));
        (left_dot * left_scale, right_dot * right_scale)
    }
}

#[target_feature(enable = "neon")]
unsafe fn decode_q5_weights(weights: &[u8]) -> (int8x16_t, int8x16_t) {
    // SAFETY: callers pass a complete Q5_0 block.
    unsafe {
        let packed = vld1q_u8(weights.as_ptr().add(6));
        let low_nibbles = vandq_u8(packed, vdupq_n_u8(0x0f));
        let high_nibbles = vshrq_n_u8(packed, 4);
        let (low_offsets, high_offsets) = q5_signed_offsets(weights);
        (
            vaddq_s8(
                vreinterpretq_s8_u8(low_nibbles),
                vreinterpretq_s8_u8(low_offsets),
            ),
            vaddq_s8(
                vreinterpretq_s8_u8(high_nibbles),
                vreinterpretq_s8_u8(high_offsets),
            ),
        )
    }
}

#[target_feature(enable = "dotprod")]
unsafe fn block_dot(weights: &[u8], activation: &BlockQ8Residual) -> f32 {
    let weight_scale =
        unsafe { native_f16_bits_to_f32(u16::from_le_bytes([weights[0], weights[1]])) };
    unsafe {
        let packed = vld1q_u8(weights.as_ptr().add(6));
        let low_nibbles = vandq_u8(packed, vdupq_n_u8(0x0f));
        let high_nibbles = vshrq_n_u8(packed, 4);
        let (low_offsets, high_offsets) = q5_signed_offsets(weights);
        let low_weights = vaddq_s8(
            vreinterpretq_s8_u8(low_nibbles),
            vreinterpretq_s8_u8(low_offsets),
        );
        let high_weights = vaddq_s8(
            vreinterpretq_s8_u8(high_nibbles),
            vreinterpretq_s8_u8(high_offsets),
        );

        let mut activation_dot = 0.0f32;
        for pass in 0..Q8_RESIDUAL_PASSES {
            let low_activation = vld1q_s8(activation.quants[pass].as_ptr());
            let high_activation = vld1q_s8(activation.quants[pass].as_ptr().add(16));
            let zero = vdupq_n_s32(0);
            let dot = vaddq_s32(
                dot_i8x16(zero, low_weights, low_activation),
                dot_i8x16(zero, high_weights, high_activation),
            );
            activation_dot += vaddvq_s32(dot) as f32 * activation.scales[pass];
        }
        activation_dot * weight_scale
    }
}

#[inline(always)]
unsafe fn dot_i8x16(mut sum: int32x4_t, left: int8x16_t, right: int8x16_t) -> int32x4_t {
    unsafe {
        asm!(
            "sdot {sum:v}.4s, {left:v}.16b, {right:v}.16b",
            sum = inout(vreg) sum,
            left = in(vreg) left,
            right = in(vreg) right,
            options(nostack, nomem, pure),
        );
    }
    sum
}

#[inline(always)]
unsafe fn matrix_dot_i8x16(mut sum: int32x4_t, rows: int8x16_t, columns: int8x16_t) -> int32x4_t {
    unsafe {
        asm!(
            "smmla {sum:v}.4s, {rows:v}.16b, {columns:v}.16b",
            sum = inout(vreg) sum,
            rows = in(vreg) rows,
            columns = in(vreg) columns,
            options(nostack, nomem, pure),
        );
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::q5_0::q5_0_signed_values;

    #[test]
    fn block_dot_matches_residual_reconstruction() -> Result<(), InferenceError> {
        let mut weights = vec![0u8; Q5_0_BLOCK_BYTES];
        weights[..2].copy_from_slice(&0x3c00u16.to_le_bytes());
        weights[2..6].copy_from_slice(&0xa55a_3cc3u32.to_le_bytes());
        for (index, value) in weights[6..].iter_mut().enumerate() {
            *value = (index * 29 + 7) as u8;
        }
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| (index as f32 - 15.5) / 9.0)
            .collect::<Vec<_>>();
        let activation = BlockQ8Residual::quantize_blocks(&vector)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual Q8 block"))?;
        let signed = q5_0_signed_values(&weights);
        let expected = signed
            .iter()
            .enumerate()
            .map(|(index, weight)| {
                let reconstructed = (0..Q8_RESIDUAL_PASSES)
                    .map(|pass| activation.scales[pass] * f32::from(activation.quants[pass][index]))
                    .sum::<f32>();
                f32::from(*weight) * reconstructed
            })
            .sum::<f32>();

        let actual = unsafe { block_dot(&weights, &activation) };
        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    #[test]
    fn paired_matvec_matches_independent_residual_calls() -> Result<(), InferenceError> {
        let rows = 512;
        let cols = 64;
        let left = patterned_matrix(rows * cols / Q5_0_BLOCK_VALUES, 17);
        let right = patterned_matrix(rows * cols / Q5_0_BLOCK_VALUES, 93);
        let vector = (0..cols)
            .map(|index| ((index * 13 % 31) as f32 - 15.0) / 7.0)
            .collect::<Vec<_>>();

        let expected_left = neon_q5_0_q8_residual_mul_vec(&left, rows, cols, &vector)?;
        let expected_right = neon_q5_0_q8_residual_mul_vec(&right, rows, cols, &vector)?;
        let actual = neon_q5_0_q8_residual_mul_vec_pair(&left, &right, rows, cols, &vector)?;

        assert_eq!(actual, (expected_left, expected_right));
        Ok(())
    }

    #[test]
    fn i8mm_block_pair_matches_two_sdot_block_dots() -> Result<(), InferenceError> {
        if !std::arch::is_aarch64_feature_detected!("i8mm") {
            return Ok(());
        }
        let left = patterned_matrix(1, 17);
        let right = patterned_matrix(1, 93);
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| ((index * 13 % 31) as f32 - 15.0) / 7.0)
            .collect::<Vec<_>>();
        let activation = BlockQ8Residual::quantize_blocks(&vector)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual Q8 block"))?;

        let expected = unsafe {
            (
                block_dot(&left, &activation),
                block_dot(&right, &activation),
            )
        };
        let actual = unsafe { block_dot_pair_i8mm(&left, &right, &activation) };
        assert_eq!(actual, expected);
        Ok(())
    }

    fn patterned_matrix(blocks: usize, seed: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(blocks * Q5_0_BLOCK_BYTES);
        for block_index in 0..blocks {
            bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
            let high_bits = (block_index.wrapping_mul(0x9e37_79b9) ^ seed) as u32;
            bytes.extend_from_slice(&high_bits.to_le_bytes());
            bytes.extend((0..16).map(|index| {
                block_index
                    .wrapping_mul(29)
                    .wrapping_add(index * 11)
                    .wrapping_add(seed) as u8
            }));
        }
        bytes
    }
}
