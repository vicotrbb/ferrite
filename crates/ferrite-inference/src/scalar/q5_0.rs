#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, InferenceError};

pub(super) const Q5_0_BLOCK_VALUES: usize = 32;
pub(super) const Q5_0_BLOCK_BYTES: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q5_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "aarch64")]
    Aarch64NeonRowParallel,
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

pub(super) fn validate_q5_0_finite_scales(bytes: &[u8]) -> Result<(), InferenceError> {
    for block in bytes.chunks_exact(Q5_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        if !scale.is_finite() {
            return Err(InferenceError::new(
                "Q5_0 matrix scale values must be finite",
            ));
        }
    }
    Ok(())
}

pub(super) fn q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q5_0_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

/// Multiplies two same-shaped Q5_0 matrices by one activation vector while
/// exposing both independent dot products to each worker. Per-matrix block
/// order is unchanged, so results are bit-identical to two standalone calls.
pub(super) fn q5_0_mul_vec_pair(
    left: &[u8],
    right: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(Vec<f32>, Vec<f32>), InferenceError> {
    validate_q5_0_mul_vec(left, rows, cols, vector)?;
    validate_q5_0_mul_vec(right, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return Ok(super::q5_0_neon::neon_q5_0_mul_vec_pair(
                left, right, rows, cols, vector,
            ));
        }
    }

    let left = scalar_q5_0_mul_vec(left, rows, cols, vector)?.values;
    let right = scalar_q5_0_mul_vec(right, rows, cols, vector)?.values;
    Ok((left, right))
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
            return Ok(super::q5_0_neon::neon_q5_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            return Ok(super::q5_0_avx2::avx2_q5_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }

    scalar_q5_0_mul_vec(bytes, rows, cols, vector)
}

/// Upper bound on how many activation vectors one batched matvec call
/// serves; larger batches are processed in chunks of this size.
pub(super) const Q5_0_MAX_BATCH: usize = 8;

/// Batched matvec across several activation vectors. Each stream's output
/// is bit-identical to a `q5_0_mul_vec` call with that vector.
pub(super) fn q5_0_mul_vec_batch(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
) -> Result<Vec<Vec<f32>>, InferenceError> {
    let Some(first) = vectors.first() else {
        return Ok(Vec::new());
    };
    for vector in vectors {
        if vector.len() != cols {
            return Err(InferenceError::new(format!(
                "matrix columns {cols} do not match vector length {}",
                vector.len()
            )));
        }
    }
    validate_q5_0_mul_vec(bytes, rows, cols, first)?;

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut outputs = Vec::with_capacity(vectors.len());
            for chunk in vectors.chunks(Q5_0_MAX_BATCH) {
                outputs.extend(super::q5_0_neon::neon_q5_0_mul_vec_batch(
                    bytes, rows, cols, chunk,
                ));
            }
            return Ok(outputs);
        }
    }

    vectors
        .iter()
        .map(|vector| q5_0_mul_vec(bytes, rows, cols, vector))
        .collect()
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

pub(super) fn q5_0_signed_values(block: &[u8]) -> [i8; Q5_0_BLOCK_VALUES] {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_matvec_is_bit_identical_to_independent_calls() -> Result<(), InferenceError> {
        let rows = 512;
        let cols = 64;
        let blocks = rows * cols / Q5_0_BLOCK_VALUES;
        let left = patterned_matrix(blocks, 17);
        let right = patterned_matrix(blocks, 93);
        let vector = (0..cols)
            .map(|index| ((index * 13 % 31) as f32 - 15.0) / 7.0)
            .collect::<Vec<_>>();

        let expected_left = q5_0_mul_vec(&left, rows, cols, &vector)?;
        let expected_right = q5_0_mul_vec(&right, rows, cols, &vector)?;
        let (actual_left, actual_right) = q5_0_mul_vec_pair(&left, &right, rows, cols, &vector)?;

        assert_eq!(actual_left, expected_left);
        assert_eq!(actual_right, expected_right);
        Ok(())
    }

    fn patterned_matrix(blocks: usize, seed: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(blocks * Q5_0_BLOCK_BYTES);
        for block_index in 0..blocks {
            bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
            let high_bits = (block_index.wrapping_mul(0x9e37_79b9) ^ seed) as u32;
            bytes.extend_from_slice(&high_bits.to_le_bytes());
            bytes.extend((0..16).map(|index| {
                block_index
                    .wrapping_mul(29)
                    .wrapping_add(index * 11)
                    .wrapping_add(seed) as u8
            }));
        }
        bytes
    }
}
