#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q8_0_BLOCK_VALUES: usize = 32;
pub(super) const Q8_0_BLOCK_BYTES: usize = 34;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q8_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "aarch64")]
    Aarch64NeonRowParallel,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2RowParallel,
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
            return Ok(aarch64::neon_q8_0_mul_vec(bytes, rows, cols, vector));
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

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    use super::{
        f16_bits_to_f32, Q8_0MatVecBackend, Q8_0MatVecOutput, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES,
    };
    use rayon::prelude::*;
    use std::arch::aarch64::{
        vaddvq_f32, vcvtq_f32_s32, vdupq_n_f32, vfmaq_f32, vget_high_s16, vget_low_s16, vld1_s8,
        vld1q_f32, vmovl_s16, vmovl_s8,
    };

    pub(super) fn neon_q8_0_mul_vec(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Q8_0MatVecOutput {
        let row_bytes = (cols / Q8_0_BLOCK_VALUES) * Q8_0_BLOCK_BYTES;
        let backend = if rows > 1 {
            Q8_0MatVecBackend::Aarch64NeonRowParallel
        } else {
            Q8_0MatVecBackend::Aarch64Neon
        };
        let values: Vec<f32> = if rows > 1 {
            bytes
                .par_chunks_exact(row_bytes)
                .map(|row_bytes| neon_q8_0_row_dot(row_bytes, vector))
                .collect()
        } else {
            bytes
                .chunks_exact(row_bytes)
                .map(|row_bytes| neon_q8_0_row_dot(row_bytes, vector))
                .collect()
        };
        debug_assert_eq!(values.len(), rows);

        Q8_0MatVecOutput { values, backend }
    }

    fn neon_q8_0_row_dot(row_bytes: &[u8], vector: &[f32]) -> f32 {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            // SAFETY: each Q8_0 block has exactly 32 quantized bytes and
            // `cols` is validated as a multiple of 32, so every 8-byte
            // quant load and matching 4-lane vector load is in bounds.
            sum += unsafe {
                neon_q8_0_block_dot(
                    block[2..].as_ptr().cast::<i8>(),
                    vector[col_base..].as_ptr(),
                )
            } * scale;
        }
        sum
    }

    #[target_feature(enable = "neon")]
    unsafe fn neon_q8_0_block_dot(quantized: *const i8, vector: *const f32) -> f32 {
        let mut lanes = vdupq_n_f32(0.0);
        let mut offset = 0usize;
        while offset < Q8_0_BLOCK_VALUES {
            let quantized_i8 = vld1_s8(quantized.add(offset));
            let quantized_i16 = vmovl_s8(quantized_i8);

            let low_i32 = vmovl_s16(vget_low_s16(quantized_i16));
            let low_f32 = vcvtq_f32_s32(low_i32);
            let low_vector = vld1q_f32(vector.add(offset));
            lanes = vfmaq_f32(lanes, low_f32, low_vector);

            let high_i32 = vmovl_s16(vget_high_s16(quantized_i16));
            let high_f32 = vcvtq_f32_s32(high_i32);
            let high_vector = vld1q_f32(vector.add(offset + 4));
            lanes = vfmaq_f32(lanes, high_f32, high_vector);

            offset += 8;
        }
        vaddvq_f32(lanes)
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::{
        f16_bits_to_f32, Q8_0MatVecBackend, Q8_0MatVecOutput, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_VALUES,
    };
    use rayon::prelude::*;
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
        let backend = if rows > 1 {
            Q8_0MatVecBackend::X86_64Avx2RowParallel
        } else {
            Q8_0MatVecBackend::X86_64Avx2
        };
        let values: Vec<f32> = if rows > 1 {
            bytes
                .par_chunks_exact(row_bytes)
                .map(|row_bytes| avx2_q8_0_row_dot(row_bytes, vector))
                .collect()
        } else {
            bytes
                .chunks_exact(row_bytes)
                .map(|row_bytes| avx2_q8_0_row_dot(row_bytes, vector))
                .collect()
        };
        debug_assert_eq!(values.len(), rows);

        Q8_0MatVecOutput { values, backend }
    }

    fn avx2_q8_0_row_dot(row_bytes: &[u8], vector: &[f32]) -> f32 {
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
