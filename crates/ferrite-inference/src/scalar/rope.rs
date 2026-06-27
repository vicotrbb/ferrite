use super::InferenceError;

pub fn apply_rope(
    values: &[f32],
    position: usize,
    rope_dimension_count: usize,
    rope_freq_base: f32,
) -> Result<Vec<f32>, InferenceError> {
    if rope_dimension_count == 0 {
        return Ok(values.to_vec());
    }
    if rope_dimension_count > values.len() {
        return Err(InferenceError::new(format!(
            "rope dimension count {rope_dimension_count} exceeds vector length {}",
            values.len()
        )));
    }
    if !rope_dimension_count.is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "rope dimension count {rope_dimension_count} must be even"
        )));
    }
    if rope_freq_base <= 0.0 {
        return Err(InferenceError::new(format!(
            "rope frequency base {rope_freq_base} must be positive"
        )));
    }

    let mut output = values.to_vec();
    for pair_start in (0..rope_dimension_count).step_by(2) {
        let exponent = pair_start as f32 / rope_dimension_count as f32;
        let theta = position as f32 / rope_freq_base.powf(exponent);
        let cos = theta.cos();
        let sin = theta.sin();
        let even = values[pair_start];
        let odd = values[pair_start + 1];
        output[pair_start] = even * cos - odd * sin;
        output[pair_start + 1] = even * sin + odd * cos;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 0.0001,
            "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
        );
    }

    #[test]
    fn rope_rotates_full_tier1_head_dimensions() -> Result<(), InferenceError> {
        for head_dim in [64, 128] {
            let mut values = vec![0.0; head_dim];
            values[0] = 1.0;
            values[head_dim - 2] = 1.0;

            let position = 3;
            let base = 10_000.0;
            let rotated = apply_rope(&values, position, head_dim, base)?;

            let first_theta = position as f32;
            assert_close(rotated[0], first_theta.cos());
            assert_close(rotated[1], first_theta.sin());

            let last_pair_start = head_dim - 2;
            let last_theta = position as f32 / base.powf(last_pair_start as f32 / head_dim as f32);
            assert_close(rotated[last_pair_start], last_theta.cos());
            assert_close(rotated[last_pair_start + 1], last_theta.sin());
            assert_eq!(rotated.len(), head_dim);
        }
        Ok(())
    }
}
