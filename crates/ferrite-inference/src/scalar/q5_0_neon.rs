#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q5_0::{Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int8x16_t, vaddvq_f32, vandq_u8, vcombine_u8, vcvtq_f32_s32, vdup_n_u8, vdupq_n_f32,
    vdupq_n_s8, vdupq_n_u8, vfmaq_f32, vget_high_s16, vget_high_s8, vget_low_s16, vget_low_s8,
    vld1q_f32, vld1q_u8, vmovl_s16, vmovl_s8, vorrq_u8, vreinterpretq_s8_u8, vshrq_n_u8, vsubq_s8,
    vtstq_u8,
};

const ROW_PARALLEL_MIN_ROWS: usize = 512;
/// Minimum rows per rayon task: keeps per-task work in the tens of
/// microseconds so fork-join overhead and straggler tails stay small
/// relative to the streamed weight bytes.
const ROW_PARALLEL_MIN_ROWS_PER_TASK: usize = 128;

pub(super) fn neon_q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Q5_0MatVecOutput {
    let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
    if uses_row_parallel(rows, cols) {
        return neon_q5_0_mul_vec_row_parallel(bytes, rows, row_bytes, vector);
    }

    let mut values = vec![0.0; rows];
    for (row, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
        values[row] = neon_q5_0_row_dot(row_chunk, vector);
    }

    Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::Aarch64Neon,
    }
}

fn neon_q5_0_mul_vec_row_parallel(
    bytes: &[u8],
    rows: usize,
    row_bytes: usize,
    vector: &[f32],
) -> Q5_0MatVecOutput {
    let values = bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(ROW_PARALLEL_MIN_ROWS_PER_TASK)
        .map(|row_chunk| neon_q5_0_row_dot(row_chunk, vector))
        .collect::<Vec<_>>();
    debug_assert_eq!(values.len(), rows);

    Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::Aarch64NeonRowParallel,
    }
}

fn neon_q5_0_row_dot(row_chunk: &[u8], vector: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (block_index, block) in row_chunk.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
        let col_base = block_index * Q5_0_BLOCK_VALUES;
        // SAFETY: the dispatch path checks NEON support, `block` has exactly
        // one Q5_0 block, and the matrix constructor validates that columns
        // are a multiple of 32 so the per-block vector slice is in bounds.
        sum +=
            unsafe { neon_q5_0_block_dot(block, &vector[col_base..col_base + Q5_0_BLOCK_VALUES]) };
    }
    sum
}

fn uses_row_parallel(rows: usize, _cols: usize) -> bool {
    rows >= ROW_PARALLEL_MIN_ROWS
}

/// Bit masks selecting the per-lane high bit: lanes 0-7 test bits 0-7 of one
/// duplicated `high_bits` byte, lanes 8-15 test bits 0-7 of the next byte.
const HIGH_BIT_LANE_MASK: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];

