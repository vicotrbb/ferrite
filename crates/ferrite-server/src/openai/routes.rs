use super::{
    auth::ensure_authorized,
    error::OpenAiHttpError,
    generation::{generate_text, generate_texts},
    guards::{
        acquire_inference_permit, ensure_model, ensure_supported_chat_request,
        ensure_supported_completion_request, normalized_max_tokens, required_engine,
    },
    json::OpenAiJson,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
    },
    stream_generation::{
        chat_stream_response, completion_stream_response, ChatStreamOptions,
        CompletionStreamOptions,
    },
};
use crate::state::ServerState;
use axum::{
    extract::{OriginalUri, State},
    http::HeaderMap,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    routing::post,
    Json, Router,
};

pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(super::catalog::health))
        .route(
            "/v1/models",
            get(super::catalog::models).options(super::cors::openai_preflight),
        )
        .route(
            "/v1/models/:model",
            get(super::catalog::model).options(super::cors::openai_preflight),
        )
        .route(
            "/v1/chat/completions",
            post(chat_completions).options(super::cors::openai_preflight),
        )
        .route(
            "/v1/completions",
            post(completions).options(super::cors::openai_preflight),
        )
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(middleware::from_fn(super::cors::add_openai_cors_headers))
        .with_state(state)
}

async fn chat_completions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    OpenAiJson(request): OpenAiJson<ChatCompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_authorized(&state, &headers)?;
    ensure_model(&state, request.model())?;
    ensure_supported_chat_request(&request)?;
    let prompt = render_chat_prompt(request.messages())?;
    let max_tokens =
        normalized_max_tokens(&state, request.max_tokens(), request.max_tokens_param())?;
    let engine = required_engine(&state)?;
    let permit = acquire_inference_permit(&state).await?;
    if request.stream() {
        return Ok(chat_stream_response(
            engine,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            ChatStreamOptions::new(
                request.stop_sequences(),
                request.stream_include_usage(),
                request.response_service_tier(),
            ),
            permit,
        ));
    }
    let generated = generate_text(
        Some(engine),
        prompt,
        max_tokens,
        request.stop_sequences(),
        permit,
    )
    .await?;
    Ok(Json(ChatCompletionResponse::from_generation(
        state.model_id().to_owned(),
        generated,
        request.response_service_tier(),
    ))
    .into_response())
}

async fn completions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    OpenAiJson(request): OpenAiJson<CompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_authorized(&state, &headers)?;
    ensure_model(&state, request.model())?;
    ensure_supported_completion_request(&request)?;
    if request.prompts().is_empty() {
        return Err(OpenAiHttpError::invalid_request(
            "prompt must contain at least one item",
        ));
    }
    if request
        .prompts()
        .iter()
        .any(|prompt| prompt.trim().is_empty())
    {
        return Err(OpenAiHttpError::invalid_request(
            "prompt must contain non-whitespace text",
        ));
    }
    let max_tokens =
        normalized_max_tokens(&state, request.max_tokens(), request.max_tokens_param())?;
    let stream_prompt = if request.stream() {
        Some(
            request
                .single_prompt()
                .ok_or_else(|| {
                    OpenAiHttpError::invalid_request(
                        "streaming completions require exactly one text prompt",
                    )
                })?
                .to_owned(),
        )
    } else {
        None
    };
    let engine = required_engine(&state)?;
    let permit = acquire_inference_permit(&state).await?;
    if let Some(prompt) = stream_prompt {
        return Ok(completion_stream_response(
            engine,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            CompletionStreamOptions::new(request.stop_sequences(), request.stream_include_usage()),
            permit,
        ));
    }
    let generated = generate_texts(
        Some(engine),
        request.prompts().to_vec(),
        max_tokens,
        request.stop_sequences(),
        permit,
    )
    .await?;
    Ok(Json(CompletionResponse::from_generations(
        state.model_id().to_owned(),
        generated,
    ))
    .into_response())
}

async fn method_not_allowed(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> OpenAiHttpError {
    if let Err(error) = ensure_authorized(&state, &headers) {
        return error;
    }
    OpenAiHttpError::method_not_allowed()
}

async fn not_found(
    State(state): State<ServerState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> OpenAiHttpError {
    if uri.path().starts_with("/v1/") {
        if let Err(error) = ensure_authorized(&state, &headers) {
            return error;
        }
    }
    OpenAiHttpError::route_not_found(uri.path())
}
