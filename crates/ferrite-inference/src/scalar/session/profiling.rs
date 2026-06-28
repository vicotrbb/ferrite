use crate::scalar::{profile::ScalarProfileEvent, InferenceError, Matrix};
use std::time::{Duration, Instant};

pub(super) fn profiled_layer_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    layer_index: usize,
    role: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
) -> Result<Vec<f32>, InferenceError> {
    if profile_events.is_none() {
        return matrix.mul_vec(vector);
    }
    profiled_mul_vec(
        matrix,
        vector,
        &format!("layer.{layer_index}.{role}"),
        profile_events,
    )
}

pub(super) fn profiled_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
) -> Result<Vec<f32>, InferenceError> {
    let Some(events) = profile_events else {
        return matrix.mul_vec(vector);
    };
    let started = Instant::now();
    let output = matrix.mul_vec(vector)?;
    let elapsed = started.elapsed();
    events.push(ScalarProfileEvent::new(
        label,
        nonzero_duration(elapsed),
        matrix,
    ));
    Ok(output)
}

pub(super) fn profiled_argmax_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
) -> Result<usize, InferenceError> {
    let Some(events) = profile_events else {
        return matrix.argmax_mul_vec(vector);
    };
    let started = Instant::now();
    let token_id = matrix.argmax_mul_vec(vector)?;
    let elapsed = started.elapsed();
    events.push(ScalarProfileEvent::new(
        label,
        nonzero_duration(elapsed),
        matrix,
    ));
    Ok(token_id)
}

fn nonzero_duration(elapsed: Duration) -> Duration {
    if elapsed.is_zero() {
        Duration::from_nanos(1)
    } else {
        elapsed
    }
}
