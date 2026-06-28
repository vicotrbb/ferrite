#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q8_0_BLOCK_VALUES: usize = 32;
pub(super) const Q8_0_BLOCK_BYTES: usize = 34;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q8_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Q8_0MatVecOutput {
    pub(super) values: Vec<f32>,
    pub(super) backend: Q8_0MatVecBackend,
}

pub(super) fn q8_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    if !cols.is_multiple_of(Q8_0_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q8_0 value count {cols} must be divisible by {Q8_0_BLOCK_VALUES}"
        )));
    }
    cols.checked_div(Q8_0_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q8_0_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q8_0 byte length overflow"))
}

pub(super) fn q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q8_0_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

pub(super) fn q8_0_argmax_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    validate_q8_0_mul_vec(bytes, rows, cols, vector)?;
    if rows == 0 {
        return Err(InferenceError::new("argmax input must not be empty"));
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return Ok(super::q8_0_neon::neon_q8_0_argmax_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            return Ok(super::q8_0_avx2::avx2_q8_0_argmax_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }

    scalar_q8_0_argmax_mul_vec(bytes, rows, cols, vector)
}

pub(super) fn q8_0_mul_vec_with_backend(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q8_0MatVecOutput, InferenceError> {
    validate_q8_0_mul_vec(bytes, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return Ok(super::q8_0_neon::neon_q8_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            return Ok(super::q8_0_avx2::avx2_q8_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }

    scalar_q8_0_mul_vec(bytes, rows, cols, vector)
}

pub(super) fn decode_q8_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
    let expected = q8_0_row_bytes(cols)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q8_0 row byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(cols);
    for block in bytes.chunks_exact(Q8_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for quantized in &block[2..] {
            values.push(scale * f32::from(*quantized as i8));
        }
    }
    Ok(values)
}

fn validate_q8_0_mul_vec(
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
    let row_bytes = q8_0_row_bytes(cols)?;
    let expected = rows
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("Q8_0 matrix byte length overflow"))?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q8_0 matrix byte length {} does not match shape {rows}x{cols}",
            bytes.len()
        )));
    }
    Ok(())
}

fn scalar_q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q8_0MatVecOutput, InferenceError> {
    let row_bytes = q8_0_row_bytes(cols)?;
    let mut values = vec![0.0; rows];
    for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            for (offset, quantized) in block[2..].iter().enumerate() {
                sum += scale * f32::from(*quantized as i8) * vector[col_base + offset];
            }
        }
        values[row] = sum;
    }

    Ok(Q8_0MatVecOutput {
        values,
        backend: Q8_0MatVecBackend::Scalar,
    })
}

fn scalar_q8_0_argmax_mul_vec(
    bytes: &[u8],
    _rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    let row_bytes = q8_0_row_bytes(cols)?;
    Ok(argmax_q8_0_rows(bytes, row_bytes, |row_bytes| {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            for (offset, quantized) in block[2..].iter().enumerate() {
                sum += scale * f32::from(*quantized as i8) * vector[col_base + offset];
            }
        }
        sum
    }))
}

pub(super) fn argmax_q8_0_rows<F>(bytes: &[u8], row_bytes: usize, mut row_dot: F) -> usize
where
    F: FnMut(&[u8]) -> f32,
{
    let mut best_index = 0usize;
    let mut best_value = f32::NEG_INFINITY;

    for (row_index, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
        let sum = row_dot(row_chunk);
        if row_index == 0 || sum > best_value {
            best_index = row_index;
            best_value = sum;
        }
    }

    best_index
}
