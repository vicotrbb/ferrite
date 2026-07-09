#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    neon_util::widen_s8_lanes,
    q6_k::{Q6KMatVecBackend, Q6KMatVecOutput, Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES},
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    vaddvq_f32, vandq_u8, vdupq_n_f32, vdupq_n_s8, vdupq_n_u8, vfmaq_f32, vld1q_f32, vld1q_u8,
    vmulq_f32, vorrq_u8, vreinterpretq_s8_u8, vshlq_n_u8, vshrq_n_u8, vsubq_s8,
};

pub(super) fn neon_q6_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q6KMatVecOutput, InferenceError> {
    let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
    let values = bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(64)
        .map(|row_chunk| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q6_K_BLOCK_VALUES;
                // SAFETY: the dispatch path checks NEON support, `block` has
                // exactly one Q6_K block, and `cols` is a multiple of 256 so
                // the per-block vector slice is in bounds.
                sum += unsafe {
                    neon_q6_k_block_dot(block, &vector[col_base..col_base + Q6_K_BLOCK_VALUES])?
                };
            }
            Ok(sum)
        })
        .collect::<Result<Vec<_>, InferenceError>>()?;
    debug_assert_eq!(values.len(), rows);

    Ok(Q6KMatVecOutput {
        values,
        backend: Q6KMatVecBackend::Aarch64Neon,
    })
}

/// Batched matvec: streams each Q6_K weight row once per step for the
/// whole batch (rows stay cache-hot across streams). Per-stream block/FMA
/// order matches `neon_q6_k_mul_vec` exactly.
pub(super) fn neon_q6_k_mul_vec_batch(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
) -> Result<Vec<Vec<f32>>, InferenceError> {
    let batch = vectors.len();
    let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
    let mut flat = vec![0.0f32; rows * batch];
    bytes
        .par_chunks_exact(row_bytes)
        .zip(flat.par_chunks_exact_mut(batch))
        .with_min_len(64)
        .try_for_each(|(row_chunk, row_out)| {
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q6_K_BLOCK_VALUES;
                for (out, vector) in row_out.iter_mut().zip(vectors.iter()) {
                    // SAFETY: the dispatch path checks NEON support, `block`
                    // has exactly one Q6_K block, and every vector was
                    // validated to `cols` (a multiple of 256).
                    *out += unsafe {
                        neon_q6_k_block_dot(block, &vector[col_base..col_base + Q6_K_BLOCK_VALUES])?
                    };
                }
            }
            Ok::<(), InferenceError>(())
        })?;

    Ok((0..batch)
        .map(|stream| (0..rows).map(|row| flat[row * batch + stream]).collect())
        .collect())
}

