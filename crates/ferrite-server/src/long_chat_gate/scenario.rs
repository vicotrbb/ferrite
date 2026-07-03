#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongChatScenario<'a> {
    model: &'a str,
    turn: usize,
    token_length: usize,
    prompt_cache_key: Option<&'a str>,
}

impl<'a> LongChatScenario<'a> {
    pub(super) fn new(model: &'a str, turn: usize, token_length: usize) -> Self {
        Self::new_with_prompt_cache_key(model, turn, token_length, None)
    }

    pub(super) fn new_with_prompt_cache_key(
        model: &'a str,
        turn: usize,
        token_length: usize,
        prompt_cache_key: Option<&'a str>,
    ) -> Self {
        Self {
            model,
            turn,
            token_length,
            prompt_cache_key,
        }
    }

    pub fn model(&self) -> &str {
        self.model
    }

    pub fn turn(&self) -> usize {
        self.turn
    }

    pub fn token_length(&self) -> usize {
        self.token_length
    }

    pub fn prompt_cache_key(&self) -> Option<&str> {
        self.prompt_cache_key
    }
}
