//! Experimental Q8_0 × two-pass residual-Q8 matvec for FEAT_I8MM CPUs.
#![allow(unsafe_code)]

use super::{
    neon_util::native_f16_bits_to_f32,
    q8_0::{q8_0_row_bytes, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES},
    q8_residual_activation::BlockQ8Residual,
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int32x4_t, int8x16_t, vcombine_s8, vdupq_n_s32, vget_high_s8, vget_low_s8, vgetq_lane_s32,
    vld1q_s8,
};
use std::arch::asm;

const ROWS_PER_TASK: usize = 128;

pub(super) fn neon_q8_0_q8_residual_i8mm_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    let (row_bytes, activation) = prepare(bytes, rows, cols, vector)?;
    let paired_rows = rows / 2 * 2;
    let paired_bytes = paired_rows * row_bytes;
    let mut values = vec![0.0; rows];
    let (paired_values, tail_values) = values.split_at_mut(paired_rows);
    paired_values
        .par_chunks_exact_mut(2)
        .zip(bytes[..paired_bytes].par_chunks_exact(row_bytes * 2))
        .with_min_len(ROWS_PER_TASK / 2)
        .for_each(|(output, rows)| {
            let (left, right) = rows.split_at(row_bytes);
            // SAFETY: dispatch checked FEAT_I8MM and `prepare` validated every
            // row and activation block.
            let pair = unsafe { row_dot_pair(left, right, &activation) };
            output.copy_from_slice(&[pair.0, pair.1]);
        });
    if let Some(output) = tail_values.first_mut() {
        *output =
            unsafe { row_dot_pair(&bytes[paired_bytes..], &bytes[paired_bytes..], &activation) }.0;
    }
    Ok(values)
}

pub(super) fn neon_q8_0_q8_residual_i8mm_argmax(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    if rows == 0 {
        return Err(InferenceError::new("argmax input must not be empty"));
    }
    let (row_bytes, activation) = prepare(bytes, rows, cols, vector)?;
    let paired_rows = rows / 2 * 2;
    let paired_bytes = paired_rows * row_bytes;
    let mut best = bytes[..paired_bytes]
        .par_chunks_exact(row_bytes * 2)
        .with_min_len(ROWS_PER_TASK / 2)
        .enumerate()
        .map(|(pair_index, rows)| {
            let (left, right) = rows.split_at(row_bytes);
            // SAFETY: same validated dispatch contract as the full matvec.
            let scores = unsafe { row_dot_pair(left, right, &activation) };
            let left_index = pair_index * 2;
            choose_best((left_index, scores.0), (left_index + 1, scores.1))
        })
        .reduce(|| (usize::MAX, f32::NEG_INFINITY), choose_best);
    if paired_rows != rows {
        let score =
            unsafe { row_dot_pair(&bytes[paired_bytes..], &bytes[paired_bytes..], &activation) }.0;
        best = choose_best(best, (paired_rows, score));
    }
    Ok(best.0)
}

fn prepare(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(usize, Vec<BlockQ8Residual>), InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q8_0_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid Q8_0 x residual-Q8 I8MM matvec shape",
        ));
    }
    let row_bytes = q8_0_row_bytes(cols)?;
    if bytes.len() != rows * row_bytes {
        return Err(InferenceError::new(
            "invalid Q8_0 x residual-Q8 I8MM matrix byte length",
        ));
    }
    Ok((row_bytes, BlockQ8Residual::quantize_blocks(vector)?))
}

fn choose_best(left: (usize, f32), right: (usize, f32)) -> (usize, f32) {
    if right.1 > left.1 || (right.1 == left.1 && right.0 < left.0) {
        right
    } else {
        left
    }
}

#[target_feature(enable = "i8mm")]
unsafe fn row_dot_pair(left: &[u8], right: &[u8], activation: &[BlockQ8Residual]) -> (f32, f32) {
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    for ((left, right), activation) in left
        .chunks_exact(Q8_0_BLOCK_BYTES)
        .zip(right.chunks_exact(Q8_0_BLOCK_BYTES))
        .zip(activation)
    {
        // SAFETY: callers validated exact Q8_0 block sizes.
        let pair = unsafe { block_dot_pair(left, right, activation) };
        left_sum += pair.0;
        right_sum += pair.1;
    }
    (left_sum, right_sum)
}

#[target_feature(enable = "i8mm")]
unsafe fn block_dot_pair(left: &[u8], right: &[u8], activation: &BlockQ8Residual) -> (f32, f32) {
    // SAFETY: each weight block has 32 signed quant bytes after its scale and
    // each residual pass has 32 activation quants.
    unsafe {
        let left_low = vld1q_s8(left.as_ptr().add(2).cast::<i8>());
        let left_high = vld1q_s8(left.as_ptr().add(18).cast::<i8>());
        let right_low = vld1q_s8(right.as_ptr().add(2).cast::<i8>());
        let right_high = vld1q_s8(right.as_ptr().add(18).cast::<i8>());
        let pass_0 = vld1q_s8(activation.quants[0].as_ptr());
        let pass_0_high = vld1q_s8(activation.quants[0].as_ptr().add(16));
        let pass_1 = vld1q_s8(activation.quants[1].as_ptr());
        let pass_1_high = vld1q_s8(activation.quants[1].as_ptr().add(16));
        let mut sums = vdupq_n_s32(0);

        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_low_s8(left_low), vget_low_s8(right_low)),
            vcombine_s8(vget_low_s8(pass_0), vget_low_s8(pass_1)),
        );
        sums = matrix_dot_i8x16(
            sums,
            vcombine_s8(vget_high_s8(left_low), vget_high_s8(right_low)),
            vcombine_s8(vget_high_s8(pass_0), vget_high_s8(pass_1)),
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

    #[test]
    fn i8mm_block_pair_matches_scalar_residual_dots() -> Result<(), InferenceError> {
        if !std::arch::is_aarch64_feature_detected!("i8mm") {
            return Ok(());
        }
        let left = patterned_block(17);
        let right = patterned_block(93);
        let vector = (0..Q8_0_BLOCK_VALUES)
            .map(|index| ((index * 13 % 31) as f32 - 15.0) / 7.0)
            .collect::<Vec<_>>();
        let activation = BlockQ8Residual::quantize_blocks(&vector)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual activation block"))?;
        let expected = (
            reference_dot(&left, &activation),
            reference_dot(&right, &activation),
        );
        let actual = unsafe { block_dot_pair(&left, &right, &activation) };

        assert_eq!(actual, expected);
        Ok(())
    }

    fn reference_dot(weights: &[u8], activation: &BlockQ8Residual) -> f32 {
        let weight_scale =
            unsafe { native_f16_bits_to_f32(u16::from_le_bytes([weights[0], weights[1]])) };
        let mut dot = 0.0;
        for pass in 0..2 {
            let integer_dot = weights[2..]
                .iter()
                .zip(activation.quants[pass])
                .map(|(weight, value)| i32::from(*weight as i8) * i32::from(value))
                .sum::<i32>();
            dot += integer_dot as f32 * activation.scales[pass];
        }
        dot * weight_scale
    }

    fn patterned_block(seed: usize) -> Vec<u8> {
        let mut block = Vec::with_capacity(Q8_0_BLOCK_BYTES);
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend((0..Q8_0_BLOCK_VALUES).map(|index| (index * 29 + seed) as u8));
        block
    }
}
