use super::{auth::ensure_authorized, error::OpenAiHttpError};
use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

use super::schema::{HealthResponse, ModelObject, ModelsResponse};

pub(super) async fn health(State(state): State<ServerState>) -> Json<HealthResponse> {
    Json(HealthResponse::new(
        state.model_id().to_owned(),
        state.has_loaded_model(),
    ))
}

pub(super) async fn models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelsResponse>, OpenAiHttpError> {
    ensure_authorized(&state, &headers)?;
    let models = if state.has_loaded_model() {
        vec![ModelObject::local(state.model_id().to_owned())]
    } else {
        Vec::new()
    };
    Ok(Json(ModelsResponse::new(models)))
}

pub(super) async fn model(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(model): Path<String>,
) -> Result<Json<ModelObject>, OpenAiHttpError> {
    ensure_authorized(&state, &headers)?;
    if model != state.model_id() || !state.has_loaded_model() {
        return Err(OpenAiHttpError::model_not_found(model));
    }
    Ok(Json(ModelObject::local(state.model_id().to_owned())))
}
