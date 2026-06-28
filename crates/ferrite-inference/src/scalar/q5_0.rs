#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q5_0_BLOCK_VALUES: usize = 32;
pub(super) const Q5_0_BLOCK_BYTES: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q5_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Q5_0MatVecOutput {
    pub(super) values: Vec<f32>,
    pub(super) backend: Q5_0MatVecBackend,
}

pub(super) fn q5_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    if !cols.is_multiple_of(Q5_0_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q5_0 value count {cols} must be divisible by {Q5_0_BLOCK_VALUES}"
        )));
    }
    cols.checked_div(Q5_0_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q5_0_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q5_0 byte length overflow"))
}

pub(super) fn q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q5_0_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

pub(super) fn q5_0_mul_vec_with_backend(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q5_0MatVecOutput, InferenceError> {
    validate_q5_0_mul_vec(bytes, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return Ok(aarch64::neon_q5_0_mul_vec(bytes, rows, cols, vector));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            return Ok(x86_64::avx2_q5_0_mul_vec(bytes, rows, cols, vector));
        }
    }

    scalar_q5_0_mul_vec(bytes, rows, cols, vector)
}

pub(super) fn decode_q5_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
    let expected = q5_0_row_bytes(cols)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q5_0 row byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(cols);
    for block in bytes.chunks_exact(Q5_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let signed = q5_0_signed_values(block);
        values.extend(signed.into_iter().map(|value| scale * value as f32));
    }
    Ok(values)
}

fn validate_q5_0_mul_vec(
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
    let row_bytes = q5_0_row_bytes(cols)?;
    let expected = rows
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("Q5_0 matrix byte length overflow"))?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q5_0 matrix byte length {} does not match shape {rows}x{cols}",
            bytes.len()
        )));
    }
    Ok(())
}

fn scalar_q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q5_0MatVecOutput, InferenceError> {
    let row_bytes = q5_0_row_bytes(cols)?;
    let mut values = vec![0.0; rows];
    for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let signed = q5_0_signed_values(block);
            let col_base = block_index * Q5_0_BLOCK_VALUES;
            for (offset, value) in signed.iter().enumerate() {
                sum += scale * *value as f32 * vector[col_base + offset];
            }
        }
        values[row] = sum;
    }

    Ok(Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::Scalar,
    })
}

