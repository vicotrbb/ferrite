use crate::diagnostic_hash::{fnv64_bytes, format_fnv64};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingTextSummary {
    text: String,
    chunks: Vec<String>,
}

impl StreamingTextSummary {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            chunks: vec![text.clone()],
            text,
        }
    }

    pub fn from_chunks(chunks: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let chunks = chunks.into_iter().map(Into::into).collect::<Vec<_>>();
        Self {
            text: chunks.concat(),
            chunks,
        }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        let chunks = body
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(str::trim)
            .filter(|data| *data != "[DONE]")
            .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .flat_map(text_from_event)
            .collect::<Vec<_>>();

        (!chunks.is_empty()).then(|| Self::from_chunks(chunks))
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn chunks(&self) -> &[String] {
        &self.chunks
    }

    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn text_hash(&self) -> u64 {
        fnv64_bytes(self.text.as_bytes())
    }

    pub fn formatted_text_hash(&self) -> String {
        format_fnv64(self.text_hash())
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