pub(super) fn neon_q6_k_argmax_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
    let row_scores = bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(64)
        .enumerate()
        .map(|(row_index, row_chunk)| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q6_K_BLOCK_VALUES;
                // SAFETY: the dispatch path checks NEON support, `block` has
                // exactly one Q6_K block, and `cols` is a multiple of 256 so
                // the per-block vector slice is in bounds.
                sum += unsafe {
                    neon_q6_k_block_dot(block, &vector[col_base..col_base + Q6_K_BLOCK_VALUES])?
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

#[target_feature(enable = "neon")]
pub(super) unsafe fn neon_q6_k_block_dot(
    block: &[u8],
    vector: &[f32],
) -> Result<f32, InferenceError> {
    if block.len() != Q6_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q6_K block byte length {} does not match {Q6_K_BLOCK_BYTES}",
            block.len()
        )));
    }
    if vector.len() != Q6_K_BLOCK_VALUES {
        return Err(InferenceError::new(format!(
            "Q6_K block vector length {} does not match {Q6_K_BLOCK_VALUES}",
            vector.len()
        )));
    }

    let low_bits = &block[0..128];
    let high_bits = &block[128..192];
    let scales = &block[192..208];
    let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));
    let mut lanes = vdupq_n_f32(0.0);

    // SAFETY: `block` was length-checked above, so every 16-byte load from
    // `low_bits`/`high_bits` and every 4-lane load from the 256-element
    // `vector` slice below stays in bounds.
    unsafe {
        let nibble_mask = vdupq_n_u8(0x0f);
        let two_bit_mask = vdupq_n_u8(0x03);
        let offset = vdupq_n_s8(32);

        for half in 0..2 {
            let value_base = half * 128;
            let low_base = half * 64;
            let high_base = half * 32;
            let scale_base = half * 8;

            // Each 16-value window shares one scale per quarter; decode the
            // four 16-value groups of the window entirely in registers, then
            // replay the previous kernel's exact FMA sequence (q1..q4 per
            // 4-lane step) so the accumulated sum stays bit-identical.
            for window in 0..2 {
                let window_base = window * 16;
                let low_1 = vld1q_u8(low_bits.as_ptr().add(low_base + window_base));
                let low_2 = vld1q_u8(low_bits.as_ptr().add(low_base + window_base + 32));
                let high = vld1q_u8(high_bits.as_ptr().add(high_base + window_base));

                let group_1 = vorrq_u8(
                    vandq_u8(low_1, nibble_mask),
                    vshlq_n_u8(vandq_u8(high, two_bit_mask), 4),
                );
                let group_2 = vorrq_u8(
                    vandq_u8(low_2, nibble_mask),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 2), two_bit_mask), 4),
                );
                let group_3 = vorrq_u8(
                    vshrq_n_u8(low_1, 4),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 4), two_bit_mask), 4),
                );
                let group_4 = vorrq_u8(
                    vshrq_n_u8(low_2, 4),
                    vshlq_n_u8(vandq_u8(vshrq_n_u8(high, 6), two_bit_mask), 4),
                );

                let quads = [
                    widen_s8_lanes(vsubq_s8(vreinterpretq_s8_u8(group_1), offset)),
                    widen_s8_lanes(vsubq_s8(vreinterpretq_s8_u8(group_2), offset)),
                    widen_s8_lanes(vsubq_s8(vreinterpretq_s8_u8(group_3), offset)),
                    widen_s8_lanes(vsubq_s8(vreinterpretq_s8_u8(group_4), offset)),
                ];
                let group_scales = [
                    vdupq_n_f32(super_scale * f32::from(scales[scale_base + window] as i8)),
                    vdupq_n_f32(super_scale * f32::from(scales[scale_base + window + 2] as i8)),
                    vdupq_n_f32(super_scale * f32::from(scales[scale_base + window + 4] as i8)),
                    vdupq_n_f32(super_scale * f32::from(scales[scale_base + window + 6] as i8)),
                ];

                for step in 0..4 {
                    let index = window_base + step * 4;
                    for (group, (group_quads, group_scale)) in
                        quads.iter().zip(group_scales.iter()).enumerate()
                    {
                        let vector_values =
                            vld1q_f32(vector.as_ptr().add(value_base + index + group * 32));
                        lanes = vfmaq_f32(
                            lanes,
                            vmulq_f32(group_quads[step], *group_scale),
                            vector_values,
                        );
                    }
                }
            }
        }
    }

    Ok(vaddvq_f32(lanes))
}

#[cfg(test)]
mod tests {
    use super::neon_q6_k_block_dot;
    use crate::scalar::{
        q6_k::{decode_q6_k_values, Q6_K_BLOCK_VALUES},
        InferenceError,
    };

    #[test]
    fn neon_q6_k_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
        let block = patterned_q6_k_block();
        let vector = (0..Q6_K_BLOCK_VALUES)
            .map(|index| (index % 13) as f32 - 6.0)
            .collect::<Vec<_>>();

        let actual = unsafe { neon_q6_k_block_dot(&block, &vector)? };
        let expected = decode_q6_k_values(&block, Q6_K_BLOCK_VALUES)?
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

    fn patterned_q6_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend((0..128).map(|index| (index * 37) as u8));
        block.extend((0..64).map(|index| (index * 19) as u8));
        block.extend(
            [-3i8, 2, -5, 4, -7, 6, -9, 8, 9, -8, 7, -6, 5, -4, 3, -2].map(|value| value as u8),
        );
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }
}
