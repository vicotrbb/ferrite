use super::{q4_k::q4_k_block_values, q8_k::BlockQ8K, InferenceError};

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
    use super::q4_k_q8_k_block_dot;
    use crate::scalar::{
        q4_k::{q4_k_block_values, Q4_K_BLOCK_VALUES},
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

    fn patterned_q4_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0x3800u16.to_le_bytes());
        block.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        for index in 0..128 {
            let low = index as u8 & 0x0f;
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
}
