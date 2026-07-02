#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamingUsageSummary {
    prompt_tokens: u64,
    cached_prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

impl StreamingUsageSummary {
    pub fn new(prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            cached_prompt_tokens: 0,
            completion_tokens,
            total_tokens,
        }
    }

    pub fn with_cached_prompt_tokens(mut self, cached_prompt_tokens: u64) -> Self {
        self.cached_prompt_tokens = cached_prompt_tokens;
        self
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

    pub fn cached_prompt_tokens(&self) -> u64 {
        self.cached_prompt_tokens
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
            cached_prompt_tokens: value
                .get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            completion_tokens: value.get("completion_tokens")?.as_u64()?,
            total_tokens: value.get("total_tokens")?.as_u64()?,
        })
    }
}
