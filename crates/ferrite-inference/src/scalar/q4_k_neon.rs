#![allow(unsafe_code)]

use super::{
    float::f16_bits_to_f32,
    q4_k::{Q4KMatVecBackend, Q4KMatVecOutput, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32, vmulq_f32, vsubq_f32};

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

    let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    let scales = &block[4..16];
    let quants = &block[16..];
    let mut lanes = vdupq_n_f32(0.0);
    let mut scale_index = 0usize;
    let mut vector_offset = 0usize;

    for quant_chunk in quants.chunks_exact(32) {
        let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
        let d_low = vdupq_n_f32(d * f32::from(scale_low));
        let d_high = vdupq_n_f32(d * f32::from(scale_high));
        let min_low = vdupq_n_f32(dmin * f32::from(min_low));
        let min_high = vdupq_n_f32(dmin * f32::from(min_high));

        for lane_offset in (0..32).step_by(4) {
            let quant_lanes = [
                f32::from(quant_chunk[lane_offset] & 0x0f),
                f32::from(quant_chunk[lane_offset + 1] & 0x0f),
                f32::from(quant_chunk[lane_offset + 2] & 0x0f),
                f32::from(quant_chunk[lane_offset + 3] & 0x0f),
            ];
            // SAFETY: the temporary lane array and the vector slice both have
            // at least four contiguous f32 values from this offset.
            unsafe {
                let quant_values =
                    vsubq_f32(vmulq_f32(vld1q_f32(quant_lanes.as_ptr()), d_low), min_low);
                let vector_values = vld1q_f32(vector.as_ptr().add(vector_offset + lane_offset));
                lanes = vfmaq_f32(lanes, quant_values, vector_values);
            }
        }

        for lane_offset in (0..32).step_by(4) {
            let quant_lanes = [
                f32::from(quant_chunk[lane_offset] >> 4),
                f32::from(quant_chunk[lane_offset + 1] >> 4),
                f32::from(quant_chunk[lane_offset + 2] >> 4),
                f32::from(quant_chunk[lane_offset + 3] >> 4),
            ];
            // SAFETY: the temporary lane array and the vector slice both have
            // at least four contiguous f32 values from this offset.
            unsafe {
                let quant_values =
                    vsubq_f32(vmulq_f32(vld1q_f32(quant_lanes.as_ptr()), d_high), min_high);
                let vector_values =
                    vld1q_f32(vector.as_ptr().add(vector_offset + 32 + lane_offset));
                lanes = vfmaq_f32(lanes, quant_values, vector_values);
            }
        }

        scale_index += 2;
        vector_offset += 64;
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
