use super::{float::f16_bits_to_f32, InferenceError, ScalarExecutionOptions};

pub(super) const Q5_K_BLOCK_VALUES: usize = 256;
pub(super) const Q5_K_BLOCK_BYTES: usize = 176;

/// Multiplies a GGML `Q5_K` matrix by one activation vector.
///
/// `Q5_K` currently uses the architecture-neutral reference path. The execution
/// options parameter keeps it behind the same provider boundary as optimized
/// matrix formats without pretending that an ISA-specific `Q5_K` kernel exists.
pub(super) fn q5_k_mul_vec_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    let _ = options;
    validate_q5_k_mul_vec(bytes, rows, cols, vector)?;

    let mut output = vec![0.0; rows];
    for (block_index, block) in bytes.chunks_exact(Q5_K_BLOCK_BYTES).enumerate() {
        let value_offset = block_index
            .checked_mul(Q5_K_BLOCK_VALUES)
            .ok_or_else(|| InferenceError::new("Q5_K block value offset overflow"))?;
        for (local_offset, value) in q5_k_block_values(block)?.iter().enumerate() {
            let matrix_index = value_offset
                .checked_add(local_offset)
                .ok_or_else(|| InferenceError::new("Q5_K matrix index overflow"))?;
            let row = matrix_index / cols;
            let col = matrix_index % cols;
            let target = output
                .get_mut(row)
                .ok_or_else(|| InferenceError::new("Q5_K matrix row index out of bounds"))?;
            let activation = vector
                .get(col)
                .ok_or_else(|| InferenceError::new("Q5_K matrix column index out of bounds"))?;
            *target += value * activation;
        }
    }
    Ok(output)
}

pub(super) fn q5_k_mul_vec_batch_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
    options: ScalarExecutionOptions,
) -> Result<Vec<Vec<f32>>, InferenceError> {
    vectors
        .iter()
        .map(|vector| q5_k_mul_vec_with_options(bytes, rows, cols, vector, options))
        .collect()
}

fn validate_q5_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(), InferenceError> {
    if cols == 0 {
        return Err(InferenceError::new(
            "Q5_K matrix columns must be greater than zero",
        ));
    }
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q5_K matrix value count overflow"))?;
    let expected = q5_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q5_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    Ok(())
}

pub(super) fn q5_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    if !value_count.is_multiple_of(Q5_K_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q5_K value count {value_count} must be divisible by {Q5_K_BLOCK_VALUES}"
        )));
    }
    value_count
        .checked_div(Q5_K_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q5_K_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q5_K byte length overflow"))
}

pub(super) fn validate_q5_k_finite_scales(bytes: &[u8]) -> Result<(), InferenceError> {
    if !bytes.len().is_multiple_of(Q5_K_BLOCK_BYTES) {
        return Err(InferenceError::new(format!(
            "Q5_K byte length {} must be divisible by {Q5_K_BLOCK_BYTES}",
            bytes.len()
        )));
    }
    for block in bytes.chunks_exact(Q5_K_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let minimum_scale = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
        if !scale.is_finite() || !minimum_scale.is_finite() {
            return Err(InferenceError::new(
                "Q5_K matrix scale values must be finite",
            ));
        }
    }
    Ok(())
}

pub(super) fn decode_q5_k_values(
    bytes: &[u8],
    value_count: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected = q5_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q5_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    let mut values = Vec::with_capacity(value_count);
    for block in bytes.chunks_exact(Q5_K_BLOCK_BYTES) {
        values.extend(q5_k_block_values(block)?);
    }
    Ok(values)
}

