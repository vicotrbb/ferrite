use super::InferenceError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The coordinate-pairing convention used by rotary position encoding.
pub enum RopeLayout {
    /// Pair consecutive coordinates, as used by Llama.
    AdjacentPairs,
    /// Pair coordinates from the first and second halves, as used by Qwen2.
    SplitHalf,
}

/// Applies adjacent-pair rotary position encoding to one attention head.
///
/// Dimensions beyond `rope_dimension_count` are copied unchanged.
///
/// # Errors
///
/// Returns an error for non-finite input, an odd or out-of-range rotary
/// dimension count, or a non-positive or non-finite frequency base.
pub fn apply_rope(
    values: &[f32],
    position: usize,
    rope_dimension_count: usize,
    rope_freq_base: f32,
) -> Result<Vec<f32>, InferenceError> {
    let mut output = values.to_vec();
    apply_rope_with_layout_in_place(
        &mut output,
        position,
        rope_dimension_count,
        rope_freq_base,
        RopeLayout::AdjacentPairs,
    )?;
    Ok(output)
}

#[cfg(test)]
pub(super) fn apply_rope_with_layout(
    values: &[f32],
    position: usize,
    rope_dimension_count: usize,
    rope_freq_base: f32,
    layout: RopeLayout,
) -> Result<Vec<f32>, InferenceError> {
    let mut output = values.to_vec();
    apply_rope_with_layout_in_place(
        &mut output,
        position,
        rope_dimension_count,
        rope_freq_base,
        layout,
    )?;
    Ok(output)
}

pub(super) fn apply_rope_with_layout_in_place(
    values: &mut [f32],
    position: usize,
    rope_dimension_count: usize,
    rope_freq_base: f32,
    layout: RopeLayout,
) -> Result<(), InferenceError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("rope input must be finite"));
    }
    if rope_dimension_count == 0 {
        return Ok(());
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
    if !rope_freq_base.is_finite() {
        return Err(InferenceError::new("rope frequency base must be finite"));
    }
    if rope_freq_base <= 0.0 {
        return Err(InferenceError::new(format!(
            "rope frequency base {rope_freq_base} must be positive"
        )));
    }

    match layout {
        RopeLayout::AdjacentPairs => {
            for pair_start in (0..rope_dimension_count).step_by(2) {
                let (cos, sin) =
                    rotation(pair_start, position, rope_dimension_count, rope_freq_base);
                let left = values[pair_start];
                let right = values[pair_start + 1];
                values[pair_start] = left * cos - right * sin;
                values[pair_start + 1] = left * sin + right * cos;
            }
        }
        RopeLayout::SplitHalf => {
            let half = rope_dimension_count / 2;
            for index in 0..half {
                let (cos, sin) =
                    rotation(index * 2, position, rope_dimension_count, rope_freq_base);
                let left = values[index];
                let right = values[index + half];
                values[index] = left * cos - right * sin;
                values[index + half] = left * sin + right * cos;
            }
        }
    }

    Ok(())
}

fn rotation(
    frequency_index: usize,
    position: usize,
    rope_dimension_count: usize,
    rope_freq_base: f32,
) -> (f32, f32) {
    let exponent = frequency_index as f32 / rope_dimension_count as f32;
    let theta = position as f32 / rope_freq_base.powf(exponent);
    (theta.cos(), theta.sin())
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

    fn reference_apply_rope_with_layout(
        values: &[f32],
        position: usize,
        rope_dimension_count: usize,
        rope_freq_base: f32,
        layout: RopeLayout,
    ) -> Vec<f32> {
        let mut output = values.to_vec();
        match layout {
            RopeLayout::AdjacentPairs => {
                for pair_start in (0..rope_dimension_count).step_by(2) {
                    let (cos, sin) =
                        rotation(pair_start, position, rope_dimension_count, rope_freq_base);
                    let left = values[pair_start];
                    let right = values[pair_start + 1];
                    output[pair_start] = left * cos - right * sin;
                    output[pair_start + 1] = left * sin + right * cos;
                }
            }
            RopeLayout::SplitHalf => {
                let half = rope_dimension_count / 2;
                for index in 0..half {
                    let (cos, sin) =
                        rotation(index * 2, position, rope_dimension_count, rope_freq_base);
                    let left = values[index];
                    let right = values[index + half];
                    output[index] = left * cos - right * sin;
                    output[index + half] = left * sin + right * cos;
                }
            }
        }
        output
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

    #[test]
    fn rope_split_half_layout_rotates_values_offset_by_half_dimension() -> Result<(), InferenceError>
    {
        let rotated =
            apply_rope_with_layout(&[1.0, 0.0, 0.0, 1.0], 1, 4, 1.0, RopeLayout::SplitHalf)?;

        assert_close(rotated[0], 1.0_f32.cos());
        assert_close(rotated[2], 1.0_f32.sin());
        assert_close(rotated[1], -1.0_f32.sin());
        assert_close(rotated[3], 1.0_f32.cos());
        Ok(())
    }

    #[test]
    fn in_place_rope_matches_allocating_api_without_replacing_storage() -> Result<(), InferenceError>
    {
        for layout in [RopeLayout::AdjacentPairs, RopeLayout::SplitHalf] {
            for head_dimension in [64, 128] {
                let original = (0..head_dimension)
                    .map(|index| (index as f32 - 31.0) / 7.0)
                    .collect::<Vec<_>>();
                for rope_dimension_count in [0, head_dimension / 2, head_dimension] {
                    for position in [0, 1, 7, 257] {
                        let expected = reference_apply_rope_with_layout(
                            &original,
                            position,
                            rope_dimension_count,
                            10_000.0,
                            layout,
                        );
                        let mut actual = original.clone();
                        let pointer = actual.as_ptr();
                        let capacity = actual.capacity();

                        apply_rope_with_layout_in_place(
                            &mut actual,
                            position,
                            rope_dimension_count,
                            10_000.0,
                            layout,
                        )?;

                        assert_eq!(actual, expected);
                        assert_eq!(actual.as_ptr(), pointer);
                        assert_eq!(actual.capacity(), capacity);
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn rope_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match apply_rope(&[1.0, value], 1, 2, 10_000.0) {
                Ok(_) => return Err(InferenceError::new("non-finite rope input should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("rope input must be finite"));
        }
        Ok(())
    }
}
