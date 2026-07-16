//! Experimental `Q6_K` x two-pass residual-`Q8_K` matvec for `FEAT_I8MM` CPUs.
#![allow(
    unsafe_code,
    reason = "audited aarch64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    InferenceError,
    neon_util::native_f16_bits_to_f32,
    q6_k::{Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES, q6_k_storage_bytes},
    q8_residual_activation::BlockQ8KResidual,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int8x16_t, int32x4_t, vandq_u8, vcombine_s8, vdupq_n_s8, vdupq_n_s32, vdupq_n_u8, vget_high_s8,
    vget_low_s8, vgetq_lane_s32, vld1_s8, vld1q_u8, vorrq_u8, vreinterpretq_s8_u8, vshlq_n_u8,
    vshrq_n_u8, vsubq_s8,
};
use std::arch::asm;

const ROWS_PER_TASK: usize = 64;

pub(super) fn neon_q6_k_q8_residual_i8mm_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q6_K_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid Q6_K x residual-Q8_K I8MM matvec shape",
        ));
    }
    let expected = q6_k_storage_bytes(
        rows.checked_mul(cols)
            .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?,
    )?;
    if bytes.len() != expected {
        return Err(InferenceError::new(
            "invalid Q6_K x residual-Q8_K I8MM matrix byte length",
        ));
    }

    let activation = BlockQ8KResidual::quantize_blocks(vector)?;
    let row_bytes = cols / Q6_K_BLOCK_VALUES * Q6_K_BLOCK_BYTES;
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
            // SAFETY: dispatch checked FEAT_I8MM and all block layouts were
            // validated above.
            let pair = unsafe { row_dot_pair(left, right, &activation) };
            output.copy_from_slice(&[pair.0, pair.1]);
        });
    if let Some(output) = tail_values.first_mut() {
        // Reusing the row as both operands preserves the left result and only
        // affects the uncommon odd-row tail.
        // SAFETY: dispatch checked FEAT_I8MM and the tail contains one
        // complete validated row, which is deliberately paired with itself.
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
        .chunks_exact(Q6_K_BLOCK_BYTES)
        .zip(right.chunks_exact(Q6_K_BLOCK_BYTES))
        .zip(activation)
    {
        // SAFETY: the row slices contain exact Q6_K blocks.
        let pair = unsafe { block_dot_pair(left, right, activation) };
        left_sum += pair.0;
        right_sum += pair.1;
    }
    (left_sum, right_sum)
}

#[target_feature(enable = "i8mm")]
unsafe fn block_dot_pair(left: &[u8], right: &[u8], activation: &BlockQ8KResidual) -> (f32, f32) {
    let left_low = &left[..128];
    let left_high = &left[128..192];
    let left_scales = &left[192..208];
    let right_low = &right[..128];
    let right_high = &right[128..192];
    let right_scales = &right[192..208];
    let mut weighted = [0i32; 4];

    // SAFETY: both blocks are exactly 210 bytes and each residual pass owns
    // 256 activation quants.
    unsafe {
        let nibble_mask = vdupq_n_u8(0x0f);
        let two_bit_mask = vdupq_n_u8(0x03);
        let offset = vdupq_n_s8(32);

        for half in 0..2 {
            let value_base = half * 128;
            let low_base = half * 64;
            let high_base = half * 32;
            let scale_base = half * 8;
            for window in 0..2 {
                let window_base = window * 16;
                let left_groups = decode_groups(
                    left_low,
                    left_high,
                    low_base,
                    high_base,
                    window_base,
                    nibble_mask,
                    two_bit_mask,
                    offset,
                );
                let right_groups = decode_groups(
                    right_low,
                    right_high,
                    low_base,
                    high_base,
                    window_base,
                    nibble_mask,
                    two_bit_mask,
                    offset,
                );
                let scale_indices = [
                    scale_base + window,
                    scale_base + window + 2,
                    scale_base + window + 4,
                    scale_base + window + 6,
                ];
                for group in 0..4 {
                    let activation_offset = value_base + window_base + group * 32;
                    let pass_0 = activation.passes[0].qs.as_ptr().add(activation_offset);
                    let pass_1 = activation.passes[1].qs.as_ptr().add(activation_offset);
                    let mut sums = vdupq_n_s32(0);
                    sums = matrix_dot_i8x16(
                        sums,
                        vcombine_s8(
                            vget_low_s8(left_groups[group]),
                            vget_low_s8(right_groups[group]),
                        ),
                        vcombine_s8(vld1_s8(pass_0), vld1_s8(pass_1)),
                    );
                    sums = matrix_dot_i8x16(
                        sums,
                        vcombine_s8(
                            vget_high_s8(left_groups[group]),
                            vget_high_s8(right_groups[group]),
                        ),
                        vcombine_s8(vld1_s8(pass_0.add(8)), vld1_s8(pass_1.add(8))),
                    );
                    let left_scale = i32::from(left_scales[scale_indices[group]] as i8);
                    let right_scale = i32::from(right_scales[scale_indices[group]] as i8);
                    weighted[0] += left_scale * vgetq_lane_s32::<0>(sums);
                    weighted[1] += left_scale * vgetq_lane_s32::<1>(sums);
                    weighted[2] += right_scale * vgetq_lane_s32::<2>(sums);
                    weighted[3] += right_scale * vgetq_lane_s32::<3>(sums);
                }
            }
        }
    }

    // SAFETY: I8MM implies NEON support, and both complete Q6_K blocks store
    // their half-precision super-scales in bytes 208 and 209.
    let (left_super, right_super) = unsafe {
        (
            native_f16_bits_to_f32(u16::from_le_bytes([left[208], left[209]])),
            native_f16_bits_to_f32(u16::from_le_bytes([right[208], right[209]])),
        )
    };
    let left_result = activation.passes[0].d * left_super * weighted[0] as f32
        + activation.passes[1].d * left_super * weighted[1] as f32;
    let right_result = activation.passes[0].d * right_super * weighted[2] as f32
        + activation.passes[1].d * right_super * weighted[3] as f32;
    (left_result, right_result)
}

