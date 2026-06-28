use super::{
    error::OpenAiHttpError,
    json::OpenAiJson,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, ChatCompletionStreamContext,
        CompletionRequest, CompletionResponse, CompletionStreamContext, HealthResponse,
        ModelObject, ModelsResponse,
    },
    streaming,
};
use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::get,
    routing::post,
    Json, Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedSemaphorePermit;

const DEFAULT_MAX_TOKENS: usize = 16;
const HARD_MAX_TOKENS: usize = 256;

pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/models/:model", get(model))
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

async fn model(
    State(state): State<ServerState>,
    Path(model): Path<String>,
) -> Result<Json<ModelObject>, OpenAiHttpError> {
    if model != state.model_id() {
        return Err(OpenAiHttpError::model_not_found(model));
    }
    Ok(Json(ModelObject::local(state.model_id().to_owned())))
}

async fn chat_completions(
    State(state): State<ServerState>,
    OpenAiJson(request): OpenAiJson<ChatCompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    ensure_supported_chat_request(&request)?;
    let prompt = render_chat_prompt(request.messages())?;
    let max_tokens = normalized_max_tokens(request.max_tokens())?;
    let permit = acquire_inference_permit(&state)?;
    if request.stream() {
        return Ok(chat_stream_response(
            required_engine(&state)?,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            permit,
        ));
    }
    let generated = generate_text(state.engine(), prompt, max_tokens, permit).await?;
    Ok(Json(ChatCompletionResponse::from_generation(
        state.model_id().to_owned(),
        generated,
    ))
    .into_response())
}

async fn completions(
    State(state): State<ServerState>,
    OpenAiJson(request): OpenAiJson<CompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    if request.prompt().trim().is_empty() {
        return Err(OpenAiHttpError::invalid_request(
            "prompt must contain non-whitespace text",
        ));
    }
    let max_tokens = normalized_max_tokens(request.max_tokens())?;
    let permit = acquire_inference_permit(&state)?;
    if request.stream() {
        return Ok(completion_stream_response(
            required_engine(&state)?,
            state.model_id().to_owned(),
            request.prompt().to_owned(),
            max_tokens,
            permit,
        ));
    }
    let generated = generate_text(
        state.engine(),
        request.prompt().to_owned(),
        max_tokens,
        permit,
    )
    .await?;
    Ok(Json(CompletionResponse::from_generation(
        state.model_id().to_owned(),
        generated,
    ))
    .into_response())
}

fn ensure_supported_chat_request(request: &ChatCompletionRequest) -> Result<(), OpenAiHttpError> {
    let unsupported = request.unsupported_fields();
    if unsupported.is_empty() {
        return Ok(());
    }

    Err(OpenAiHttpError::invalid_request(format!(
        "unsupported chat completion field(s): {}",
        unsupported.join(", ")
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

fn required_engine(
    state: &ServerState,
) -> Result<Arc<Mutex<crate::runtime::InferenceEngine>>, OpenAiHttpError> {
    state.engine().ok_or_else(|| {
        OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        )
    })
}

fn acquire_inference_permit(state: &ServerState) -> Result<OwnedSemaphorePermit, OpenAiHttpError> {
    state.try_acquire_inference_permit().ok_or_else(|| {
        OpenAiHttpError::rate_limited("inference request queue is full; retry later")
    })
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

pub(super) fn completion_stream_response(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    model: String,
    prompt: String,
    max_tokens: usize,
    permit: OwnedSemaphorePermit,
) -> Response {
    let context = CompletionStreamContext::new(model);
    let stop_chunk = context.stop();
    stream_generated_text(
        engine,
        prompt,
        max_tokens,
        move |piece| context.token(piece.to_owned()),
        stop_chunk,
        permit,
    )
}

fn chat_stream_response(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    model: String,
    prompt: String,
    max_tokens: usize,
    permit: OwnedSemaphorePermit,
) -> Response {
    let context = ChatCompletionStreamContext::new(model);
    let stop_chunk = context.stop();
    stream_generated_text(
        engine,
        prompt,
        max_tokens,
        move |piece| context.token(piece.to_owned()),
        stop_chunk,
        permit,
    )
}

fn stream_generated_text<T>(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    prompt: String,
    max_tokens: usize,
    mut token_chunk: impl FnMut(&str) -> T + Send + 'static,
    stop_chunk: T,
    permit: OwnedSemaphorePermit,
) -> Response
where
    T: serde::Serialize + Send + 'static,
{
    let (sender, response) = streaming::channel_response();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let result = (|| -> Result<(), OpenAiHttpError> {
            let engine = engine
                .lock()
                .map_err(|_| OpenAiHttpError::internal("inference engine lock is poisoned"))?;
            engine
                .generate_with_token_callback(&prompt, max_tokens, |piece| {
                    sender
                        .send_json_blocking(&token_chunk(piece))
                        .map_err(|error| crate::runtime::RuntimeError::new(error.to_string()))?;
                    Ok(())
                })
                .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
            sender.send_json_blocking(&stop_chunk)?;
            sender.send_done_blocking()
        })();
        if result.is_err() {
            let _ = sender.send_done_blocking();
        }
    });
    response
}

async fn generate_text(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompt: String,
    max_tokens: usize,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let Some(engine) = engine else {
        return Err(OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        ));
    };

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
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
