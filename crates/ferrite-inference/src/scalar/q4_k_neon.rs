#![allow(
    unsafe_code,
    reason = "audited aarch64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    neon_util::{native_f16_bits_to_f32, widen_s8_lanes},
    q4_k::{Q4KMatVecBackend, Q4KMatVecOutput, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    vaddvq_f32, vandq_u8, vdupq_n_f32, vdupq_n_u8, vfmaq_f32, vld1q_f32, vld1q_u8, vmulq_f32,
    vreinterpretq_s8_u8, vshrq_n_u8, vsubq_f32,
};

pub(super) fn neon_q4_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q4KMatVecOutput, InferenceError> {
    let blocks_per_row = cols / Q4_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q4_K_BLOCK_BYTES;
    let values = bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(64)
        .map(|row_chunk| {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q4_K_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q4_K_BLOCK_VALUES;
                // SAFETY: the dispatch path checks NEON support, `block` has
                // exactly one Q4_K block, and `cols` is a multiple of 256 so
                // the per-block vector slice is in bounds.
                sum += unsafe {
                    neon_q4_k_block_dot(block, &vector[col_base..col_base + Q4_K_BLOCK_VALUES])?
                };
            }
            Ok(sum)
        })
        .collect::<Result<Vec<_>, InferenceError>>()?;
    debug_assert_eq!(values.len(), rows);

    Ok(Q4KMatVecOutput {
        values,
        backend: Q4KMatVecBackend::Aarch64Neon,
    })
}

/// Batched matvec: streams each `Q4_K` weight row once per step for the
/// whole batch (rows stay cache-hot across streams). Per-stream block/`FMA`
/// order matches `neon_q4_k_mul_vec` exactly.
pub(super) fn neon_q4_k_mul_vec_batch(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
) -> Result<Vec<Vec<f32>>, InferenceError> {
    let batch = vectors.len();
    let blocks_per_row = cols / Q4_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row * Q4_K_BLOCK_BYTES;
    let mut flat = vec![0.0f32; rows * batch];
    bytes
        .par_chunks_exact(row_bytes)
        .zip(flat.par_chunks_exact_mut(batch))
        .with_min_len(64)
        .try_for_each(|(row_chunk, row_out)| {
            for (block_index, block) in row_chunk.chunks_exact(Q4_K_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q4_K_BLOCK_VALUES;
                for (out, vector) in row_out.iter_mut().zip(vectors.iter()) {
                    // SAFETY: the dispatch path checks NEON support, `block`
                    // has exactly one Q4_K block, and every vector was
                    // validated to `cols` (a multiple of 256).
                    *out += unsafe {
                        neon_q4_k_block_dot(block, &vector[col_base..col_base + Q4_K_BLOCK_VALUES])?
                    };
                }
            }
            Ok::<(), InferenceError>(())
        })?;

    Ok((0..batch)
        .map(|stream| (0..rows).map(|row| flat[row * batch + stream]).collect())
        .collect())
}

#[target_feature(enable = "neon")]
unsafe fn neon_q4_k_block_dot(block: &[u8], vector: &[f32]) -> Result<f32, InferenceError> {
    if block.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q4_K block byte length {} does not match {Q4_K_BLOCK_BYTES}",
            block.len()
        )));
    }
    if vector.len() != Q4_K_BLOCK_VALUES {
        return Err(InferenceError::new(format!(
            "Q4_K block vector length {} does not match {Q4_K_BLOCK_VALUES}",
            vector.len()
        )));
    }

    // SAFETY: NEON is enabled on this function and the block length check
    // above makes both half-precision scale reads valid.
    let (d, dmin) = unsafe {
        (
            native_f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]])),
            native_f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]])),
        )
    };
    let scales = &block[4..16];
    let quants = &block[16..];
    let mut lanes = vdupq_n_f32(0.0);
    let mut scale_index = 0usize;
    let mut vector_offset = 0usize;

    // SAFETY: `block` was length-checked above (128 quant bytes), and each
    // 64-value chunk reads a 64-element window of the length-checked
    // 256-element vector slice, so every 16-byte quant load and 4-lane
    // vector load stays in bounds.
    unsafe {
        let nibble_mask = vdupq_n_u8(0x0f);

        for quant_chunk in quants.chunks_exact(32) {
            let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
            let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
            let d_low = vdupq_n_f32(d * f32::from(scale_low));
            let d_high = vdupq_n_f32(d * f32::from(scale_high));
            let min_low = vdupq_n_f32(dmin * f32::from(min_low));
            let min_high = vdupq_n_f32(dmin * f32::from(min_high));

            let bytes_0 = vld1q_u8(quant_chunk.as_ptr());
            let bytes_1 = vld1q_u8(quant_chunk.as_ptr().add(16));

            // Nibble values are 0..16, so the u8→s8 reinterpret and exact
            // f32 widening preserve them; the previous kernel's FMA order
            // (all 8 low quads, then all 8 high quads) is replayed so the
            // sum stays bit-identical.
            let low_quads = [
                widen_s8_lanes(vreinterpretq_s8_u8(vandq_u8(bytes_0, nibble_mask))),
                widen_s8_lanes(vreinterpretq_s8_u8(vandq_u8(bytes_1, nibble_mask))),
            ];
            let high_quads = [
                widen_s8_lanes(vreinterpretq_s8_u8(vshrq_n_u8(bytes_0, 4))),
                widen_s8_lanes(vreinterpretq_s8_u8(vshrq_n_u8(bytes_1, 4))),
            ];

            for (group, quads) in low_quads.iter().enumerate() {
                for (step, quad) in quads.iter().enumerate() {
                    let lane_offset = group * 16 + step * 4;
                    let quant_values = vsubq_f32(vmulq_f32(*quad, d_low), min_low);
                    let vector_values = vld1q_f32(vector.as_ptr().add(vector_offset + lane_offset));
                    lanes = vfmaq_f32(lanes, quant_values, vector_values);
                }
            }
            for (group, quads) in high_quads.iter().enumerate() {
                for (step, quad) in quads.iter().enumerate() {
                    let lane_offset = group * 16 + step * 4;
                    let quant_values = vsubq_f32(vmulq_f32(*quad, d_high), min_high);
                    let vector_values =
                        vld1q_f32(vector.as_ptr().add(vector_offset + 32 + lane_offset));
                    lanes = vfmaq_f32(lanes, quant_values, vector_values);
                }
            }

            scale_index += 2;
            vector_offset += 64;
        }
    }

    Ok(vaddvq_f32(lanes))
}

fn q4_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::neon_q4_k_block_dot;
    use crate::scalar::{
        q4_k::{decode_q4_k_values, Q4_K_BLOCK_VALUES},
        InferenceError,
    };

    #[test]
    fn neon_q4_k_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
        let block = patterned_q4_k_block();
        let vector = (0..Q4_K_BLOCK_VALUES)
            .map(|index| (index % 11) as f32 - 5.0)
            .collect::<Vec<_>>();

        // SAFETY: the test provides one complete Q4_K block and a matching
        // 256-value activation on an aarch64 target with baseline NEON.
        let actual = unsafe { neon_q4_k_block_dot(&block, &vector)? };
        let expected = decode_q4_k_values(&block, Q4_K_BLOCK_VALUES)?
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

    fn patterned_q4_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        for index in 0..128 {
            let low = index as u8 & 0x0f;
            let high = 15 - low;
            block.push(low | (high << 4));
        }
        block
    }
}