fn q5_0_signed_values(block: &[u8]) -> [i8; Q5_0_BLOCK_VALUES] {
    let high_bits = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
    let quants = &block[6..];
    let mut values = [0i8; Q5_0_BLOCK_VALUES];

    for (index, quant) in quants.iter().enumerate() {
        let high = ((high_bits >> index) << 4) as u8 & 0x10;
        values[index] = ((quant & 0x0f) | high) as i8 - 16;
    }

    for (index, quant) in quants.iter().enumerate() {
        let high = (high_bits >> (index + 12)) as u8 & 0x10;
        values[index + 16] = ((quant >> 4) | high) as i8 - 16;
    }

    values
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    use super::{
        f16_bits_to_f32, Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES,
    };
    use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32};

    pub(super) fn neon_q5_0_mul_vec(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Q5_0MatVecOutput {
        let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
        let mut values = vec![0.0; rows];
        for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
            let mut sum = 0.0;
            for (block_index, block) in row_bytes.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
                let col_base = block_index * Q5_0_BLOCK_VALUES;
                // SAFETY: the dispatch path checks NEON support, `block` has
                // exactly one Q5_0 block, and `cols` is a multiple of 32 so the
                // per-block vector slice is in bounds.
                sum += unsafe {
                    neon_q5_0_block_dot(block, &vector[col_base..col_base + Q5_0_BLOCK_VALUES])
                };
            }
            values[row] = sum;
        }

        Q5_0MatVecOutput {
            values,
            backend: Q5_0MatVecBackend::Aarch64Neon,
        }
    }

    #[target_feature(enable = "neon")]
    unsafe fn neon_q5_0_block_dot(block: &[u8], vector: &[f32]) -> f32 {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let high_bits = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
        let quants = &block[6..];
        let mut lanes = vdupq_n_f32(0.0);

        for lane_base in (0..16).step_by(4) {
            let mut low_values = [0.0; 4];
            let mut high_values = [0.0; 4];
            for lane in 0..4 {
                let index = lane_base + lane;
                let quant = quants[index];
                let low_high_bit = ((high_bits >> index) << 4) as u8 & 0x10;
                let high_high_bit = (high_bits >> (index + 12)) as u8 & 0x10;
                low_values[lane] = f32::from(((quant & 0x0f) | low_high_bit) as i8 - 16);
                high_values[lane] = f32::from(((quant >> 4) | high_high_bit) as i8 - 16);
            }

            // SAFETY: the temporary lane arrays contain four contiguous f32
            // values, and the vector slice was validated to contain exactly one
            // Q5_0 block worth of elements.
            unsafe {
                let low_vector = vld1q_f32(vector.as_ptr().add(lane_base));
                lanes = vfmaq_f32(lanes, vld1q_f32(low_values.as_ptr()), low_vector);

                let high_vector = vld1q_f32(vector.as_ptr().add(lane_base + 16));
                lanes = vfmaq_f32(lanes, vld1q_f32(high_values.as_ptr()), high_vector);
            }
        }

        vaddvq_f32(lanes) * scale
    }

    #[cfg(test)]
    mod tests {
        use super::neon_q5_0_block_dot;
        use crate::scalar::{
            q5_0::{decode_q5_0_row, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
            InferenceError,
        };

        #[test]
        fn neon_q5_0_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
            let block = patterned_q5_0_block();
            let vector = (0..Q5_0_BLOCK_VALUES)
                .map(|index| (index % 9) as f32 - 4.0)
                .collect::<Vec<_>>();

            let actual = unsafe { neon_q5_0_block_dot(&block, &vector) };
            let expected = decode_q5_0_row(&block, Q5_0_BLOCK_VALUES)?
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

        fn patterned_q5_0_block() -> Vec<u8> {
            let mut block = Vec::with_capacity(Q5_0_BLOCK_BYTES);
            block.extend_from_slice(&0x3c00u16.to_le_bytes());
            block.extend_from_slice(&0xa5c33c5au32.to_le_bytes());
            for index in 0..16 {
                let low = (index * 3) as u8 & 0x0f;
                let high = (15 - index) as u8 & 0x0f;
                block.push(low | (high << 4));
            }
            block
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::{
        f16_bits_to_f32, q5_0_signed_values, Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES,
        Q5_0_BLOCK_VALUES,
    };
    use std::arch::x86_64::{
        __m128i, _mm256_add_ps, _mm256_cvtepi32_ps, _mm256_cvtepi8_epi32, _mm256_loadu_ps,
        _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps, _mm_loadl_epi64,
    };

    pub(super) fn avx2_q5_0_mul_vec(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Q5_0MatVecOutput {
        let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
        let mut values = vec![0.0; rows];
        for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
            let mut sum = 0.0;
            for (block_index, block) in row_bytes.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
                let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
                let signed = q5_0_signed_values(block);
                let col_base = block_index * Q5_0_BLOCK_VALUES;
                // SAFETY: `signed` contains exactly 32 decoded Q5_0 values and
                // `cols` is validated as a multiple of 32, so every 8-byte
                // signed load and matching 8-lane vector load is in bounds.
                sum +=
                    unsafe { avx2_i8_f32_block_dot(signed.as_ptr(), vector[col_base..].as_ptr()) }
                        * scale;
            }
            values[row] = sum;
        }

        Q5_0MatVecOutput {
            values,
            backend: Q5_0MatVecBackend::X86_64Avx2,
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn avx2_i8_f32_block_dot(signed: *const i8, vector: *const f32) -> f32 {
        let mut lanes = _mm256_setzero_ps();
        let mut offset = 0usize;
        while offset < Q5_0_BLOCK_VALUES {
            let signed_i8 = _mm_loadl_epi64(signed.add(offset).cast::<__m128i>());
            let signed_i32 = _mm256_cvtepi8_epi32(signed_i8);
            let signed_f32 = _mm256_cvtepi32_ps(signed_i32);
            let vector_lanes = _mm256_loadu_ps(vector.add(offset));
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(signed_f32, vector_lanes));
            offset += 8;
        }

        let mut partial = [0.0f32; 8];
        _mm256_storeu_ps(partial.as_mut_ptr(), lanes);
        partial.iter().sum()
    }
}
