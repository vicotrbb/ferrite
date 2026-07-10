#![allow(
    unsafe_code,
    reason = "audited x86_64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    q4_k::{
        q4_k_block_values, Q4KMatVecBackend, Q4KMatVecOutput, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES,
    },
    InferenceError,
};
use rayon::prelude::*;
use std::arch::x86_64::{
    _mm256_add_ps, _mm256_loadu_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps,
};

pub(super) fn avx2_q4_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q4KMatVecOutput, InferenceError> {
    let blocks_per_row = cols / Q4_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q4_K_BLOCK_BYTES;
    let values = bytes
        .par_chunks_exact(row_bytes)
        .map(|row_chunk| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q4_K_BLOCK_BYTES).enumerate() {
                let block_values = q4_k_block_values(block)?;
                let col_base = block_index * Q4_K_BLOCK_VALUES;
                // SAFETY: `block_values` contains exactly 256 contiguous f32
                // values and `cols` is a multiple of 256, so every 8-lane load
                // from the block and vector slice is in bounds.
                sum += unsafe {
                    avx2_f32_block_dot(block_values.as_ptr(), vector[col_base..].as_ptr())
                };
            }
            Ok(sum)
        })
        .collect::<Result<Vec<_>, InferenceError>>()?;
    debug_assert_eq!(values.len(), rows);

    Ok(Q4KMatVecOutput {
        values,
        backend: Q4KMatVecBackend::X86_64Avx2,
    })
}

#[target_feature(enable = "avx2")]
unsafe fn avx2_f32_block_dot(left: *const f32, right: *const f32) -> f32 {
    let mut lanes = _mm256_setzero_ps();
    let mut offset = 0usize;
    while offset < Q4_K_BLOCK_VALUES {
        let left_lanes = unsafe { _mm256_loadu_ps(left.add(offset)) };
        let right_lanes = unsafe { _mm256_loadu_ps(right.add(offset)) };
        lanes = _mm256_add_ps(lanes, _mm256_mul_ps(left_lanes, right_lanes));
        offset += 8;
    }

    let mut partial = [0.0f32; 8];
    unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
    partial.iter().sum()
}
