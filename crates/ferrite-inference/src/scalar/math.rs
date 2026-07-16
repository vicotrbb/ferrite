use super::InferenceError;

/// Applies root-mean-square normalization and element-wise weights.
///
/// # Errors
///
/// Returns an error for empty or mismatched inputs, non-finite values, an
/// invalid epsilon, or a zero or non-finite normalization scale.
pub fn rms_norm(input: &[f32], weight: &[f32], epsilon: f32) -> Result<Vec<f32>, InferenceError> {
    if input.is_empty() {
        return Err(InferenceError::new("rms_norm input must not be empty"));
    }
    ensure_len("rms_norm weight", weight, input.len())?;
    if input.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("rms_norm input must be finite"));
    }
    if weight.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("rms_norm weight must be finite"));
    }
    if !epsilon.is_finite() || epsilon < 0.0 {
        return Err(InferenceError::new(
            "rms_norm epsilon must be finite and non-negative",
        ));
    }

    let mean_square = input.iter().map(|value| value * value).sum::<f32>() / input.len() as f32;
    let scale = (mean_square + epsilon).sqrt();
    if scale == 0.0 {
        return Err(InferenceError::new("rms_norm scale is zero"));
    }
    if !scale.is_finite() {
        return Err(InferenceError::new("rms_norm scale must be finite"));
    }

    let mut output = Vec::with_capacity(input.len());
    for (value, weight) in input.iter().zip(weight.iter()) {
        let normalized = value / scale * weight;
        if !normalized.is_finite() {
            return Err(InferenceError::new("rms_norm output must be finite"));
        }
        output.push(normalized);
    }
    Ok(output)
}

/// Returns the index of the first greatest finite value.
///
/// # Errors
///
/// Returns an error when `values` is empty or contains a non-finite value.
pub fn argmax(values: &[f32]) -> Result<usize, InferenceError> {
    if values.is_empty() {
        return Err(InferenceError::new("argmax input must not be empty"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("argmax input must be finite"));
    }

    let mut best_index = 0usize;
    let mut best_value = values[0];
    for (index, value) in values.iter().enumerate().skip(1) {
        if *value > best_value {
            best_value = *value;
            best_index = index;
        }
    }

    Ok(best_index)
}

#[cfg(test)]
fn swiglu(gate: &[f32], up: &[f32]) -> Result<Vec<f32>, InferenceError> {
    ensure_len("ffn up", up, gate.len())?;
    if gate.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu gate must be finite"));
    }
    if up.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu up must be finite"));
    }
    let mut output = Vec::with_capacity(gate.len());
    for (gate, up) in gate.iter().zip(up.iter()) {
        let value = silu(*gate) * *up;
        if !value.is_finite() {
            return Err(InferenceError::new("swiglu result must be finite"));
        }
        output.push(value);
    }
    Ok(output)
}

pub(super) fn swiglu_in_place(gate: &mut [f32], up: &[f32]) -> Result<(), InferenceError> {
    ensure_len("ffn up", up, gate.len())?;
    if gate.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu gate must be finite"));
    }
    if up.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu up must be finite"));
    }
    for (gate, up) in gate.iter_mut().zip(up) {
        let value = silu(*gate) * *up;
        if !value.is_finite() {
            return Err(InferenceError::new("swiglu result must be finite"));
        }
        *gate = value;
    }
    Ok(())
}

fn silu(value: f32) -> f32 {
    value / (1.0 + (-value).exp())
}

pub(super) fn dot(left: &[f32], right: &[f32]) -> Result<f32, InferenceError> {
    ensure_len("dot right", right, left.len())?;
    if left.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("dot left must be finite"));
    }
    if right.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("dot right must be finite"));
    }
    let mut sum = 0.0;
    for (left, right) in left.iter().zip(right.iter()) {
        let product = *left * *right;
        if !product.is_finite() {
            return Err(InferenceError::new("dot result must be finite"));
        }
        sum += product;
        if !sum.is_finite() {
            return Err(InferenceError::new("dot result must be finite"));
        }
    }
    Ok(sum)
}

pub(super) fn add_assign(left: &mut [f32], right: &[f32]) -> Result<(), InferenceError> {
    ensure_len("residual", right, left.len())?;
    if left.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("residual left must be finite"));
    }
    if right.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("residual right must be finite"));
    }
    for (left, right) in left.iter().zip(right.iter()) {
        let result = *left + *right;
        if !result.is_finite() {
            return Err(InferenceError::new("residual result must be finite"));
        }
    }
    for (left, right) in left.iter_mut().zip(right.iter()) {
        *left += *right;
    }
    Ok(())
}

