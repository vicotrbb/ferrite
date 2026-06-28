#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q5_0::{Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
};
use rayon::prelude::*;
use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32};

const ROW_PARALLEL_MIN_ROWS: usize = 4096;
const ROW_PARALLEL_MAX_COLS: usize = 1024;

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

fn uses_row_parallel(rows: usize, cols: usize) -> bool {
    rows >= ROW_PARALLEL_MIN_ROWS && cols <= ROW_PARALLEL_MAX_COLS
}

#[target_feature(enable = "neon")]
pub(super) unsafe fn neon_q5_0_block_dot(block: &[u8], vector: &[f32]) -> f32 {
    let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let high_bits = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
    let quants = &block[6..];
    let mut lanes = vdupq_n_f32(0.0);

    for lane_base in (0..16).step_by(4) {
        let mut low_values = [0.0; 4];
        let mut high_values = [0.0; 4];
        for lane in 0..4 {
            let index = lane_base + lane;
            let quant = quants[index];
            let low_high_bit = ((high_bits >> index) << 4) as u8 & 0x10;
            let high_high_bit = (high_bits >> (index + 12)) as u8 & 0x10;
            low_values[lane] = f32::from(((quant & 0x0f) | low_high_bit) as i8 - 16);
            high_values[lane] = f32::from(((quant >> 4) | high_high_bit) as i8 - 16);
        }

        // SAFETY: the temporary lane arrays contain four contiguous f32
        // values, and the vector slice was validated to contain exactly one
        // Q5_0 block worth of elements.
        unsafe {
            let low_vector = vld1q_f32(vector.as_ptr().add(lane_base));
            lanes = vfmaq_f32(lanes, vld1q_f32(low_values.as_ptr()), low_vector);

            let high_vector = vld1q_f32(vector.as_ptr().add(lane_base + 16));
            lanes = vfmaq_f32(lanes, vld1q_f32(high_values.as_ptr()), high_vector);
        }
    }

    vaddvq_f32(lanes) * scale
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
