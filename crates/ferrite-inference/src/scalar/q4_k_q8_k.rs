use super::{
    q4_k::{q4_k_block_values, q4_k_storage_bytes, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    q8_k::BlockQ8K,
    InferenceError,
};

pub(in crate::scalar) fn q4_k_q8_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    validate_q4_k_q8_k_mul_vec(bytes, rows, cols, vector)?;
    let activation_blocks = BlockQ8K::quantize_blocks(vector)?;
    let blocks_per_row = cols / Q4_K_BLOCK_VALUES;
    let row_bytes = blocks_per_row
        .checked_mul(Q4_K_BLOCK_BYTES)
        .ok_or_else(|| InferenceError::new("Q4_K row byte length overflow"))?;

    bytes
        .chunks_exact(row_bytes)
        .map(|row| {
            row.chunks_exact(Q4_K_BLOCK_BYTES)
                .enumerate()
                .map(|(block_index, block)| {
                    q4_k_q8_k_block_dot(block, &activation_blocks[block_index])
                })
                .collect::<Result<Vec<_>, InferenceError>>()
                .map(|parts| parts.iter().sum())
        })
        .collect()
}

fn validate_q4_k_q8_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(), InferenceError> {
    if cols == 0 {
        return Err(InferenceError::new("Q4_K Q8_K columns must not be zero"));
    }
    if !cols.is_multiple_of(Q4_K_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q4_K Q8_K columns {cols} must be divisible by {Q4_K_BLOCK_VALUES}"
        )));
    }
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

pub(in crate::scalar) fn q4_k_q8_k_block_dot(
    block: &[u8],
    activation: &BlockQ8K,
) -> Result<f32, InferenceError> {
    let weights = q4_k_block_values(block)?;
    Ok(weights
        .iter()
        .zip(&activation.qs)
        .map(|(weight, quantized)| weight * activation.d * f32::from(*quantized))
        .sum())
}

#[cfg(test)]
mod tests {
    use super::{q4_k_q8_k_block_dot, q4_k_q8_k_mul_vec};
    use crate::scalar::{
        q4_k::{q4_k_block_values, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
        q8_k::BlockQ8K,
        InferenceError,
    };

    #[test]
    fn q4_k_q8_k_block_dot_matches_dequantized_q8_activation() -> Result<(), InferenceError> {
        let block = patterned_q4_k_block();
        let vector = patterned_activation();
        let activation = BlockQ8K::quantize(&vector)?;

        let actual = q4_k_q8_k_block_dot(&block, &activation)?;
        let expected = q4_k_block_values(&block)?
            .iter()
            .zip(&activation.qs)
            .map(|(weight, quantized)| weight * activation.d * f32::from(*quantized))
            .sum::<f32>();

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    #[test]
    fn q4_k_q8_k_mul_vec_accumulates_rows_and_blocks() -> Result<(), InferenceError> {
        let cols = Q4_K_BLOCK_VALUES * 2;
        let rows = 2;
        let vector = (0..cols)
            .map(|index| (index % 31) as f32 / 9.0 - 1.5)
            .collect::<Vec<_>>();
        let activation_blocks = BlockQ8K::quantize_blocks(&vector)?;
        let bytes = [
            patterned_q4_k_block_with_seed(0),
            patterned_q4_k_block_with_seed(1),
            patterned_q4_k_block_with_seed(2),
            patterned_q4_k_block_with_seed(3),
        ]
        .concat();

        let actual = q4_k_q8_k_mul_vec(&bytes, rows, cols, &vector)?;
        let expected = bytes
            .chunks_exact(Q4_K_BLOCK_BYTES * 2)
            .map(|row| {
                row.chunks_exact(Q4_K_BLOCK_BYTES)
                    .enumerate()
                    .map(|(block_index, block)| {
                        expected_block_dot(block, &activation_blocks[block_index])
                    })
                    .collect::<Result<Vec<_>, InferenceError>>()
                    .map(|parts| parts.iter().sum::<f32>())
            })
            .collect::<Result<Vec<_>, InferenceError>>()?;

        assert_eq!(actual.len(), rows);
        for (actual, expected) in actual.iter().zip(&expected) {
            assert!(
                (actual - expected).abs() < 0.001,
                "actual={actual} expected={expected}"
            );
        }
        Ok(())
    }

    #[test]
    fn q4_k_q8_k_mul_vec_rejects_zero_columns() -> Result<(), InferenceError> {
        let err = match q4_k_q8_k_mul_vec(&[], 1, 0, &[]) {
            Ok(_) => return Err(InferenceError::new("zero columns must fail")),
            Err(err) => err,
        };

        assert_eq!(err.to_string(), "Q4_K Q8_K columns must not be zero");
        Ok(())
    }

    #[test]
    fn q4_k_q8_k_mul_vec_rejects_partial_block_columns() -> Result<(), InferenceError> {
        let bytes = patterned_q4_k_block();
        let vector = vec![1.0; Q4_K_BLOCK_VALUES / 2];

        let err = match q4_k_q8_k_mul_vec(&bytes, 2, Q4_K_BLOCK_VALUES / 2, &vector) {
            Ok(_) => return Err(InferenceError::new("partial-block columns must fail")),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "Q4_K Q8_K columns 128 must be divisible by 256"
        );
        Ok(())
    }

    fn patterned_q4_k_block() -> Vec<u8> {
        patterned_q4_k_block_with_seed(0)
    }

    fn patterned_q4_k_block_with_seed(seed: u8) -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend((1..=12).map(|value| value + seed));
        for index in 0..128 {
            let low = (index as u8).wrapping_add(seed) & 0x0f;
            let high = 15 - low;
            block.push(low | (high << 4));
        }
        block
    }

    fn patterned_activation() -> [f32; Q4_K_BLOCK_VALUES] {
        let mut values = [0.0; Q4_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let wave = (index % 23) as f32 - 11.0;
            *value = wave / 5.0;
        }
        values
    }

    fn expected_block_dot(block: &[u8], activation: &BlockQ8K) -> Result<f32, InferenceError> {
        Ok(q4_k_block_values(block)?
            .iter()
            .zip(&activation.qs)
            .map(|(weight, quantized)| weight * activation.d * f32::from(*quantized))
            .sum())
    }
}
