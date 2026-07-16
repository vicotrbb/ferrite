use std::time::Duration;

use super::{InferenceError, Matrix, NextToken, matrix::MatrixStorageKind};

#[derive(Clone, Debug, PartialEq)]
/// Timing and storage metadata for one profiled matrix-vector operation.
pub struct ScalarProfileEvent {
    label: String,
    elapsed: Duration,
    storage_kind: MatrixStorageKind,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
}

#[derive(Clone, Debug, PartialEq)]
/// Numeric and argmax comparison between reference and candidate matvecs.
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
    reference_argmax_margin: f32,
    candidate_argmax_margin: f32,
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
        let reference_argmax_margin = argmax_margin(reference);
        let candidate_argmax_margin = argmax_margin(candidate);

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
            reference_argmax_margin,
            candidate_argmax_margin,
        })
    }

    /// Returns the projection or operation label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the matrix storage representation.
    pub fn storage_kind(&self) -> MatrixStorageKind {
        self.storage_kind
    }

    /// Returns the matrix row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the matrix column count.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns the physical bytes owned by the matrix.
    pub fn storage_bytes(&self) -> u128 {
        self.storage_bytes
    }

    /// Returns the largest absolute element difference.
    pub fn max_abs_diff(&self) -> f32 {
        self.max_abs_diff
    }

    /// Returns the largest relative element difference, with a small floor on
    /// the reference denominator.
    pub fn max_relative_diff(&self) -> f32 {
        self.max_relative_diff
    }

    /// Returns the greatest-value index in the reference output.
    pub fn reference_argmax_index(&self) -> usize {
        self.reference_argmax_index
    }

    /// Returns the greatest-value index in the candidate output.
    pub fn candidate_argmax_index(&self) -> usize {
        self.candidate_argmax_index
    }

    /// Returns the reference output's gap between its largest two values.
    pub fn reference_argmax_margin(&self) -> f32 {
        self.reference_argmax_margin
    }

    /// Returns the candidate output's gap between its largest two values.
    pub fn candidate_argmax_margin(&self) -> f32 {
        self.candidate_argmax_margin
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

fn argmax_margin(values: &[f32]) -> f32 {
    let mut best = f32::NEG_INFINITY;
    let mut second = f32::NEG_INFINITY;
    for value in values {
        if *value > best {
            second = best;
            best = *value;
        } else if *value > second {
            second = *value;
        }
    }
    if second.is_finite() {
        best - second
    } else {
        0.0
    }
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

    /// Returns the projection or operation label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the measured operation duration.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the matrix storage representation.
    pub fn storage_kind(&self) -> MatrixStorageKind {
        self.storage_kind
    }

    /// Returns the matrix row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the matrix column count.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns the physical bytes owned by the matrix.
    pub fn storage_bytes(&self) -> u128 {
        self.storage_bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A next-token result with per-matvec timing and comparison records.
pub struct ProfiledNextToken {
    /// The selected token and full vocabulary logits.
    pub next_token: NextToken,
    /// Timed matrix-vector operations in execution order.
    pub events: Vec<ScalarProfileEvent>,
    /// Optional reference-versus-candidate comparisons.
    pub comparisons: Vec<ScalarMatVecComparison>,
}

impl ProfiledNextToken {
    /// Returns the sum of recorded matrix-vector durations.
    pub fn total_elapsed(&self) -> Duration {
        self.events
            .iter()
            .map(ScalarProfileEvent::elapsed)
            .sum::<Duration>()
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A selected token ID with per-matvec timing and comparison records.
pub struct ProfiledTokenId {
    /// The selected vocabulary token ID.
    pub token_id: usize,
    /// Timed matrix-vector operations in execution order.
    pub events: Vec<ScalarProfileEvent>,
    /// Optional reference-versus-candidate comparisons.
    pub comparisons: Vec<ScalarMatVecComparison>,
}

impl ProfiledTokenId {
    /// Returns the sum of recorded matrix-vector durations.
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
    fn matvec_comparison_records_argmax_indexes_and_margins() -> Result<(), InferenceError> {
        let matrix = Matrix::from_row_major(3, 2, vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0])?;
        let comparison = ScalarMatVecComparison::new(
            "q8_k_probe",
            &matrix,
            &[0.25, 2.0, 1.5],
            &[0.25, 1.0, 3.0],
        )?;

        assert_eq!(comparison.reference_argmax_index(), 1);
        assert_eq!(comparison.candidate_argmax_index(), 2);
        assert_eq!(comparison.reference_argmax_margin(), 0.5);
        assert_eq!(comparison.candidate_argmax_margin(), 2.0);
        Ok(())
    }
}
