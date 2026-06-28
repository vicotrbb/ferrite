use super::{
    error::OpenAiHttpError,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
        HealthResponse, ModelObject, ModelsResponse,
    },
};
use crate::state::ServerState;
use axum::{extract::State, routing::get, routing::post, Json, Router};
use std::sync::{Arc, Mutex};

const DEFAULT_MAX_TOKENS: usize = 16;
const HARD_MAX_TOKENS: usize = 256;

pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
        .with_state(state)
}

async fn health(State(state): State<ServerState>) -> Json<HealthResponse> {
    Json(HealthResponse::ready(state.model_id().to_owned()))
}

async fn models(State(state): State<ServerState>) -> Json<ModelsResponse> {
    Json(ModelsResponse::new(vec![ModelObject::local(
        state.model_id().to_owned(),
    )]))
}

async fn chat_completions(
    State(state): State<ServerState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, OpenAiHttpError> {
    if request.stream() {
        return Err(OpenAiHttpError::not_implemented(
            "chat completion streaming is not implemented yet",
        ));
    }
    ensure_model(&state, request.model())?;
    let prompt = render_chat_prompt(request.messages())?;
    let max_tokens = normalized_max_tokens(request.max_tokens())?;
    let generated = generate_text(state.engine(), prompt, max_tokens).await?;
    Ok(Json(ChatCompletionResponse::from_generation(
        state.model_id().to_owned(),
        generated,
    )))
}

async fn completions(
    State(state): State<ServerState>,
    Json(request): Json<CompletionRequest>,
) -> Result<Json<CompletionResponse>, OpenAiHttpError> {
    if request.stream() {
        return Err(OpenAiHttpError::not_implemented(
            "completion streaming is not implemented yet",
        ));
    }
    ensure_model(&state, request.model())?;
    if request.prompt().trim().is_empty() {
        return Err(OpenAiHttpError::invalid_request(
            "prompt must contain non-whitespace text",
        ));
    }
    let max_tokens = normalized_max_tokens(request.max_tokens())?;
    let generated = generate_text(state.engine(), request.prompt().to_owned(), max_tokens).await?;
    Ok(Json(CompletionResponse::from_generation(
        state.model_id().to_owned(),
        generated,
    )))
}

fn ensure_model(state: &ServerState, requested_model: &str) -> Result<(), OpenAiHttpError> {
    if requested_model == state.model_id() {
        Ok(())
    } else {
        Err(OpenAiHttpError::invalid_request(format!(
            "model {requested_model} is not loaded"
        )))
    }
}

fn normalized_max_tokens(value: Option<usize>) -> Result<usize, OpenAiHttpError> {
    let tokens = value.unwrap_or(DEFAULT_MAX_TOKENS);
    if tokens == 0 {
        return Err(OpenAiHttpError::invalid_request(
            "max_tokens must be greater than zero",
        ));
    }
    if tokens > HARD_MAX_TOKENS {
        return Err(OpenAiHttpError::invalid_request(format!(
            "max_tokens must be less than or equal to {HARD_MAX_TOKENS}"
        )));
    }
    Ok(tokens)
}

async fn generate_text(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompt: String,
    max_tokens: usize,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let Some(engine) = engine else {
        return Err(OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        ));
    };

    tokio::task::spawn_blocking(move || {
        let engine = engine
            .lock()
            .map_err(|_| OpenAiHttpError::internal("inference engine lock is poisoned"))?;
        engine
            .generate(&prompt, max_tokens)
            .map_err(|error| OpenAiHttpError::internal(error.to_string()))
    })
    .await
    .map_err(|error| OpenAiHttpError::internal(format!("inference task failed: {error}")))?
}
