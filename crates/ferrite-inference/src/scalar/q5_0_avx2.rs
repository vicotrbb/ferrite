#![allow(
    unsafe_code,
    reason = "audited x86_64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    float::f16_bits_to_f32,
    q5_0::{
        Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES, Q5_0MatVecBackend, Q5_0MatVecOutput,
        q5_0_signed_values,
    },
};
use std::arch::x86_64::{
    __m128i, _mm_loadl_epi64, _mm256_add_ps, _mm256_cvtepi8_epi32, _mm256_cvtepi32_ps,
    _mm256_loadu_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps,
};

pub(super) fn avx2_q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Q5_0MatVecOutput {
    let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
    let mut values = vec![0.0; rows];
    for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let signed = q5_0_signed_values(block);
            let col_base = block_index * Q5_0_BLOCK_VALUES;
            // SAFETY: `signed` contains exactly 32 decoded Q5_0 values and
            // `cols` is validated as a multiple of 32, so every 8-byte signed
            // load and matching 8-lane vector load is in bounds.
            sum += unsafe { avx2_i8_f32_block_dot(signed.as_ptr(), vector[col_base..].as_ptr()) }
                * scale;
        }
        values[row] = sum;
    }

    Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::X86_64Avx2,
    }
}

#[target_feature(enable = "avx2")]
unsafe fn avx2_i8_f32_block_dot(signed: *const i8, vector: *const f32) -> f32 {
    let mut lanes = _mm256_setzero_ps();
    let mut offset = 0usize;
    while offset < Q5_0_BLOCK_VALUES {
        // SAFETY: the caller provides 32 readable signed values and 32
        // readable vector values. `offset` advances in eight-value chunks,
        // so both loads remain within those ranges.
        let (signed_i8, vector_lanes) = unsafe {
            (
                _mm_loadl_epi64(signed.add(offset).cast::<__m128i>()),
                _mm256_loadu_ps(vector.add(offset)),
            )
        };
        let signed_i32 = _mm256_cvtepi8_epi32(signed_i8);
        let signed_f32 = _mm256_cvtepi32_ps(signed_i32);
        lanes = _mm256_add_ps(lanes, _mm256_mul_ps(signed_f32, vector_lanes));
        offset += 8;
    }

    let mut partial = [0.0f32; 8];
    // SAFETY: `partial` provides eight writable `f32` lanes, and this store
    // accepts an unaligned destination.
    unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
    partial.iter().sum()
}
