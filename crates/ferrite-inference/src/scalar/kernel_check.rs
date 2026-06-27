use super::InferenceError;

pub(super) fn ensure_within_relative_error(
    actual: &[f32],
    expected: &[f32],
    tolerance: f32,
) -> Result<(), InferenceError> {
    if tolerance < 0.0 {
        return Err(InferenceError::new(format!(
            "relative error tolerance {tolerance} must be non-negative"
        )));
    }
    if actual.len() != expected.len() {
        return Err(InferenceError::new(format!(
            "actual output length {} does not match reference length {}",
            actual.len(),
            expected.len()
        )));
    }

    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        let denominator = expected.abs().max(1.0);
        let relative_error = (actual - expected).abs() / denominator;
        if relative_error > tolerance {
            return Err(InferenceError::new(format!(
                "matvec output {index} relative error {relative_error} exceeds tolerance {tolerance}: actual={actual}, reference={expected}"
            )));
        }
    }

    Ok(())
}