#[target_feature(enable = "neon")]
pub(super) unsafe fn neon_q5_0_block_dot(block: &[u8], vector: &[f32]) -> f32 {
    let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let high_bits = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
    let quants = &block[6..];

    // SAFETY: callers pass a length-checked 22-byte Q5_0 block (16 quant
    // bytes after the scale and high-bit words) and a 32-element vector
    // slice, so the 16-byte quant load, the mask-table load, and the eight
    // 4-lane vector loads below are all in bounds.
    unsafe {
        let bit_mask = vld1q_u8(HIGH_BIT_LANE_MASK.as_ptr());
        let quant_bytes = vld1q_u8(quants.as_ptr());

        // Values 0..16 come from the low nibbles plus high bits 0..16;
        // values 16..32 come from the high nibbles plus high bits 16..32.
        let low_nibbles = vandq_u8(quant_bytes, vdupq_n_u8(0x0f));
        let high_nibbles = vshrq_n_u8(quant_bytes, 4);

        let low_bit_bytes = vcombine_u8(
            vdup_n_u8(high_bits as u8),
            vdup_n_u8((high_bits >> 8) as u8),
        );
        let high_bit_bytes = vcombine_u8(
            vdup_n_u8((high_bits >> 16) as u8),
            vdup_n_u8((high_bits >> 24) as u8),
        );
        let low_high_bits = vandq_u8(vtstq_u8(low_bit_bytes, bit_mask), vdupq_n_u8(0x10));
        let high_high_bits = vandq_u8(vtstq_u8(high_bit_bytes, bit_mask), vdupq_n_u8(0x10));

        let offset = vdupq_n_s8(16);
        let low_signed = vsubq_s8(
            vreinterpretq_s8_u8(vorrq_u8(low_nibbles, low_high_bits)),
            offset,
        );
        let high_signed = vsubq_s8(
            vreinterpretq_s8_u8(vorrq_u8(high_nibbles, high_high_bits)),
            offset,
        );

        // Keep the exact FMA accumulation order of the previous kernel
        // (low group then high group per 4-lane step) so the result stays
        // bit-identical: integer-to-f32 conversion of values in -16..16 is
        // exact, so only the accumulation order could change the sum.
        let low_quads = widen_s8_lanes(low_signed);
        let high_quads = widen_s8_lanes(high_signed);
        let mut lanes = vdupq_n_f32(0.0);
        for step in 0..4 {
            let low_vector = vld1q_f32(vector.as_ptr().add(step * 4));
            lanes = vfmaq_f32(lanes, low_quads[step], low_vector);

            let high_vector = vld1q_f32(vector.as_ptr().add(step * 4 + 16));
            lanes = vfmaq_f32(lanes, high_quads[step], high_vector);
        }

        vaddvq_f32(lanes) * scale
    }
}

/// Widens 16 signed bytes into four 4-lane f32 vectors (exact conversion).
#[target_feature(enable = "neon")]
unsafe fn widen_s8_lanes(values: int8x16_t) -> [std::arch::aarch64::float32x4_t; 4] {
    let low_half = vmovl_s8(vget_low_s8(values));
    let high_half = vmovl_s8(vget_high_s8(values));
    [
        vcvtq_f32_s32(vmovl_s16(vget_low_s16(low_half))),
        vcvtq_f32_s32(vmovl_s16(vget_high_s16(low_half))),
        vcvtq_f32_s32(vmovl_s16(vget_low_s16(high_half))),
        vcvtq_f32_s32(vmovl_s16(vget_high_s16(high_half))),
    ]
}

#[cfg(test)]
mod tests {
    use super::neon_q5_0_block_dot;
    use crate::scalar::{
        q5_0::{decode_q5_0_row, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
        InferenceError,
    };

    #[test]
    fn neon_q5_0_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
        let block = patterned_q5_0_block();
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| (index % 9) as f32 - 4.0)
            .collect::<Vec<_>>();

        let actual = unsafe { neon_q5_0_block_dot(&block, &vector) };
        let expected = decode_q5_0_row(&block, Q5_0_BLOCK_VALUES)?
            .iter()
            .zip(&vector)
            .map(|(left, right)| left * right)
            .sum::<f32>();

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    #[test]
    fn neon_q5_0_block_dot_matches_decoded_values_across_high_bit_patterns(
    ) -> Result<(), InferenceError> {
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| (index % 7) as f32 - 3.0)
            .collect::<Vec<_>>();

        for high_bits in [
            0u32,
            u32::MAX,
            0xa5c3_3c5a,
            0x0f0f_f0f0,
            0x8000_0001,
            0x0001_8000,
        ] {
            let mut block = patterned_q5_0_block();
            block[2..6].copy_from_slice(&high_bits.to_le_bytes());

            let actual = unsafe { neon_q5_0_block_dot(&block, &vector) };
            let expected = decode_q5_0_row(&block, Q5_0_BLOCK_VALUES)?
                .iter()
                .zip(&vector)
                .map(|(left, right)| left * right)
                .sum::<f32>();

            assert!(
                (actual - expected).abs() < 0.001,
                "high_bits={high_bits:#010x} actual={actual} expected={expected}"
            );
        }
        Ok(())
    }

    fn patterned_q5_0_block() -> Vec<u8> {
        let mut block = Vec::with_capacity(Q5_0_BLOCK_BYTES);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0xa5c33c5au32.to_le_bytes());
        for index in 0..16 {
            let low = (index * 3) as u8 & 0x0f;
            let high = (15 - index) as u8 & 0x0f;
            block.push(low | (high << 4));
        }
        block
    }
}
