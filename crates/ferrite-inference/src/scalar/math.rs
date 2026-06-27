use super::InferenceError;

pub fn rms_norm(input: &[f32], weight: &[f32], epsilon: f32) -> Result<Vec<f32>, InferenceError> {
    if input.is_empty() {
        return Err(InferenceError::new("rms_norm input must not be empty"));
    }
    ensure_len("rms_norm weight", weight, input.len())?;

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

pub(super) fn swiglu(gate: &[f32], up: &[f32]) -> Result<Vec<f32>, InferenceError> {
    ensure_len("ffn up", up, gate.len())?;
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
    Ok(left.iter().zip(right.iter()).map(|(a, b)| a * b).sum())
}

pub(super) fn add_assign(left: &mut [f32], right: &[f32]) -> Result<(), InferenceError> {
    ensure_len("residual", right, left.len())?;
    for (left, right) in left.iter_mut().zip(right.iter()) {
        *left += right;
    }
    Ok(())
}

pub(super) fn softmax(values: &[f32]) -> Result<Vec<f32>, InferenceError> {
    if values.is_empty() {
        return Err(InferenceError::new("softmax input must not be empty"));
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
