use std::time::Duration;

use super::{matrix::MatrixStorageKind, InferenceError, Matrix, NextToken};

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarProfileEvent {
    label: String,
    elapsed: Duration,
    storage_kind: MatrixStorageKind,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarMatVecComparison {
    label: String,
    storage_kind: MatrixStorageKind,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
    max_abs_diff: f32,
    max_relative_diff: f32,
    reference_argmax_index: usize,
    candidate_argmax_index: usize,
}

impl ScalarMatVecComparison {
    pub(super) fn new(
        label: impl Into<String>,
        matrix: &Matrix,
        reference: &[f32],
        candidate: &[f32],
    ) -> Result<Self, InferenceError> {
        if reference.len() != candidate.len() {
            return Err(InferenceError::new(format!(
                "matvec comparison length {} does not match {}",
                reference.len(),
                candidate.len()
            )));
        }
        if reference.is_empty() {
            return Err(InferenceError::new(
                "matvec comparison output must not be empty",
            ));
        }

        let mut max_abs_diff = 0.0f32;
        let mut max_relative_diff = 0.0f32;
        let label = label.into();
        for (index, (reference, candidate)) in reference.iter().zip(candidate).enumerate() {
            if !reference.is_finite() {
                return Err(InferenceError::new(format!(
                    "matvec comparison {label} reference value {index} is not finite"
                )));
            }
            if !candidate.is_finite() {
                return Err(InferenceError::new(format!(
                    "matvec comparison {label} candidate value {index} is not finite"
                )));
            }
            let abs_diff = (reference - candidate).abs();
            max_abs_diff = max_abs_diff.max(abs_diff);
            let denominator = reference.abs().max(1.0e-6);
            max_relative_diff = max_relative_diff.max(abs_diff / denominator);
        }
        let reference_argmax_index = argmax_index(reference)
            .ok_or_else(|| InferenceError::new("matvec comparison output must not be empty"))?;
        let candidate_argmax_index = argmax_index(candidate)
            .ok_or_else(|| InferenceError::new("matvec comparison output must not be empty"))?;

        Ok(Self {
            label,
            storage_kind: matrix.storage_kind(),
            rows: matrix.rows(),
            cols: matrix.cols(),
            storage_bytes: matrix.storage_bytes(),
            max_abs_diff,
            max_relative_diff,
            reference_argmax_index,
            candidate_argmax_index,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn storage_kind(&self) -> MatrixStorageKind {
        self.storage_kind
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn storage_bytes(&self) -> u128 {
        self.storage_bytes
    }

    pub fn max_abs_diff(&self) -> f32 {
        self.max_abs_diff
    }

    pub fn max_relative_diff(&self) -> f32 {
        self.max_relative_diff
    }

    pub fn reference_argmax_index(&self) -> usize {
        self.reference_argmax_index
    }

    pub fn candidate_argmax_index(&self) -> usize {
        self.candidate_argmax_index
    }
}

fn argmax_index(values: &[f32]) -> Option<usize> {
    values
        .iter()
        .enumerate()
        .max_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right)
                .then_with(|| right_index.cmp(left_index))
        })
        .map(|(index, _)| index)
}

impl ScalarProfileEvent {
    pub(super) fn new(label: impl Into<String>, elapsed: Duration, matrix: &Matrix) -> Self {
        Self {
            label: label.into(),
            elapsed,
            storage_kind: matrix.storage_kind(),
            rows: matrix.rows(),
            cols: matrix.cols(),
            storage_bytes: matrix.storage_bytes(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn storage_kind(&self) -> MatrixStorageKind {
        self.storage_kind
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn storage_bytes(&self) -> u128 {
        self.storage_bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfiledNextToken {
    pub next_token: NextToken,
    pub events: Vec<ScalarProfileEvent>,
    pub comparisons: Vec<ScalarMatVecComparison>,
}

impl ProfiledNextToken {
    pub fn total_elapsed(&self) -> Duration {
        self.events
            .iter()
            .map(ScalarProfileEvent::elapsed)
            .sum::<Duration>()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfiledTokenId {
    pub token_id: usize,
    pub events: Vec<ScalarProfileEvent>,
    pub comparisons: Vec<ScalarMatVecComparison>,
}

impl ProfiledTokenId {
    pub fn total_elapsed(&self) -> Duration {
        self.events
            .iter()
            .map(ScalarProfileEvent::elapsed)
            .sum::<Duration>()
    }
}

#[cfg(test)]
mod tests {
    use super::ScalarMatVecComparison;
    use crate::scalar::{InferenceError, Matrix};

    #[test]
    fn matvec_comparison_rejects_non_finite_values() -> Result<(), InferenceError> {
        let matrix = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;

        let err =
            match ScalarMatVecComparison::new("q8_k_probe", &matrix, &[1.0, f32::NAN], &[1.0, 1.0])
            {
                Ok(_) => return Err(InferenceError::new("non-finite comparison must fail")),
                Err(err) => err,
            };

        assert_eq!(
            err.to_string(),
            "matvec comparison q8_k_probe reference value 1 is not finite"
        );
        Ok(())
    }

    #[test]
    fn matvec_comparison_rejects_non_finite_candidate_values() -> Result<(), InferenceError> {
        let matrix = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;

        let err =
            match ScalarMatVecComparison::new("q8_k_probe", &matrix, &[1.0, 1.0], &[1.0, f32::NAN])
            {
                Ok(_) => {
                    return Err(InferenceError::new(
                        "non-finite candidate comparison must fail",
                    ));
                }
                Err(err) => err,
            };

        assert_eq!(
            err.to_string(),
            "matvec comparison q8_k_probe candidate value 1 is not finite"
        );
        Ok(())
    }

    #[test]
    fn matvec_comparison_records_argmax_indexes() -> Result<(), InferenceError> {
        let matrix = Matrix::from_row_major(3, 2, vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0])?;
        let comparison = ScalarMatVecComparison::new(
            "q8_k_probe",
            &matrix,
            &[0.25, 2.0, 1.5],
            &[0.25, 1.0, 3.0],
        )?;

        assert_eq!(comparison.reference_argmax_index(), 1);
        assert_eq!(comparison.candidate_argmax_index(), 2);
        Ok(())
    }
}
