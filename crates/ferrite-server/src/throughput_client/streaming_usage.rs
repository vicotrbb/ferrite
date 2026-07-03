#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingUsageSummary {
    prompt_tokens: u64,
    cached_prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    prompt_cache_trace: Option<StreamingPromptCacheTraceSummary>,
}

impl StreamingUsageSummary {
    pub fn new(prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            cached_prompt_tokens: 0,
            completion_tokens,
            total_tokens,
            prompt_cache_trace: None,
        }
    }

    pub fn with_cached_prompt_tokens(mut self, cached_prompt_tokens: u64) -> Self {
        self.cached_prompt_tokens = cached_prompt_tokens;
        self
    }

    pub fn with_prompt_cache_trace(
        mut self,
        prompt_cache_trace: StreamingPromptCacheTraceSummary,
    ) -> Self {
        self.prompt_cache_trace = Some(prompt_cache_trace);
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

    pub fn prompt_cache_trace(&self) -> Option<&StreamingPromptCacheTraceSummary> {
        self.prompt_cache_trace.as_ref()
    }

    fn from_value(value: serde_json::Value) -> Option<Self> {
        if value.is_null() {
            return None;
        }
        let prompt_details = value.get("prompt_tokens_details");
        Some(Self {
            prompt_tokens: value.get("prompt_tokens")?.as_u64()?,
            cached_prompt_tokens: prompt_details
                .and_then(|details| details.get("cached_tokens"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            completion_tokens: value.get("completion_tokens")?.as_u64()?,
            total_tokens: value.get("total_tokens")?.as_u64()?,
            prompt_cache_trace: prompt_details
                .and_then(|details| details.get("ferrite_cache"))
                .and_then(StreamingPromptCacheTraceSummary::from_value),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingPromptCacheTraceSummary {
    lookup: String,
    prompt_token_hash: String,
    shared_prefix_tokens: u64,
    selected_entry_token_hash: Option<String>,
}

impl StreamingPromptCacheTraceSummary {
    pub fn new(lookup: String, prompt_token_hash: String, shared_prefix_tokens: u64) -> Self {
        Self {
            lookup,
            prompt_token_hash,
            shared_prefix_tokens,
            selected_entry_token_hash: None,
        }
    }

    pub fn with_selected_entry_token_hash(mut self, selected_entry_token_hash: String) -> Self {
        self.selected_entry_token_hash = Some(selected_entry_token_hash);
        self
    }

    pub fn lookup(&self) -> &str {
        &self.lookup
    }

    pub fn prompt_token_hash(&self) -> &str {
        &self.prompt_token_hash
    }

    pub fn shared_prefix_tokens(&self) -> u64 {
        self.shared_prefix_tokens
    }

    pub fn selected_entry_token_hash(&self) -> Option<&str> {
        self.selected_entry_token_hash.as_deref()
    }

    fn from_value(value: &serde_json::Value) -> Option<Self> {
        let mut summary = Self::new(
            value.get("lookup")?.as_str()?.to_owned(),
            value.get("prompt_token_hash")?.as_str()?.to_owned(),
            value.get("shared_prefix_tokens")?.as_u64()?,
        );
        if let Some(selected_entry_token_hash) = value
            .get("selected_entry_token_hash")
            .and_then(serde_json::Value::as_str)
        {
            summary = summary.with_selected_entry_token_hash(selected_entry_token_hash.to_owned());
        }
        Some(summary)
    }
}
