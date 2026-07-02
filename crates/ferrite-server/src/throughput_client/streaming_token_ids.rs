#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamingTokenIdsSummary {
    content_chunks: usize,
    token_id_chunks: usize,
    token_ids: usize,
}

impl StreamingTokenIdsSummary {
    pub fn new(content_chunks: usize, token_id_chunks: usize, token_ids: usize) -> Self {
        Self {
            content_chunks,
            token_id_chunks,
            token_ids,
        }
    }

    pub fn from_sse_body(body: &str) -> Option<Self> {
        let mut summary = Self::new(0, 0, 0);

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
