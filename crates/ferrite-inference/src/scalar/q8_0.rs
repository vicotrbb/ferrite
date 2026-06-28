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
            return Ok(x86_64::avx2_q8_0_argmax_mul_vec(bytes, rows, cols, vector));
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
            return Ok(x86_64::avx2_q8_0_mul_vec(bytes, rows, cols, vector));
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

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::{
        f16_bits_to_f32, Q8_0MatVecBackend, Q8_0MatVecOutput, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES,
    };
    use std::arch::x86_64::{
        __m128i, _mm256_add_ps, _mm256_cvtepi32_ps, _mm256_cvtepi8_epi32, _mm256_loadu_ps,
        _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps, _mm_loadl_epi64,
    };

    pub(super) fn avx2_q8_0_mul_vec(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Q8_0MatVecOutput {
        let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
        let mut values = vec![0.0; rows];
        for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
            let mut sum = 0.0;
            for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
                let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
                let col_base = block_index * Q8_0_BLOCK_VALUES;
                // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
                // `cols` is validated as a multiple of 32, so every 8-byte
                // quant load and matching 8-lane vector load is in bounds.
                sum += unsafe {
                    avx2_q8_0_block_dot(
                        block[2..].as_ptr().cast::<i8>(),
                        vector[col_base..].as_ptr(),
                    )
                } * scale;
            }
            values[row] = sum;
        }

        Q8_0MatVecOutput {
            values,
            backend: Q8_0MatVecBackend::X86_64Avx2,
        }
    }

    pub(super) fn avx2_q8_0_argmax_mul_vec(
        bytes: &[u8],
        _rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> usize {
        let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
        super::argmax_q8_0_rows(bytes, row_bytes, |row_bytes| {
            let mut sum = 0.0;
            for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
                let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
                let col_base = block_index * Q8_0_BLOCK_VALUES;
                // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
                // `cols` is validated as a multiple of 32, so every 8-byte
                // quant load and matching 8-lane vector load is in bounds.
                sum += unsafe {
                    avx2_q8_0_block_dot(
                        block[2..].as_ptr().cast::<i8>(),
                        vector[col_base..].as_ptr(),
                    )
                } * scale;
            }
            sum
        })
    }

    #[target_feature(enable = "avx2")]
    unsafe fn avx2_q8_0_block_dot(quantized: *const i8, vector: *const f32) -> f32 {
        let mut lanes = _mm256_setzero_ps();
        let mut offset = 0usize;
        while offset < Q8_0_BLOCK_VALUES {
            let quantized_i8 = _mm_loadl_epi64(quantized.add(offset).cast::<__m128i>());
            let quantized_i32 = _mm256_cvtepi8_epi32(quantized_i8);
            let quantized_f32 = _mm256_cvtepi32_ps(quantized_i32);
            let vector_lanes = _mm256_loadu_ps(vector.add(offset));
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(quantized_f32, vector_lanes));
            offset += 8;
        }

        let mut partial = [0.0f32; 8];
        _mm256_storeu_ps(partial.as_mut_ptr(), lanes);
        partial.iter().sum()
    }
}
