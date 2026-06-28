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

        let mut max_abs_diff = 0.0f32;
        let mut max_relative_diff = 0.0f32;
        for (reference, candidate) in reference.iter().zip(candidate) {
            let abs_diff = (reference - candidate).abs();
            max_abs_diff = max_abs_diff.max(abs_diff);
            let denominator = reference.abs().max(1.0e-6);
            max_relative_diff = max_relative_diff.max(abs_diff / denominator);
        }

        Ok(Self {
            label: label.into(),
            storage_kind: matrix.storage_kind(),
            rows: matrix.rows(),
            cols: matrix.cols(),
            storage_bytes: matrix.storage_bytes(),
            max_abs_diff,
            max_relative_diff,
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
