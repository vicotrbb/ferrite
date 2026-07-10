//! Experimental `Q4_K` x two-pass residual-`Q8_K` matvec for `FEAT_I8MM` CPUs.
#![allow(
    unsafe_code,
    reason = "audited aarch64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    neon_util::native_f16_bits_to_f32,
    q4_k::{q4_k_storage_bytes, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    q8_residual_activation::BlockQ8KResidual,
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int32x4_t, int8x16_t, vandq_u8, vcombine_s8, vdupq_n_s32, vdupq_n_u8, vget_high_s8,
    vget_low_s8, vgetq_lane_s32, vld1q_s8, vld1q_u8, vreinterpretq_s8_u8, vshrq_n_u8,
};
use std::arch::asm;

const ROWS_PER_TASK: usize = 64;

pub(super) fn neon_q4_k_q8_residual_i8mm_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q4_K_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid Q4_K x residual-Q8_K I8MM matvec shape",
        ));
    }
    let expected = q4_k_storage_bytes(
        rows.checked_mul(cols)
            .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?,
    )?;
    if bytes.len() != expected {
        return Err(InferenceError::new(
            "invalid Q4_K x residual-Q8_K I8MM matrix byte length",
        ));
    }

    let activation = BlockQ8KResidual::quantize_blocks(vector)?;
    let row_bytes = cols / Q4_K_BLOCK_VALUES * Q4_K_BLOCK_BYTES;
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
            // SAFETY: dispatch checked FEAT_I8MM and all layouts were
            // validated above.
            let pair = unsafe { row_dot_pair(left, right, &activation) };
            output.copy_from_slice(&[pair.0, pair.1]);
        });
    if let Some(output) = tail_values.first_mut() {
        // SAFETY: dispatch checked FEAT_I8MM, and the tail is one validated
        // row paired with itself so the paired kernel can reuse its layout.
        *output =
            unsafe { row_dot_pair(&bytes[paired_bytes..], &bytes[paired_bytes..], &activation) }.0;
    }
    Ok(values)
}

#[target_feature(enable = "i8mm")]
unsafe fn row_dot_pair(left: &[u8], right: &[u8], activation: &[BlockQ8KResidual]) -> (f32, f32) {
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    for ((left, right), activation) in left
        .chunks_exact(Q4_K_BLOCK_BYTES)
        .zip(right.chunks_exact(Q4_K_BLOCK_BYTES))
        .zip(activation)
    {
        // SAFETY: row slices contain exact Q4_K blocks.
        let pair = unsafe { block_dot_pair(left, right, activation) };
        left_sum += pair.0;
        right_sum += pair.1;
    }
    (left_sum, right_sum)
}

