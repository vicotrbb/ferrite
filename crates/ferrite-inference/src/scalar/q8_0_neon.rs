#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q8_0::{
        argmax_q8_0_rows, parallel_argmax_q8_0_rows, Q8_0MatVecBackend, Q8_0MatVecOutput,
        Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES,
    },
};
use rayon::prelude::*;
use std::arch::aarch64::{
    vaddvq_f32, vcvtq_f32_s32, vdupq_n_f32, vfmaq_f32, vget_high_s16, vget_low_s16, vld1_s8,
    vld1q_f32, vmovl_s16, vmovl_s8,
};

const ROW_PARALLEL_MIN_ROWS: usize = 4096;
const ROW_PARALLEL_MAX_COLS: usize = 2048;

pub(super) fn neon_q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Q8_0MatVecOutput {
    let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
    if uses_row_parallel(rows, cols) {
        return neon_q8_0_mul_vec_row_parallel(bytes, rows, row_bytes, vector);
    }

    let mut values = vec![0.0; rows];
    for (row, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
        values[row] = neon_q8_0_row_dot(row_chunk, vector);
    }

    Q8_0MatVecOutput {
        values,
        backend: Q8_0MatVecBackend::Aarch64Neon,
    }
}

fn neon_q8_0_mul_vec_row_parallel(
    bytes: &[u8],
    rows: usize,
    row_bytes: usize,
    vector: &[f32],
) -> Q8_0MatVecOutput {
    let values = bytes
        .par_chunks_exact(row_bytes)
        .map(|row_chunk| neon_q8_0_row_dot(row_chunk, vector))
        .collect::<Vec<_>>();
    debug_assert_eq!(values.len(), rows);

    Q8_0MatVecOutput {
        values,
        backend: Q8_0MatVecBackend::Aarch64NeonRowParallel,
    }
}

fn neon_q8_0_row_dot(row_chunk: &[u8], vector: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (block_index, block) in row_chunk.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let col_base = block_index * Q8_0_BLOCK_VALUES;
        // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
        // `cols` is validated as a multiple of 32, so every 8-byte
        // quant load and matching 4-lane vector load is in bounds.
        sum += unsafe {
            neon_q8_0_block_dot(
                block[2..].as_ptr().cast::<i8>(),
                vector[col_base..].as_ptr(),
            )
        } * scale;
    }
    sum
}

fn uses_row_parallel(rows: usize, cols: usize) -> bool {
    rows >= ROW_PARALLEL_MIN_ROWS && cols <= ROW_PARALLEL_MAX_COLS
}

pub(super) fn neon_q8_0_argmax_mul_vec(
    bytes: &[u8],
    _rows: usize,
    cols: usize,
    vector: &[f32],
) -> usize {
    let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
    argmax_q8_0_rows(bytes, row_bytes, |row_bytes| {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
            // `cols` is validated as a multiple of 32, so every 8-byte
            // quant load and matching 4-lane vector load is in bounds.
            sum += unsafe {
                neon_q8_0_block_dot(
                    block[2..].as_ptr().cast::<i8>(),
                    vector[col_base..].as_ptr(),
                )
            } * scale;
        }
        sum
    })
}

pub(super) fn neon_q8_0_parallel_argmax_mul_vec(
    bytes: &[u8],
    cols: usize,
    vector: &[f32],
) -> usize {
    let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
    parallel_argmax_q8_0_rows(bytes, row_bytes, |row_bytes| {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
            // `cols` is validated as a multiple of 32, so every 8-byte
            // quant load and matching 4-lane vector load is in bounds.
            sum += unsafe {
                neon_q8_0_block_dot(
                    block[2..].as_ptr().cast::<i8>(),
                    vector[col_base..].as_ptr(),
                )
            } * scale;
        }
        sum
    })
}

#[target_feature(enable = "neon")]
pub(super) unsafe fn neon_q8_0_block_dot(quantized: *const i8, vector: *const f32) -> f32 {
    let mut lanes = vdupq_n_f32(0.0);
    let mut offset = 0usize;
    while offset < Q8_0_BLOCK_VALUES {
        let quantized_i8 = unsafe { vld1_s8(quantized.add(offset)) };
        let quantized_i16 = vmovl_s8(quantized_i8);

        let low_i32 = vmovl_s16(vget_low_s16(quantized_i16));
        let low_f32 = vcvtq_f32_s32(low_i32);
        let low_vector = unsafe { vld1q_f32(vector.add(offset)) };
        lanes = vfmaq_f32(lanes, low_f32, low_vector);

        let high_i32 = vmovl_s16(vget_high_s16(quantized_i16));
        let high_f32 = vcvtq_f32_s32(high_i32);
        let high_vector = unsafe { vld1q_f32(vector.add(offset + 4)) };
        lanes = vfmaq_f32(lanes, high_f32, high_vector);

        offset += 8;
    }
    vaddvq_f32(lanes)
}

#[cfg(test)]
mod tests {
    use super::neon_q8_0_block_dot;
    use crate::scalar::{q8_0::Q8_0_BLOCK_VALUES, InferenceError};

    #[test]
    fn neon_q8_0_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
        let quantized = (0..Q8_0_BLOCK_VALUES)
            .map(|index| index as i8 - 16)
            .collect::<Vec<_>>();
        let vector = (0..Q8_0_BLOCK_VALUES)
            .map(|index| (index % 9) as f32 - 4.0)
            .collect::<Vec<_>>();

        let actual = unsafe { neon_q8_0_block_dot(quantized.as_ptr(), vector.as_ptr()) };
        let expected = quantized
            .iter()
            .zip(&vector)
            .map(|(left, right)| f32::from(*left) * right)
            .sum::<f32>();

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }
}
