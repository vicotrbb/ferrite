#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerState {
    model_id: String,
}

impl ServerState {
    pub fn new(model_id: String) -> Self {
        Self { model_id }
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}
