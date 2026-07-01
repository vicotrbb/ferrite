#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingTextSummary {
    text: String,
}

impl StreamingTextSummary {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        let text = body
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(str::trim)
            .filter(|data| *data != "[DONE]")
            .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .flat_map(text_from_event)
            .collect::<String>();

        (!text.is_empty()).then(|| Self::new(text))
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn byte_len(&self) -> usize {
        self.text.len()
    }
}

fn text_from_event(event: serde_json::Value) -> Vec<String> {
    let Some(choices) = event.get("choices").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };

    choices
        .iter()
        .filter_map(|choice| {
            choice
                .get("delta")
                .and_then(|delta| delta.get("content"))
                .and_then(serde_json::Value::as_str)
                .or_else(|| choice.get("text").and_then(serde_json::Value::as_str))
        })
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .collect()
}
