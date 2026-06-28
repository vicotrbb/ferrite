use std::time::Duration;

use super::NextToken;

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarProfileEvent {
    label: String,
    elapsed: Duration,
}

impl ScalarProfileEvent {
    pub(super) fn new(label: impl Into<String>, elapsed: Duration) -> Self {
        Self {
            label: label.into(),
            elapsed,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
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
