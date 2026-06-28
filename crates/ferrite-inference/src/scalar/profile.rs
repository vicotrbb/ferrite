use std::time::Duration;

use super::{matrix::MatrixStorageKind, Matrix, NextToken};

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarProfileEvent {
    label: String,
    elapsed: Duration,
    storage_kind: MatrixStorageKind,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
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
}

impl ProfiledTokenId {
    pub fn total_elapsed(&self) -> Duration {
        self.events
            .iter()
            .map(ScalarProfileEvent::elapsed)
            .sum::<Duration>()
    }
}
