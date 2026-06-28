#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q6_k::{Q6KMatVecBackend, Q6KMatVecOutput, Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES},
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32, vmulq_f32};

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

#[target_feature(enable = "neon")]
unsafe fn neon_q6_k_block_dot(block: &[u8], vector: &[f32]) -> Result<f32, InferenceError> {
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

    for half in 0..2 {
        let value_base = half * 128;
        let low_base = half * 64;
        let high_base = half * 32;
        let scale_base = half * 8;

        for index in (0..32).step_by(4) {
            let scale_index = index / 16;
            let scale_1 =
                vdupq_n_f32(super_scale * f32::from(scales[scale_base + scale_index] as i8));
            let scale_2 =
                vdupq_n_f32(super_scale * f32::from(scales[scale_base + scale_index + 2] as i8));
            let scale_3 =
                vdupq_n_f32(super_scale * f32::from(scales[scale_base + scale_index + 4] as i8));
            let scale_4 =
                vdupq_n_f32(super_scale * f32::from(scales[scale_base + scale_index + 6] as i8));
            let mut q1 = [0.0; 4];
            let mut q2 = [0.0; 4];
            let mut q3 = [0.0; 4];
            let mut q4 = [0.0; 4];

            for lane in 0..4 {
                let offset = index + lane;
                let high = high_bits[high_base + offset];
                q1[lane] = (i32::from((low_bits[low_base + offset] & 0x0f) | ((high & 3) << 4))
                    - 32) as f32;
                q2[lane] = (i32::from(
                    (low_bits[low_base + offset + 32] & 0x0f) | (((high >> 2) & 3) << 4),
                ) - 32) as f32;
                q3[lane] =
                    (i32::from((low_bits[low_base + offset] >> 4) | (((high >> 4) & 3) << 4)) - 32)
                        as f32;
                q4[lane] =
                    (i32::from((low_bits[low_base + offset + 32] >> 4) | (((high >> 6) & 3) << 4))
                        - 32) as f32;
            }

            // SAFETY: the temporary lane arrays and the vector slice each have
            // four contiguous f32 values at these offsets.
            unsafe {
                let vector_values = vld1q_f32(vector.as_ptr().add(value_base + index));
                lanes = vfmaq_f32(
                    lanes,
                    vmulq_f32(vld1q_f32(q1.as_ptr()), scale_1),
                    vector_values,
                );

                let vector_values = vld1q_f32(vector.as_ptr().add(value_base + index + 32));
                lanes = vfmaq_f32(
                    lanes,
                    vmulq_f32(vld1q_f32(q2.as_ptr()), scale_2),
                    vector_values,
                );

                let vector_values = vld1q_f32(vector.as_ptr().add(value_base + index + 64));
                lanes = vfmaq_f32(
                    lanes,
                    vmulq_f32(vld1q_f32(q3.as_ptr()), scale_3),
                    vector_values,
                );

                let vector_values = vld1q_f32(vector.as_ptr().add(value_base + index + 96));
                lanes = vfmaq_f32(
                    lanes,
                    vmulq_f32(vld1q_f32(q4.as_ptr()), scale_4),
                    vector_values,
                );
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
