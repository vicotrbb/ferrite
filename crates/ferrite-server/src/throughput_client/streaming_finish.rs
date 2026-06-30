#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingFinishSummary {
    reason: String,
}

impl StreamingFinishSummary {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        body.lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(str::trim)
            .filter(|data| *data != "[DONE]")
            .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .filter_map(|event| finish_reason_from_event(&event))
            .find(|reason| !reason.is_empty())
            .map(Self::new)
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

fn finish_reason_from_event(event: &serde_json::Value) -> Option<String> {
    event
        .get("choices")?
        .as_array()?
        .iter()
        .filter_map(|choice| choice.get("finish_reason")?.as_str())
        .find(|reason| !reason.is_empty())
        .map(str::to_owned)
}
