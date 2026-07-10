use super::{
    auth::ensure_authorized,
    error::OpenAiHttpError,
    generation::{generate_text, generate_texts},
    guards::{
        acquire_batch_admission_permit, acquire_inference_permit, ensure_model,
        ensure_supported_chat_request, ensure_supported_completion_request, normalized_max_tokens,
        required_engine,
    },
    json::AuthorizedOpenAiJson,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
    },
    stream_generation::{
        chat_batched_stream_response, chat_stream_response, completion_batched_stream_response,
        completion_stream_response, ChatStreamOptions, CompletionStreamOptions,
    },
};
use crate::runtime::GenerationCacheOptions;
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
            "/v1/models/*model",
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
    AuthorizedOpenAiJson(request): AuthorizedOpenAiJson<ChatCompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    ensure_supported_chat_request(&request)?;
    let prompt = render_chat_prompt(request.messages())?;
    let max_tokens =
        normalized_max_tokens(&state, request.max_tokens(), request.max_tokens_param())?;
    let engine = required_engine(&state)?;
    let cache_options = chat_cache_options(&state, &request);
    if request.stream() {
        let stream_options = ChatStreamOptions::new(
            request.stop_sequences(),
            request.stream_include_usage(),
            request.stream_include_obfuscation(),
            request.response_service_tier(),
        );
        if let Some(scheduler) = eligible_batch_scheduler(&state, &cache_options) {
            let permit = acquire_batch_admission_permit(&state).await?;
            return chat_batched_stream_response(
                scheduler,
                state.model_id().to_owned(),
                prompt,
                max_tokens,
                stream_options,
                permit,
            );
        }
        let permit = acquire_inference_permit(&state).await?;
        return Ok(chat_stream_response(
            engine,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            stream_options,
            cache_options,
            permit,
        ));
    }
    let permit = acquire_inference_permit(&state).await?;
    let generated = generate_text(
        Some(engine),
        prompt,
        max_tokens,
        request.stop_sequences(),
        cache_options,
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

fn chat_cache_options(
    state: &ServerState,
    request: &ChatCompletionRequest,
) -> GenerationCacheOptions {
    request
        .cache_options()
        .with_prefix_cache_enabled(state.prefix_cache_enabled())
}

async fn completions(
    State(state): State<ServerState>,
    AuthorizedOpenAiJson(request): AuthorizedOpenAiJson<CompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    ensure_supported_completion_request(&request)?;
    if request.prompts().is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "prompt must contain at least one item",
            "prompt",
        ));
    }
    if request
        .prompts()
        .iter()
        .any(|prompt| prompt.trim().is_empty())
    {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "prompt must contain non-whitespace text",
            "prompt",
        ));
    }
    let max_tokens =
        normalized_max_tokens(&state, request.max_tokens(), request.max_tokens_param())?;
    let stream_prompt = if request.stream() {
        Some(
            request
                .single_prompt()
                .ok_or_else(|| {
                    OpenAiHttpError::invalid_request_with_param(
                        "streaming completions require exactly one text prompt",
                        "prompt",
                    )
                })?
                .to_owned(),
        )
    } else {
        None
    };
    let engine = required_engine(&state)?;
    let cache_options = completion_cache_options(&state, &request);
    if let Some(prompt) = stream_prompt {
        let echo_prompt = request.echo().then(|| prompt.clone());
        let stream_options = CompletionStreamOptions::new(
            request.stop_sequences(),
            request.stream_include_usage(),
            request.stream_include_obfuscation(),
        )
        .with_echo_prompt(echo_prompt);
        if let Some(scheduler) = eligible_batch_scheduler(&state, &cache_options) {
            let permit = acquire_batch_admission_permit(&state).await?;
            return completion_batched_stream_response(
                scheduler,
                state.model_id().to_owned(),
                prompt,
                max_tokens,
                stream_options,
                permit,
            );
        }
        let permit = acquire_inference_permit(&state).await?;
        return Ok(completion_stream_response(
            engine,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            stream_options,
            cache_options,
            permit,
        ));
    }
    let permit = acquire_inference_permit(&state).await?;
    let generated = generate_texts(
        Some(engine),
        request.prompts().to_vec(),
        max_tokens,
        request.stop_sequences(),
        cache_options,
        permit,
    )
    .await?;
    Ok(Json(CompletionResponse::from_prompt_generations(
        state.model_id().to_owned(),
        request.prompts(),
        generated,
        request.echo(),
    ))
    .into_response())
}

fn completion_cache_options(
    state: &ServerState,
    request: &CompletionRequest,
) -> GenerationCacheOptions {
    request
        .cache_options()
        .with_prefix_cache_enabled(state.prefix_cache_enabled())
}

fn eligible_batch_scheduler(
    state: &ServerState,
    cache_options: &GenerationCacheOptions,
) -> Option<std::sync::Arc<crate::runtime::BatchScheduler>> {
    if cache_options.prefix_cache_enabled() || cache_options.prompt_cache_trace_enabled() {
        None
    } else {
        state.batch_scheduler()
    }
}

async fn method_not_allowed(
    State(state): State<ServerState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> OpenAiHttpError {
    if uri.path().starts_with("/v1/") {
        if let Err(error) = ensure_authorized(&state, &headers) {
            return error;
        }
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
