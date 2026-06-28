#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q5_0_BLOCK_VALUES: usize = 32;
pub(super) const Q5_0_BLOCK_BYTES: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q5_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
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
        f16_bits_to_f32, q5_0_signed_values, Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES,
        Q5_0_BLOCK_VALUES,
    };
    use std::arch::aarch64::{
        vaddvq_f32, vcvtq_f32_s32, vdupq_n_f32, vfmaq_f32, vld1_s8, vld1q_f32, vmovl_s16, vmovl_s8,
    };

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
                let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
                let signed = q5_0_signed_values(block);
                let col_base = block_index * Q5_0_BLOCK_VALUES;
                // SAFETY: `signed` contains 32 decoded Q5_0 values and `cols`
                // is validated as a multiple of 32, so every 8-byte signed load
                // and matching 4-lane vector load is in bounds.
                sum +=
                    unsafe { neon_i8_f32_block_dot(signed.as_ptr(), vector[col_base..].as_ptr()) }
                        * scale;
            }
            values[row] = sum;
        }

        Q5_0MatVecOutput {
            values,
            backend: Q5_0MatVecBackend::Aarch64Neon,
        }
    }

    #[target_feature(enable = "neon")]
    unsafe fn neon_i8_f32_block_dot(signed: *const i8, vector: *const f32) -> f32 {
        let mut lanes = vdupq_n_f32(0.0);
        let mut offset = 0usize;
        while offset < Q5_0_BLOCK_VALUES {
            let signed_i8 = vld1_s8(signed.add(offset));
            let signed_i16 = vmovl_s8(signed_i8);
            let low_i32 = vmovl_s16(std::arch::aarch64::vget_low_s16(signed_i16));
            let high_i32 = vmovl_s16(std::arch::aarch64::vget_high_s16(signed_i16));

            let low_f32 = vcvtq_f32_s32(low_i32);
            let low_vector = vld1q_f32(vector.add(offset));
            lanes = vfmaq_f32(lanes, low_f32, low_vector);

            let high_f32 = vcvtq_f32_s32(high_i32);
            let high_vector = vld1q_f32(vector.add(offset + 4));
            lanes = vfmaq_f32(lanes, high_f32, high_vector);

            offset += 8;
        }
        vaddvq_f32(lanes)
    }
}
