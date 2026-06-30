#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongChatScenario<'a> {
    model: &'a str,
    turn: usize,
    token_length: usize,
}

impl<'a> LongChatScenario<'a> {
    pub(super) fn new(model: &'a str, turn: usize, token_length: usize) -> Self {
        Self {
            model,
            turn,
            token_length,
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
}
