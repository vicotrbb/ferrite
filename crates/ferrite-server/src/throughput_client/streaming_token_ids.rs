#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingTokenIdsSummary {
    content_chunks: usize,
    token_id_chunks: usize,
    token_ids: usize,
    token_id_trace: Option<Vec<u64>>,
    all_request_traces_match: Option<bool>,
    prompt_token_id_traces: Option<Vec<Option<Vec<u64>>>>,
    all_prompt_traces_stable: Option<bool>,
}

impl StreamingTokenIdsSummary {
    pub fn new(content_chunks: usize, token_id_chunks: usize, token_ids: usize) -> Self {
        Self {
            content_chunks,
            token_id_chunks,
            token_ids,
            token_id_trace: None,
            all_request_traces_match: None,
            prompt_token_id_traces: None,
            all_prompt_traces_stable: None,
        }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        let mut summary = Self {
            content_chunks: 0,
            token_id_chunks: 0,
            token_ids: 0,
            token_id_trace: Some(Vec::new()),
            all_request_traces_match: None,
            prompt_token_id_traces: None,
            all_prompt_traces_stable: None,
        };

        for event in sse_json_events(body) {
            summary.accumulate_event(&event);
        }

        (summary.content_chunks > 0 || summary.token_id_chunks > 0).then_some(summary)
    }

    pub fn content_chunks(&self) -> usize {
        self.content_chunks
    }

    pub fn token_id_chunks(&self) -> usize {
        self.token_id_chunks
    }

    pub fn token_ids(&self) -> usize {
        self.token_ids
    }

    pub fn token_id_trace(&self) -> Option<&[u64]> {
        self.token_id_trace.as_deref()
    }

    pub fn all_request_traces_match(&self) -> Option<bool> {
        self.all_request_traces_match
    }

    pub fn set_all_request_traces_match(&mut self, matches: bool) {
        self.all_request_traces_match = Some(matches);
    }

    pub fn prompt_token_id_traces(&self) -> Option<&[Option<Vec<u64>>]> {
        self.prompt_token_id_traces.as_deref()
    }

    pub fn all_prompt_traces_stable(&self) -> Option<bool> {
        self.all_prompt_traces_stable
    }

    pub fn set_prompt_token_id_traces(&mut self, traces: Vec<Option<Vec<u64>>>, all_stable: bool) {
        self.prompt_token_id_traces = Some(traces);
        self.all_prompt_traces_stable = Some(all_stable);
    }

    pub fn all_content_chunks_have_token_ids(&self) -> bool {
        self.content_chunks > 0 && self.content_chunks == self.token_id_chunks
    }

    fn accumulate_event(&mut self, event: &serde_json::Value) {
        let Some(choices) = event.get("choices").and_then(serde_json::Value::as_array) else {
            return;
        };

        for choice in choices {
            if choice_has_generated_text(choice) {
                self.content_chunks += 1;
            }
            let Some(token_ids) = choice
                .get("token_ids")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            if token_ids.is_empty() {
                continue;
            }
            self.token_id_chunks += 1;
            self.token_ids += token_ids.len();
            let parsed_token_ids = token_ids
                .iter()
                .map(serde_json::Value::as_u64)
                .collect::<Option<Vec<_>>>();
            match (&mut self.token_id_trace, parsed_token_ids) {
                (Some(trace), Some(parsed)) => trace.extend(parsed),
                _ => self.token_id_trace = None,
            }
        }
    }
}

fn sse_json_events(body: &str) -> impl Iterator<Item = serde_json::Value> + '_ {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(str::trim)
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
}

fn choice_has_generated_text(choice: &serde_json::Value) -> bool {
    choice
        .get("text")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| !text.is_empty())
        || choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|content| !content.is_empty())
}
