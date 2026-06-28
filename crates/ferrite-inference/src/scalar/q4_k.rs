#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q4_K_BLOCK_VALUES: usize = 256;
pub(super) const Q4_K_BLOCK_BYTES: usize = 144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q4KMatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Q4KMatVecOutput {
    pub(super) values: Vec<f32>,
    pub(super) backend: Q4KMatVecBackend,
}

pub(super) fn q4_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q4_k_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

pub(super) fn q4_k_mul_vec_with_backend(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q4KMatVecOutput, InferenceError> {
    validate_q4_k_mul_vec(bytes, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if cols != 0
            && cols.is_multiple_of(Q4_K_BLOCK_VALUES)
            && std::arch::is_aarch64_feature_detected!("neon")
        {
            return super::q4_k_neon::neon_q4_k_mul_vec(bytes, rows, cols, vector);
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if cols != 0
            && cols.is_multiple_of(Q4_K_BLOCK_VALUES)
            && std::arch::is_x86_feature_detected!("avx2")
        {
            return x86_64::avx2_q4_k_mul_vec(bytes, rows, cols, vector);
        }
    }

    scalar_q4_k_mul_vec(bytes, rows, cols, vector)
}

fn validate_q4_k_mul_vec(
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
        .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
    let expected = q4_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q4_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    Ok(())
}

pub(super) fn q4_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    if !value_count.is_multiple_of(Q4_K_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q4_K value count {value_count} must be divisible by {Q4_K_BLOCK_VALUES}"
        )));
    }

    value_count
        .checked_div(Q4_K_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q4_K_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q4_K byte length overflow"))
}

pub(super) fn decode_q4_k_values(
    bytes: &[u8],
    value_count: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected = q4_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q4_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(value_count);
    for block in bytes.chunks_exact(Q4_K_BLOCK_BYTES) {
        values.extend(q4_k_block_values(block)?);
    }
    Ok(values)
}

fn scalar_q4_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q4KMatVecOutput, InferenceError> {
    let mut values = vec![0.0; rows];
    for (block_index, block) in bytes.chunks_exact(Q4_K_BLOCK_BYTES).enumerate() {
        let value_offset = block_index
            .checked_mul(Q4_K_BLOCK_VALUES)
            .ok_or_else(|| InferenceError::new("Q4_K block value offset overflow"))?;
        accumulate_q4_k_block(block, value_offset, rows, cols, vector, &mut values)?;
    }

    Ok(Q4KMatVecOutput {
        values,
        backend: Q4KMatVecBackend::Scalar,
    })
}

pub(super) fn accumulate_q4_k_block(
    block: &[u8],
    value_offset: usize,
    rows: usize,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    if output.len() != rows {
        return Err(InferenceError::new(format!(
            "Q4_K output rows {} do not match {rows}",
            output.len()
        )));
    }

    for (local_offset, value) in q4_k_block_values(block)?.iter().enumerate() {
        accumulate_matrix_value(value_offset + local_offset, *value, cols, vector, output)?;
    }
    Ok(())
}

fn q4_k_block_values(block: &[u8]) -> Result<[f32; Q4_K_BLOCK_VALUES], InferenceError> {
    if block.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q4_K block byte length {} does not match {Q4_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    let scales = &block[4..16];
    let quants = &block[16..];
    let mut values = [0.0; Q4_K_BLOCK_VALUES];
    let mut scale_index = 0usize;
    let mut value_index = 0usize;

    for quant_chunk in quants.chunks_exact(32) {
        let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
        let d_low = d * f32::from(scale_low);
        let d_high = d * f32::from(scale_high);
        let min_low = dmin * f32::from(min_low);
        let min_high = dmin * f32::from(min_high);

        for quant in quant_chunk {
            values[value_index] = d_low * f32::from(quant & 0x0f) - min_low;
            value_index += 1;
        }
        for quant in quant_chunk {
            values[value_index] = d_high * f32::from(quant >> 4) - min_high;
            value_index += 1;
        }
        scale_index += 2;
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

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::{
        q4_k_block_values, Q4KMatVecBackend, Q4KMatVecOutput, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES,
    };
    use crate::scalar::InferenceError;
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
                    // SAFETY: `block_values` contains exactly 256 contiguous
                    // f32 values and `cols` is a multiple of 256, so every
                    // 8-lane load from the block and vector slice is in bounds.
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
            let left_lanes = _mm256_loadu_ps(left.add(offset));
            let right_lanes = _mm256_loadu_ps(right.add(offset));
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(left_lanes, right_lanes));
            offset += 8;
        }

        let mut partial = [0.0f32; 8];
        _mm256_storeu_ps(partial.as_mut_ptr(), lanes);
        partial.iter().sum()
    }
}
