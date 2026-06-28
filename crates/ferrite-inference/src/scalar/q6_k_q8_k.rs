use super::{q6_k::q6_k_block_values, q8_k::BlockQ8K, InferenceError};

pub(in crate::scalar) fn q6_k_q8_k_block_dot(
    block: &[u8],
    activation: &BlockQ8K,
) -> Result<f32, InferenceError> {
    let weights = q6_k_block_values(block)?;
    Ok(weights
        .iter()
        .zip(&activation.qs)
        .map(|(weight, quantized)| weight * activation.d * f32::from(*quantized))
        .sum())
}

#[cfg(test)]
mod tests {
    use super::q6_k_q8_k_block_dot;
    use crate::scalar::{
        q6_k::{q6_k_block_values, Q6_K_BLOCK_VALUES},
        q8_k::BlockQ8K,
        InferenceError,
    };

    #[test]
    fn q6_k_q8_k_block_dot_matches_dequantized_q8_activation() -> Result<(), InferenceError> {
        let block = patterned_q6_k_block();
        let vector = patterned_activation();
        let activation = BlockQ8K::quantize(&vector)?;

        let actual = q6_k_q8_k_block_dot(&block, &activation)?;
        let expected = q6_k_block_values(&block)?
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

    fn patterned_q6_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend((0..128).map(|index| (index * 37) as u8));
        block.extend((0..64).map(|index| (index * 19) as u8));
        block.extend(
            [-3i8, 2, -5, 4, -7, 6, -9, 8, 9, -8, 7, -6, 5, -4, 3, -2].map(|value| value as u8),
        );
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }

    fn patterned_activation() -> [f32; Q6_K_BLOCK_VALUES] {
        let mut values = [0.0; Q6_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let wave = (index % 29) as f32 - 14.0;
            *value = wave / 7.0;
        }
        values
    }
}
