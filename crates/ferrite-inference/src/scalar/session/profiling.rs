use crate::scalar::{
    profile::{ScalarMatVecComparison, ScalarProfileEvent},
    InferenceError, Matrix, MatrixStorageKind, ScalarExecutionOptions,
};
use std::time::{Duration, Instant};

pub(super) fn profiled_layer_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    layer_index: usize,
    role: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
    comparison_events: Option<&mut Vec<ScalarMatVecComparison>>,
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    if profile_events.is_none() {
        return matrix.mul_vec_with_options(vector, options);
    }
    profiled_mul_vec(
        matrix,
        vector,
        &format!("layer.{layer_index}.{role}"),
        profile_events,
        comparison_events,
        options,
    )
}

pub(super) fn profiled_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
    comparison_events: Option<&mut Vec<ScalarMatVecComparison>>,
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    let Some(events) = profile_events else {
        return matrix.mul_vec_with_options(vector, options);
    };
    let started = Instant::now();
    let output = matrix.mul_vec_with_options(vector, options)?;
    let elapsed = started.elapsed();
    events.push(ScalarProfileEvent::new(
        label,
        nonzero_duration(elapsed),
        matrix,
    ));
    compare_q8_k_activation_matvec(matrix, vector, label, comparison_events, options)?;
    Ok(output)
}

pub(super) fn profiled_argmax_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
    comparison_events: Option<&mut Vec<ScalarMatVecComparison>>,
    options: ScalarExecutionOptions,
) -> Result<usize, InferenceError> {
    let Some(events) = profile_events else {
        return matrix.argmax_mul_vec_with_options(vector, options);
    };
    let started = Instant::now();
    let token_id = matrix.argmax_mul_vec_with_options(vector, options)?;
    let elapsed = started.elapsed();
    events.push(ScalarProfileEvent::new(
        label,
        nonzero_duration(elapsed),
        matrix,
    ));
    if let Some(events) = comparison_events {
        if options.compare_q8_k_activation_matvec() && is_q8_k_comparable(matrix.storage_kind()) {
            let reference = matrix.mul_vec(vector)?;
            let candidate =
                matrix.mul_vec_with_options(vector, options.q8_k_activation_matvec_candidate())?;
            events.push(ScalarMatVecComparison::new(
                label, matrix, &reference, &candidate,
            )?);
        }
    }
    Ok(token_id)
}

fn compare_q8_k_activation_matvec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    comparison_events: Option<&mut Vec<ScalarMatVecComparison>>,
    options: ScalarExecutionOptions,
) -> Result<(), InferenceError> {
    let Some(events) = comparison_events else {
        return Ok(());
    };
    if !options.compare_q8_k_activation_matvec() || !is_q8_k_comparable(matrix.storage_kind()) {
        return Ok(());
    }

    let reference = matrix.mul_vec(vector)?;
    let candidate =
        matrix.mul_vec_with_options(vector, options.q8_k_activation_matvec_candidate())?;
    events.push(ScalarMatVecComparison::new(
        label, matrix, &reference, &candidate,
    )?);
    Ok(())
}

fn is_q8_k_comparable(storage_kind: MatrixStorageKind) -> bool {
    matches!(
        storage_kind,
        MatrixStorageKind::Q4K
            | MatrixStorageKind::Q5_0
            | MatrixStorageKind::Q6K
            | MatrixStorageKind::Q8_0
    )
}

fn nonzero_duration(elapsed: Duration) -> Duration {
    if elapsed.is_zero() {
        Duration::from_nanos(1)
    } else {
        elapsed
    }
}