pub(super) fn softmax_in_place(values: &mut [f32]) -> Result<(), InferenceError> {
    if values.is_empty() {
        return Err(InferenceError::new("softmax input must not be empty"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("softmax input must be finite"));
    }

    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for value in values.iter_mut() {
        let exp = (*value - max).exp();
        sum += exp;
        *value = exp;
    }
    if sum == 0.0 {
        return Err(InferenceError::new("softmax denominator is zero"));
    }

    for value in values {
        *value /= sum;
    }
    Ok(())
}

pub(super) fn ensure_len(
    name: &str,
    values: &[f32],
    expected: usize,
) -> Result<(), InferenceError> {
    if values.len() == expected {
        Ok(())
    } else {
        Err(InferenceError::new(format!(
            "{name} length {} does not match expected {expected}",
            values.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softmax_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match softmax_in_place(&mut [0.0, value]) {
                Ok(()) => return Err(InferenceError::new("non-finite softmax input should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("softmax input must be finite"));
        }
        Ok(())
    }

    #[test]
    fn softmax_in_place_preserves_reference_operation_order() -> Result<(), InferenceError> {
        let mut values = vec![-3.0f32, 0.25, 1.5, -0.75];
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut exponentials = values
            .iter()
            .map(|value| (*value - max).exp())
            .collect::<Vec<_>>();
        let sum = exponentials.iter().sum::<f32>();
        for value in &mut exponentials {
            *value /= sum;
        }

        softmax_in_place(&mut values)?;

        assert_eq!(values, exponentials);
        Ok(())
    }

    #[test]
    fn rms_norm_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match rms_norm(&[1.0, value], &[1.0, 1.0], 0.0) {
                Ok(_) => return Err(InferenceError::new("non-finite rms_norm input should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("rms_norm input must be finite"));
        }

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match rms_norm(&[1.0, 2.0], &[1.0, value], 0.0) {
                Ok(_) => {
                    return Err(InferenceError::new(
                        "non-finite rms_norm weight should fail",
                    ));
                }
                Err(error) => error,
            };

            assert!(error.to_string().contains("rms_norm weight must be finite"));
        }
        Ok(())
    }

    #[test]
    fn rms_norm_rejects_non_finite_scale() -> Result<(), InferenceError> {
        let error = match rms_norm(&[f32::MAX], &[1.0], 0.0) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "overflowing rms_norm scale should fail",
                ));
            }
            Err(error) => error,
        };

        assert!(error.to_string().contains("rms_norm scale must be finite"));
        Ok(())
    }

    #[test]
    fn rms_norm_rejects_non_finite_output() -> Result<(), InferenceError> {
        let error = match rms_norm(&[1.0, 0.0], &[f32::MAX, 1.0], 0.0) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "overflowing rms_norm output should fail",
                ));
            }
            Err(error) => error,
        };

        assert!(error.to_string().contains("rms_norm output must be finite"));
        Ok(())
    }

    #[test]
    fn swiglu_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match swiglu(&[1.0, value], &[1.0, 1.0]) {
                Ok(_) => return Err(InferenceError::new("non-finite swiglu gate should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("swiglu gate must be finite"));
        }

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match swiglu(&[1.0, 2.0], &[1.0, value]) {
                Ok(_) => return Err(InferenceError::new("non-finite swiglu up should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("swiglu up must be finite"));
        }
        Ok(())
    }

    #[test]
    fn swiglu_rejects_non_finite_results() -> Result<(), InferenceError> {
        let error = match swiglu(&[f32::MAX], &[2.0]) {
            Ok(_) => return Err(InferenceError::new("overflowing swiglu result should fail")),
            Err(error) => error,
        };

        assert!(error.to_string().contains("swiglu result must be finite"));
        Ok(())
    }

    #[test]
    fn swiglu_in_place_matches_allocating_path() -> Result<(), InferenceError> {
        let mut gate = vec![-3.0, -0.5, 0.0, 0.75, 4.0];
        let up = vec![0.25, -2.0, 3.0, 1.5, -0.125];
        let expected = swiglu(&gate, &up)?;

        swiglu_in_place(&mut gate, &up)?;

        assert_eq!(gate, expected);
        Ok(())
    }

    #[test]
    fn dot_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match dot(&[1.0, value], &[1.0, 1.0]) {
                Ok(_) => return Err(InferenceError::new("non-finite dot left should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("dot left must be finite"));
        }

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = match dot(&[1.0, 2.0], &[1.0, value]) {
                Ok(_) => return Err(InferenceError::new("non-finite dot right should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("dot right must be finite"));
        }
        Ok(())
    }

    #[test]
    fn dot_rejects_non_finite_results() -> Result<(), InferenceError> {
        let product_error = match dot(&[f32::MAX], &[2.0]) {
            Ok(_) => return Err(InferenceError::new("overflowing dot product should fail")),
            Err(error) => error,
        };

        assert!(
            product_error
                .to_string()
                .contains("dot result must be finite")
        );

        let sum_error = match dot(&[f32::MAX, f32::MAX], &[1.0, 1.0]) {
            Ok(_) => return Err(InferenceError::new("overflowing dot sum should fail")),
            Err(error) => error,
        };

        assert!(sum_error.to_string().contains("dot result must be finite"));
        Ok(())
    }

    #[test]
    fn add_assign_rejects_non_finite_values() -> Result<(), InferenceError> {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut left = [1.0, value];
            let error = match add_assign(&mut left, &[1.0, 1.0]) {
                Ok(_) => return Err(InferenceError::new("non-finite residual left should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("residual left must be finite"));
        }

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut left = [1.0, 2.0];
            let error = match add_assign(&mut left, &[1.0, value]) {
                Ok(_) => return Err(InferenceError::new("non-finite residual right should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("residual right must be finite"));
        }
        Ok(())
    }

    #[test]
    fn add_assign_rejects_non_finite_results() -> Result<(), InferenceError> {
        let mut left = [f32::MAX];
        let error = match add_assign(&mut left, &[f32::MAX]) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "overflowing residual result should fail",
                ));
            }
            Err(error) => error,
        };

        assert!(error.to_string().contains("residual result must be finite"));
        assert_eq!(left, [f32::MAX]);
        Ok(())
    }
}
