#![allow(
    unsafe_code,
    reason = "audited x86_64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    float::f16_bits_to_f32,
    q8_0::{
        argmax_q8_0_rows, Q8_0MatVecBackend, Q8_0MatVecOutput, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES,
    },
};
use std::arch::x86_64::{
    __m128i, _mm256_add_ps, _mm256_cvtepi32_ps, _mm256_cvtepi8_epi32, _mm256_loadu_ps,
    _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps, _mm_loadl_epi64,
};

pub(super) fn avx2_q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Q8_0MatVecOutput {
    let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
    let mut values = vec![0.0; rows];
    for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
            // `cols` is validated as a multiple of 32, so every 8-byte
            // quant load and matching 8-lane vector load is in bounds.
            sum += unsafe {
                avx2_q8_0_block_dot(
                    block[2..].as_ptr().cast::<i8>(),
                    vector[col_base..].as_ptr(),
                )
            } * scale;
        }
        values[row] = sum;
    }

    Q8_0MatVecOutput {
        values,
        backend: Q8_0MatVecBackend::X86_64Avx2,
    }
}

pub(super) fn avx2_q8_0_argmax_mul_vec(
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
            // quant load and matching 8-lane vector load is in bounds.
            sum += unsafe {
                avx2_q8_0_block_dot(
                    block[2..].as_ptr().cast::<i8>(),
                    vector[col_base..].as_ptr(),
                )
            } * scale;
        }
        sum
    })
}

#[target_feature(enable = "avx2")]
unsafe fn avx2_q8_0_block_dot(quantized: *const i8, vector: *const f32) -> f32 {
    let mut lanes = _mm256_setzero_ps();
    let mut offset = 0usize;
    while offset < Q8_0_BLOCK_VALUES {
        // SAFETY: the caller provides 32 readable quantized values and 32
        // readable vector values. `offset` advances in eight-value chunks,
        // so both loads remain within those ranges.
        let (quantized_i8, vector_lanes) = unsafe {
            (
                _mm_loadl_epi64(quantized.add(offset).cast::<__m128i>()),
                _mm256_loadu_ps(vector.add(offset)),
            )
        };
        let quantized_i32 = _mm256_cvtepi8_epi32(quantized_i8);
        let quantized_f32 = _mm256_cvtepi32_ps(quantized_i32);
        lanes = _mm256_add_ps(lanes, _mm256_mul_ps(quantized_f32, vector_lanes));
        offset += 8;
    }

    let mut partial = [0.0f32; 8];
    // SAFETY: `partial` provides eight writable `f32` lanes, and this store
    // accepts an unaligned destination.
    unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
    partial.iter().sum()
}