#[target_feature(enable = "i8mm")]
unsafe fn block_dot_pair(left: &[u8], right: &[u8], activation: &BlockQ8KResidual) -> (f32, f32) {
    let left_scales = &left[4..16];
    let right_scales = &right[4..16];
    let left_quants = &left[16..];
    let right_quants = &right[16..];
    let mut weighted = [0i32; 4];
    let mut minimum = [0i32; 4];

    // SAFETY: both blocks contain four exact 32-byte packed-quant chunks and
    // both residual passes own 256 activation quants plus 16 group sums.
    unsafe {
        let mask = vdupq_n_u8(0x0f);
        for chunk in 0..4 {
            let left_packed_0 = vld1q_u8(left_quants.as_ptr().add(chunk * 32));
            let left_packed_1 = vld1q_u8(left_quants.as_ptr().add(chunk * 32 + 16));
            let right_packed_0 = vld1q_u8(right_quants.as_ptr().add(chunk * 32));
            let right_packed_1 = vld1q_u8(right_quants.as_ptr().add(chunk * 32 + 16));
            let left_groups = [
                (
                    vreinterpretq_s8_u8(vandq_u8(left_packed_0, mask)),
                    vreinterpretq_s8_u8(vandq_u8(left_packed_1, mask)),
                ),
                (
                    vreinterpretq_s8_u8(vshrq_n_u8(left_packed_0, 4)),
                    vreinterpretq_s8_u8(vshrq_n_u8(left_packed_1, 4)),
                ),
            ];
            let right_groups = [
                (
                    vreinterpretq_s8_u8(vandq_u8(right_packed_0, mask)),
                    vreinterpretq_s8_u8(vandq_u8(right_packed_1, mask)),
                ),
                (
                    vreinterpretq_s8_u8(vshrq_n_u8(right_packed_0, 4)),
                    vreinterpretq_s8_u8(vshrq_n_u8(right_packed_1, 4)),
                ),
            ];

            for half in 0..2 {
                let group = chunk * 2 + half;
                let activation_offset = group * 32;
                let pass_0 = vld1q_s8(activation.passes[0].qs.as_ptr().add(activation_offset));
                let pass_0_high =
                    vld1q_s8(activation.passes[0].qs.as_ptr().add(activation_offset + 16));
                let pass_1 = vld1q_s8(activation.passes[1].qs.as_ptr().add(activation_offset));
                let pass_1_high =
                    vld1q_s8(activation.passes[1].qs.as_ptr().add(activation_offset + 16));
                let mut sums = vdupq_n_s32(0);
                sums = matrix_dot_i8x16(
                    sums,
                    vcombine_s8(
                        vget_low_s8(left_groups[half].0),
                        vget_low_s8(right_groups[half].0),
                    ),
                    vcombine_s8(vget_low_s8(pass_0), vget_low_s8(pass_1)),
                );
                sums = matrix_dot_i8x16(
                    sums,
                    vcombine_s8(
                        vget_high_s8(left_groups[half].0),
                        vget_high_s8(right_groups[half].0),
                    ),
                    vcombine_s8(vget_high_s8(pass_0), vget_high_s8(pass_1)),
                );
                sums = matrix_dot_i8x16(
                    sums,
                    vcombine_s8(
                        vget_low_s8(left_groups[half].1),
                        vget_low_s8(right_groups[half].1),
                    ),
                    vcombine_s8(vget_low_s8(pass_0_high), vget_low_s8(pass_1_high)),
                );
                sums = matrix_dot_i8x16(
                    sums,
                    vcombine_s8(
                        vget_high_s8(left_groups[half].1),
                        vget_high_s8(right_groups[half].1),
                    ),
                    vcombine_s8(vget_high_s8(pass_0_high), vget_high_s8(pass_1_high)),
                );

                let (left_scale, left_min) = scale_min(group, left_scales);
                let (right_scale, right_min) = scale_min(group, right_scales);
                weighted[0] += i32::from(left_scale) * vgetq_lane_s32::<0>(sums);
                weighted[1] += i32::from(left_scale) * vgetq_lane_s32::<1>(sums);
                weighted[2] += i32::from(right_scale) * vgetq_lane_s32::<2>(sums);
                weighted[3] += i32::from(right_scale) * vgetq_lane_s32::<3>(sums);
                let activation_sum_0 = i32::from(activation.passes[0].bsums[group * 2])
                    + i32::from(activation.passes[0].bsums[group * 2 + 1]);
                let activation_sum_1 = i32::from(activation.passes[1].bsums[group * 2])
                    + i32::from(activation.passes[1].bsums[group * 2 + 1]);
                minimum[0] += i32::from(left_min) * activation_sum_0;
                minimum[1] += i32::from(left_min) * activation_sum_1;
                minimum[2] += i32::from(right_min) * activation_sum_0;
                minimum[3] += i32::from(right_min) * activation_sum_1;
            }
        }
    }

    // SAFETY: I8MM implies NEON support, and both inputs are complete Q4_K
    // blocks whose first four bytes contain the two half-precision scales.
    let (left_d, left_dmin, right_d, right_dmin) = unsafe {
        (
            native_f16_bits_to_f32(u16::from_le_bytes([left[0], left[1]])),
            native_f16_bits_to_f32(u16::from_le_bytes([left[2], left[3]])),
            native_f16_bits_to_f32(u16::from_le_bytes([right[0], right[1]])),
            native_f16_bits_to_f32(u16::from_le_bytes([right[2], right[3]])),
        )
    };
    let left_result = activation.passes[0].d
        * (left_d * weighted[0] as f32 - left_dmin * minimum[0] as f32)
        + activation.passes[1].d * (left_d * weighted[1] as f32 - left_dmin * minimum[1] as f32);
    let right_result = activation.passes[0].d
        * (right_d * weighted[2] as f32 - right_dmin * minimum[2] as f32)
        + activation.passes[1].d * (right_d * weighted[3] as f32 - right_dmin * minimum[3] as f32);
    (left_result, right_result)
}

fn scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
}

#[inline(always)]
unsafe fn matrix_dot_i8x16(mut sum: int32x4_t, rows: int8x16_t, columns: int8x16_t) -> int32x4_t {
    // SAFETY: callers enter only after FEAT_I8MM detection, and the assembly
    // touches registers only, with no memory or stack effects.
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
    use crate::scalar::q4_k_q8_k_neon::neon_q4_k_q8_k_block_dot;

    #[test]
    fn i8mm_pair_matches_two_residual_q8_k_reference_dots() -> Result<(), InferenceError> {
        if !std::arch::is_aarch64_feature_detected!("i8mm") {
            return Ok(());
        }
        let left = patterned_block(17);
        let right = patterned_block(93);
        let values = (0..Q4_K_BLOCK_VALUES)
            .map(|index| ((index * 37 % 101) as f32 - 50.0) / 13.0)
            .collect::<Vec<_>>();
        let residual = BlockQ8KResidual::quantize_blocks(&values)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual activation block"))?;
        let expected_left = residual
            .passes
            .iter()
            .map(|pass| neon_q4_k_q8_k_block_dot(&left, pass))
            .sum::<Result<f32, _>>()?;
        let expected_right = residual
            .passes
            .iter()
            .map(|pass| neon_q4_k_q8_k_block_dot(&right, pass))
            .sum::<Result<f32, _>>()?;
        // SAFETY: the runtime check above establishes FEAT_I8MM, and the test
        // inputs are complete Q4_K and residual activation blocks.
        let actual = unsafe { block_dot_pair(&left, &right, &residual) };

        assert!((actual.0 - expected_left).abs() < 0.001);
        assert!((actual.1 - expected_right).abs() < 0.001);
        Ok(())
    }

    fn patterned_block(seed: usize) -> Vec<u8> {
        let mut block = Vec::with_capacity(Q4_K_BLOCK_BYTES);
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend_from_slice(&0x3400u16.to_le_bytes());
        block.extend((0..12).map(|index| (index * 31 + seed) as u8));
        block.extend((0..128).map(|index| (index * 29 + seed * 3) as u8));
        block
    }
}
