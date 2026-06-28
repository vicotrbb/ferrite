#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q6_K_BLOCK_VALUES: usize = 256;
pub(super) const Q6_K_BLOCK_BYTES: usize = 210;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q6KMatVecBackend {
    Scalar,
    Aarch64Neon,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Q6KMatVecOutput {
    pub(super) values: Vec<f32>,
    pub(super) backend: Q6KMatVecBackend,
}

pub(super) fn q6_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q6_k_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

pub(super) fn q6_k_mul_vec_with_backend(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q6KMatVecOutput, InferenceError> {
    validate_q6_k_mul_vec(bytes, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if cols != 0
            && cols.is_multiple_of(Q6_K_BLOCK_VALUES)
            && std::arch::is_aarch64_feature_detected!("neon")
        {
            return aarch64::neon_q6_k_mul_vec(bytes, rows, cols, vector);
        }
    }

    scalar_q6_k_mul_vec(bytes, rows, cols, vector)
}

fn validate_q6_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(), InferenceError> {
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?;
    let expected = q6_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q6_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    Ok(())
}

pub(super) fn q6_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    if !value_count.is_multiple_of(Q6_K_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q6_K value count {value_count} must be divisible by {Q6_K_BLOCK_VALUES}"
        )));
    }

    value_count
        .checked_div(Q6_K_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q6_K_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q6_K byte length overflow"))
}

pub(super) fn decode_q6_k_values(
    bytes: &[u8],
    value_count: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected = q6_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q6_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(value_count);
    for block in bytes.chunks_exact(Q6_K_BLOCK_BYTES) {
        values.extend(q6_k_block_values(block)?);
    }
    Ok(values)
}

fn scalar_q6_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q6KMatVecOutput, InferenceError> {
    let mut values = vec![0.0; rows];
    for (block_index, block) in bytes.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
        let value_offset = block_index
            .checked_mul(Q6_K_BLOCK_VALUES)
            .ok_or_else(|| InferenceError::new("Q6_K block value offset overflow"))?;
        accumulate_q6_k_block(block, value_offset, rows, cols, vector, &mut values)?;
    }

    Ok(Q6KMatVecOutput {
        values,
        backend: Q6KMatVecBackend::Scalar,
    })
}

pub(super) fn accumulate_q6_k_block(
    block: &[u8],
    value_offset: usize,
    rows: usize,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    if output.len() != rows {
        return Err(InferenceError::new(format!(
            "Q6_K output rows {} do not match {rows}",
            output.len()
        )));
    }

    for (local_offset, value) in q6_k_block_values(block)?.iter().enumerate() {
        accumulate_matrix_value(value_offset + local_offset, *value, cols, vector, output)?;
    }
    Ok(())
}

fn q6_k_block_values(block: &[u8]) -> Result<[f32; Q6_K_BLOCK_VALUES], InferenceError> {
    if block.len() != Q6_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q6_K block byte length {} does not match {Q6_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    let low_bits = &block[0..128];
    let high_bits = &block[128..192];
    let scales = &block[192..208];
    let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));
    let mut values = [0.0; Q6_K_BLOCK_VALUES];

    for half in 0..2 {
        let value_base = half * 128;
        let low_base = half * 64;
        let high_base = half * 32;
        let scale_base = half * 8;

        for index in 0..32 {
            let scale_index = index / 16;
            let high = high_bits[high_base + index];
            let q1 = i32::from((low_bits[low_base + index] & 0x0f) | ((high & 3) << 4)) - 32;
            let q2 =
                i32::from((low_bits[low_base + index + 32] & 0x0f) | (((high >> 2) & 3) << 4)) - 32;
            let q3 = i32::from((low_bits[low_base + index] >> 4) | (((high >> 4) & 3) << 4)) - 32;
            let q4 =
                i32::from((low_bits[low_base + index + 32] >> 4) | (((high >> 6) & 3) << 4)) - 32;

            values[value_base + index] =
                super_scale * f32::from(scales[scale_base + scale_index] as i8) * q1 as f32;
            values[value_base + index + 32] =
                super_scale * f32::from(scales[scale_base + scale_index + 2] as i8) * q2 as f32;
            values[value_base + index + 64] =
                super_scale * f32::from(scales[scale_base + scale_index + 4] as i8) * q3 as f32;
            values[value_base + index + 96] =
                super_scale * f32::from(scales[scale_base + scale_index + 6] as i8) * q4 as f32;
        }
    }

    Ok(values)
}

fn accumulate_matrix_value(
    index: usize,
    value: f32,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    let row = index / cols;
    let col = index % cols;
    let target = output
        .get_mut(row)
        .ok_or_else(|| InferenceError::new("quantized matrix row index out of bounds"))?;
    let vector_value = vector
        .get(col)
        .ok_or_else(|| InferenceError::new("quantized matrix column index out of bounds"))?;
    *target += value * vector_value;
    Ok(())
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    use super::{
        q6_k_block_values, Q6KMatVecBackend, Q6KMatVecOutput, Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES,
    };
    use crate::scalar::InferenceError;
    use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32};

    pub(super) fn neon_q6_k_mul_vec(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Result<Q6KMatVecOutput, InferenceError> {
        let blocks_per_row = cols / Q6_K_BLOCK_VALUES;
        let row_bytes = blocks_per_row * Q6_K_BLOCK_BYTES;
        let mut values = vec![0.0; rows];

        for (row, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
            let mut sum = 0.0;
            for (block_index, block) in row_chunk.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
                let block_values = q6_k_block_values(block)?;
                let col_base = block_index * Q6_K_BLOCK_VALUES;
                // SAFETY: `block_values` contains exactly 256 contiguous f32
                // values and `cols` is a multiple of 256, so every 4-lane load
                // from the block and vector slice is in bounds.
                sum += unsafe {
                    neon_f32_block_dot(block_values.as_ptr(), vector[col_base..].as_ptr())
                };
            }
            values[row] = sum;
        }

        Ok(Q6KMatVecOutput {
            values,
            backend: Q6KMatVecBackend::Aarch64Neon,
        })
    }

    #[target_feature(enable = "neon")]
    unsafe fn neon_f32_block_dot(left: *const f32, right: *const f32) -> f32 {
        let mut lanes = vdupq_n_f32(0.0);
        let mut offset = 0usize;
        while offset < Q6_K_BLOCK_VALUES {
            let left_lanes = vld1q_f32(left.add(offset));
            let right_lanes = vld1q_f32(right.add(offset));
            lanes = vfmaq_f32(lanes, left_lanes, right_lanes);
            offset += 4;
        }
        vaddvq_f32(lanes)
    }
}