pub(super) fn q5_k_block_values(block: &[u8]) -> Result<[f32; Q5_K_BLOCK_VALUES], InferenceError> {
    if block.len() != Q5_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q5_K block byte length {} does not match {Q5_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let minimum_scale = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    if !scale.is_finite() || !minimum_scale.is_finite() {
        return Err(InferenceError::new(
            "Q5_K matrix scale values must be finite",
        ));
    }
    let scales = &block[4..16];
    let high_bits = &block[16..48];
    let low_quants = &block[48..176];
    let mut values = [0.0; Q5_K_BLOCK_VALUES];

    for group in 0..4 {
        let quant_chunk = &low_quants[group * 32..(group + 1) * 32];
        let (low_scale, low_minimum) = q5_k_scale_min(group * 2, scales);
        let (high_scale, high_minimum) = q5_k_scale_min(group * 2 + 1, scales);
        let low_multiplier = scale * f32::from(low_scale);
        let high_multiplier = scale * f32::from(high_scale);
        let low_minimum = minimum_scale * f32::from(low_minimum);
        let high_minimum = minimum_scale * f32::from(high_minimum);
        let low_high_bit = 1u8 << (group * 2);
        let high_high_bit = 2u8 << (group * 2);
        let value_base = group * 64;

        for index in 0..32 {
            let low_quant = (quant_chunk[index] & 0x0f)
                + if high_bits[index] & low_high_bit != 0 {
                    16
                } else {
                    0
                };
            let high_quant = (quant_chunk[index] >> 4)
                + if high_bits[index] & high_high_bit != 0 {
                    16
                } else {
                    0
                };
            values[value_base + index] = low_multiplier * f32::from(low_quant) - low_minimum;
            values[value_base + 32 + index] =
                high_multiplier * f32::from(high_quant) - high_minimum;
        }
    }
    Ok(values)
}

fn q5_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
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
    use super::*;
    use crate::scalar::{Matrix, MatrixStorageKind};

    fn patterned_block() -> [u8; Q5_K_BLOCK_BYTES] {
        let mut block = [0u8; Q5_K_BLOCK_BYTES];
        block[0..2].copy_from_slice(&0x3c00u16.to_le_bytes());
        block[2..4].copy_from_slice(&0x3c00u16.to_le_bytes());
        block[4..16].copy_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        for (index, quant) in block[48..].iter_mut().enumerate() {
            *quant = ((index % 16) as u8) | (((15 - index % 16) as u8) << 4);
        }
        for (index, high) in block[16..48].iter_mut().enumerate() {
            *high = if index.is_multiple_of(2) {
                0b0101_0101
            } else {
                0b1010_1010
            };
        }
        block
    }

    #[test]
    fn decoder_matches_ggml_q5_k_bit_layout() -> Result<(), InferenceError> {
        let values = q5_k_block_values(&patterned_block())?;
        assert_eq!(values[0], 16.0);
        assert_eq!(values[1], 1.0);
        assert_eq!(values[32], 15.0);
        assert_eq!(values[33], 30.0);
        assert_eq!(values[64], 16.0);
        assert_eq!(values[96], 15.0);
        assert_eq!(values[128], 16.0);
        assert_eq!(values[160], 15.0);
        assert_eq!(values[192], 16.0);
        assert_eq!(values[224], 15.0);
        Ok(())
    }

    #[test]
    fn matvec_preserves_rows_and_columns() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&patterned_block());
        bytes.extend_from_slice(&patterned_block());
        let vector = vec![1.0; Q5_K_BLOCK_VALUES];
        let output = q5_k_mul_vec_with_options(
            &bytes,
            2,
            Q5_K_BLOCK_VALUES,
            &vector,
            ScalarExecutionOptions::default(),
        )?;
        let expected: f32 = q5_k_block_values(&patterned_block())?.iter().sum();
        assert_eq!(output, vec![expected, expected]);
        Ok(())
    }

    #[test]
    fn row_range_preserves_q5_k_storage_and_values() -> Result<(), InferenceError> {
        let first = patterned_block();
        let mut second = patterned_block();
        second[48] = 0x22;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        let matrix = Matrix::from_q5_k_row_major_bytes(2, Q5_K_BLOCK_VALUES, bytes)?;

        let selected = matrix.row_range(1..2)?;
        assert_eq!(selected.storage_kind(), MatrixStorageKind::Q5K);
        assert_eq!(selected.rows(), 1);
        assert_eq!(selected.row_values(0)?, q5_k_block_values(&second)?);
        Ok(())
    }

    #[test]
    fn rejects_non_finite_scales() {
        let mut block = patterned_block();
        block[0..2].copy_from_slice(&0x7c00u16.to_le_bytes());
        assert!(q5_k_block_values(&block).is_err());
        assert!(validate_q5_k_finite_scales(&block).is_err());
    }
}
