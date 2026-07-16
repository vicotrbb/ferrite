#![allow(
    unsafe_code,
    reason = "audited x86_64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    InferenceError,
    q6_k::{
        Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES, Q6KMatVecBackend, Q6KMatVecOutput, q6_k_block_values,
    },
};
use rayon::prelude::*;
use std::arch::x86_64::{
    _mm256_add_ps, _mm256_loadu_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps,
};

pub(super) fn avx2_q6_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q6KMatVecOutput, InferenceError> {
    let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
    let values = bytes
        .par_chunks_exact(row_bytes)
        .map(|row_chunk| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let block_values = q6_k_block_values(block)?;
                let col_base = block_index * Q6_K_BLOCK_VALUES;
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

    Ok(Q6KMatVecOutput {
        values,
        backend: Q6KMatVecBackend::X86_64Avx2,
    })
}

pub(super) fn avx2_q6_k_argmax_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
    let row_scores = bytes
        .par_chunks_exact(row_bytes)
        .enumerate()
        .map(|(row_index, row_chunk)| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let block_values = q6_k_block_values(block)?;
                let col_base = block_index * Q6_K_BLOCK_VALUES;
                // SAFETY: `block_values` contains exactly 256 contiguous f32
                // values and `cols` is a multiple of 256, so every 8-lane load
                // from the block and vector slice is in bounds.
                sum += unsafe {
                    avx2_f32_block_dot(block_values.as_ptr(), vector[col_base..].as_ptr())
                };
            }
            Ok((row_index, sum))
        })
        .collect::<Result<Vec<_>, InferenceError>>()?;
    debug_assert_eq!(row_scores.len(), rows);

    row_scores
        .into_iter()
        .reduce(|best, candidate| {
            if candidate.1 > best.1 {
                candidate
            } else {
                best
            }
        })
        .map(|(row_index, _)| row_index)
        .ok_or_else(|| InferenceError::new("argmax input must not be empty"))
}

#[target_feature(enable = "avx2")]
unsafe fn avx2_f32_block_dot(left: *const f32, right: *const f32) -> f32 {
    let mut lanes = _mm256_setzero_ps();
    let mut offset = 0usize;
    while offset < Q6_K_BLOCK_VALUES {
        // SAFETY: the caller provides 256 readable values at each pointer,
        // and `offset` advances in eight-value chunks within that range.
        let (left_lanes, right_lanes) = unsafe {
            (
                _mm256_loadu_ps(left.add(offset)),
                _mm256_loadu_ps(right.add(offset)),
            )
        };
        lanes = _mm256_add_ps(lanes, _mm256_mul_ps(left_lanes, right_lanes));
        offset += 8;
    }

    let mut partial = [0.0f32; 8];
    // SAFETY: `partial` provides eight writable `f32` lanes, and this store
    // accepts an unaligned destination.
    unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
    partial.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::avx2_q6_k_argmax_mul_vec;
    use crate::scalar::{
        InferenceError,
        q6_k::{Q6_K_BLOCK_VALUES, q6_k_mul_vec},
    };

    #[test]
    fn avx2_q6_k_argmax_mul_vec_matches_full_matvec_argmax() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(patterned_q6_k_block(0));
        bytes.extend(patterned_q6_k_block(17));
        bytes.extend(patterned_q6_k_block(31));
        let vector = (0..Q6_K_BLOCK_VALUES)
            .map(|index| (index % 19) as f32 / 7.0 - 1.25)
            .collect::<Vec<_>>();

        let values = q6_k_mul_vec(&bytes, 3, Q6_K_BLOCK_VALUES, &vector)?;
        let expected = values
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                left.total_cmp(right)
                    .then_with(|| right_index.cmp(left_index))
            })
            .map(|(index, _)| index)
            .ok_or_else(|| InferenceError::new("empty argmax"))?;

        assert_eq!(
            avx2_q6_k_argmax_mul_vec(&bytes, 3, Q6_K_BLOCK_VALUES, &vector)?,
            expected
        );
        Ok(())
    }

    fn patterned_q6_k_block(seed: u8) -> Vec<u8> {
        let mut block = Vec::new();
        block.extend((0..128).map(|index| (index * 37 + usize::from(seed)) as u8));
        block.extend((0..64).map(|index| (index * 19 + usize::from(seed)) as u8));
        block.extend(
            [-3i8, 2, -5, 4, -7, 6, -9, 8, 9, -8, 7, -6, 5, -4, 3, -2]
                .map(|value| value.wrapping_add(seed as i8) as u8),
        );
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }
}
