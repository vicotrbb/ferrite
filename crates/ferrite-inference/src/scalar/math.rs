use super::InferenceError;

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

    Ok(input
        .iter()
        .zip(weight.iter())
        .map(|(value, weight)| value / scale * weight)
        .collect())
}

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

pub(super) fn swiglu(gate: &[f32], up: &[f32]) -> Result<Vec<f32>, InferenceError> {
    ensure_len("ffn up", up, gate.len())?;
    if gate.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu gate must be finite"));
    }
    if up.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("swiglu up must be finite"));
    }
    Ok(gate
        .iter()
        .zip(up.iter())
        .map(|(gate, up)| silu(*gate) * up)
        .collect())
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
    Ok(left.iter().zip(right.iter()).map(|(a, b)| a * b).sum())
}

pub(super) fn add_assign(left: &mut [f32], right: &[f32]) -> Result<(), InferenceError> {
    ensure_len("residual", right, left.len())?;
    if left.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("residual left must be finite"));
    }
    if right.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("residual right must be finite"));
    }
    for (left, right) in left.iter_mut().zip(right.iter()) {
        *left += right;
    }
    Ok(())
}

pub(super) fn softmax(values: &[f32]) -> Result<Vec<f32>, InferenceError> {
    if values.is_empty() {
        return Err(InferenceError::new("softmax input must not be empty"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("softmax input must be finite"));
    }

    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut exp_values = Vec::with_capacity(values.len());
    let mut sum = 0.0;
    for value in values {
        let exp = (*value - max).exp();
        sum += exp;
        exp_values.push(exp);
    }
    if sum == 0.0 {
        return Err(InferenceError::new("softmax denominator is zero"));
    }

    Ok(exp_values.into_iter().map(|value| value / sum).collect())
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
            let error = match softmax(&[0.0, value]) {
                Ok(_) => return Err(InferenceError::new("non-finite softmax input should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("softmax input must be finite"));
        }
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
                    ))
                }
                Err(error) => error,
            };

            assert!(error.to_string().contains("rms_norm weight must be finite"));
        }
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
}
