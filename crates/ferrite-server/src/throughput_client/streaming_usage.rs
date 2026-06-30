#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamingUsageSummary {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

impl StreamingUsageSummary {
    pub fn new(prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        body.lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(str::trim)
            .filter(|data| *data != "[DONE]")
            .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .filter_map(|event| event.get("usage").cloned())
            .find_map(Self::from_value)
    }

    pub fn prompt_tokens(&self) -> u64 {
        self.prompt_tokens
    }

    pub fn completion_tokens(&self) -> u64 {
        self.completion_tokens
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    fn from_value(value: serde_json::Value) -> Option<Self> {
        if value.is_null() {
            return None;
        }
        Some(Self {
            prompt_tokens: value.get("prompt_tokens")?.as_u64()?,
            completion_tokens: value.get("completion_tokens")?.as_u64()?,
            total_tokens: value.get("total_tokens")?.as_u64()?,
        })
    }
}
