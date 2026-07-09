use super::usage::Usage;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(super) enum StreamUsage {
    Value(Box<Usage>),
    Null(()),
}

impl StreamUsage {
    pub(super) fn null() -> Self {
        Self::Null(())
    }

    pub(super) fn value(usage: Usage) -> Self {
        Self::Value(Box::new(usage))
    }
}