#[target_feature(enable = "neon")]
#[allow(
    clippy::too_many_arguments,
    reason = "kernel arguments correspond directly to four packed Q6 lane groups"
)]
unsafe fn decode_groups(
    low: &[u8],
    high: &[u8],
    low_base: usize,
    high_base: usize,
    window_base: usize,
    nibble_mask: std::arch::aarch64::uint8x16_t,
    two_bit_mask: std::arch::aarch64::uint8x16_t,
    offset: std::arch::aarch64::int8x16_t,
) -> [int8x16_t; 4] {
    // SAFETY: callers pass complete Q6_K low and high bit planes, and all
    // offsets select one in-bounds 16-byte window while NEON is enabled.
    unsafe {
        let low_1 = vld1q_u8(low.as_ptr().add(low_base + window_base));
        let low_2 = vld1q_u8(low.as_ptr().add(low_base + window_base + 32));
        let high = vld1q_u8(high.as_ptr().add(high_base + window_base));
        [
            vsubq_s8(
                vreinterpretq_s8_u8(vorrq_u8(
                    vandq_u8(low_1, nibble_mask),
                    vshlq_n_u8(vandq_u8(high, two_bit_mask), 4),
                )),
                offset,
            ),
            vsubq_s8(
                vreinterpretq_s8_u8(vorrq_u8(
                    vandq_u8(low_2, nibble_mask),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 2), two_bit_mask), 4),
                )),
                offset,
            ),
            vsubq_s8(
                vreinterpretq_s8_u8(vorrq_u8(
                    vshrq_n_u8(low_1, 4),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 4), two_bit_mask), 4),
                )),
                offset,
            ),
            vsubq_s8(
                vreinterpretq_s8_u8(vorrq_u8(
                    vshrq_n_u8(low_2, 4),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 6), two_bit_mask), 4),
                )),
                offset,
            ),
        ]
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
    use crate::scalar::{q6_k_q8_k_neon::neon_q6_k_q8_k_block_dot, q8_k::BlockQ8K};

    #[test]
    fn i8mm_pair_matches_two_residual_q8_k_reference_dots() -> Result<(), InferenceError> {
        if !crate::scalar::CpuKernelCapabilities::detect().i8mm() {
            return Ok(());
        }
        let left = patterned_block(17);
        let right = patterned_block(93);
        let values = (0..Q6_K_BLOCK_VALUES)
            .map(|index| ((index * 37 % 101) as f32 - 50.0) / 13.0)
            .collect::<Vec<_>>();
        let residual = BlockQ8KResidual::quantize_blocks(&values)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual activation block"))?;
        let expected_left = reference_dot(&left, &residual)?;
        let expected_right = reference_dot(&right, &residual)?;
        // SAFETY: the runtime check above establishes FEAT_I8MM, and the test
        // inputs are complete Q6_K and residual activation blocks.
        let actual = unsafe { block_dot_pair(&left, &right, &residual) };

        assert!((actual.0 - expected_left).abs() < 0.001);
        assert!((actual.1 - expected_right).abs() < 0.001);
        Ok(())
    }

    fn reference_dot(weights: &[u8], residual: &BlockQ8KResidual) -> Result<f32, InferenceError> {
        residual
            .passes
            .iter()
            .map(|pass: &BlockQ8K| neon_q6_k_q8_k_block_dot(weights, pass))
            .sum()
    }

    fn patterned_block(seed: usize) -> Vec<u8> {
        let mut block = Vec::with_capacity(Q6_K_BLOCK_BYTES);
        block.extend((0..128).map(|index| (index * 37 + seed) as u8));
        block.extend((0..64).map(|index| (index * 19 + seed * 3) as u8));
        block.extend((0..16).map(|index| (index * 11 + seed) as u8));
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block
    }
}
