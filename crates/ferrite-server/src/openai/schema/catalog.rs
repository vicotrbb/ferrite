use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    ready: bool,
    model: String,
}

impl HealthResponse {
    pub fn new(model: String, ready: bool) -> Self {
        Self {
            status: "ok",
            ready,
            model,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelObject>,
}

impl ModelsResponse {
    pub fn new(data: Vec<ModelObject>) -> Self {
        Self {
            object: "list",
            data,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelObject {
    id: String,
    object: &'static str,
    created: u64,
    owned_by: &'static str,
}

impl ModelObject {
    pub fn local(id: String) -> Self {
        Self {
            id,
            object: "model",
            created: 0,
            owned_by: "ferrite",
        }
    }
}
