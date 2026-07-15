use super::{
    auth::ensure_authorized,
    error::OpenAiHttpError,
    generation::{
        generate_batched_text, generate_batched_texts, generate_json_object, generate_text,
        generate_texts,
    },
    guards::{
        acquire_batch_admission_permit, acquire_inference_permit, ensure_model,
        ensure_supported_chat_request, ensure_supported_completion_request,
        ensure_supported_responses_request, normalized_max_tokens, required_engine,
    },
    json::AuthorizedOpenAiJson,
    prompt::{
        render_chat_prompt_with_model_template, render_chat_prompt_with_model_template_and_tools,
        validate_chat_messages,
    },
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
        ResponsesRequest, ResponsesResponse,
    },
    stream_generation::{
        chat_batched_stream_response, chat_stream_response, completion_batched_stream_response,
        completion_stream_response, ChatStreamOptions, CompletionStreamOptions,
    },
};
use crate::runtime::GenerationCacheOptions;
use crate::state::ServerState;
use axum::{
    extract::{DefaultBodyLimit, OriginalUri, State},
    http::HeaderMap,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    routing::post,
    Json, Router,
};

pub(super) const MAX_OPENAI_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

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
        .route(
            "/v1/responses",
            post(responses).options(super::cors::openai_preflight),
        )
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(middleware::from_fn(super::cors::add_openai_cors_headers))
        .layer(DefaultBodyLimit::max(MAX_OPENAI_REQUEST_BODY_BYTES))
        .with_state(state)
}

async fn responses(
    State(state): State<ServerState>,
    AuthorizedOpenAiJson(request): AuthorizedOpenAiJson<ResponsesRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    ensure_supported_responses_request(&request)?;
    if request.stream() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "streaming Responses API requests are not supported",
            "stream",
        ));
    }
    let messages = request.messages().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
    let sampling = request.sampling_config().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
    let max_output_tokens = normalized_max_tokens(
        &state,
        request.max_output_tokens(),
        Some("max_output_tokens"),
    )?;
    let engine = required_engine(&state)?;
    validate_logit_bias_vocabulary(&sampling, engine.vocabulary_size())?;
    let prompt = render_chat_prompt_with_model_template(
        engine.chat_template(),
        engine.chat_template_bos_token(),
        &messages,
    )?;
    let cache_options = request
        .cache_options()
        .with_prefix_cache_enabled(state.prefix_cache_enabled());
    let generated = if let Some(scheduler) = eligible_batch_scheduler(&state, &sampling) {
        let permit = acquire_batch_admission_permit(&state).await?;
        generate_batched_text(
            scheduler,
            prompt,
            max_output_tokens,
            Vec::new(),
            cache_options,
            permit,
        )
        .await?
    } else {
        let permit = acquire_inference_permit(&state).await?;
        generate_text(
            Some(engine),
            prompt,
            max_output_tokens,
            Vec::new(),
            sampling.clone(),
            cache_options,
            permit,
        )
        .await?
    };
    Ok(Json(ResponsesResponse::from_generation(
        &request,
        state.model_id().to_owned(),
        &sampling,
        generated,
    ))
    .into_response())
}

async fn chat_completions(
    State(state): State<ServerState>,
    AuthorizedOpenAiJson(request): AuthorizedOpenAiJson<ChatCompletionRequest>,
) -> Result<Response, OpenAiHttpError> {
    ensure_model(&state, request.model())?;
    let tools = request.tool_configuration().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
    ensure_supported_chat_request(&request)?;
    let sampling = request.sampling_config().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
    validate_chat_messages(request.messages())?;
    let json_object = request.requests_json_object();
    if tools.enabled() && request.stream() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "streaming tool calls are not supported",
            "stream",
        ));
    }
    if tools.enabled() && json_object {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "response_format JSON mode cannot be combined with function tools",
            "response_format",
        ));
    }
    if tools.enabled() && !request.stop_sequences().is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "stop sequences can truncate a tool call and are not supported with function tools",
            "stop",
        ));
    }
    if json_object && request.stream() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "streaming JSON-object responses are not supported",
            "response_format",
        ));
    }
    if json_object && !request.stop_sequences().is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "stop sequences can truncate a JSON object and are not supported with JSON mode",
            "stop",
        ));
    }
    if json_object
        && !request
            .messages()
            .iter()
            .any(|message| message.content().to_ascii_lowercase().contains("json"))
    {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "JSON mode requires an instruction containing the word JSON",
            "messages",
        ));
    }
    let max_tokens =
        normalized_max_tokens(&state, request.max_tokens(), request.max_tokens_param())?;
    let engine = required_engine(&state)?;
    validate_logit_bias_vocabulary(&sampling, engine.vocabulary_size())?;
    let tool_prompt_suffix = tools.prompt_suffix().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
    let prompt = render_chat_prompt_with_model_template_and_tools(
        engine.chat_template(),
        engine.chat_template_bos_token(),
        request.messages(),
        tool_prompt_suffix.as_deref(),
    )?;
    let cache_options = chat_cache_options(&state, &request)
        .with_prefix_cache_enabled(state.prefix_cache_enabled() && !json_object);
    if request.stream() {
        let stream_options = ChatStreamOptions::new(
            request.stop_sequences(),
            request.stream_include_usage(),
            request.stream_include_obfuscation(),
            request.response_service_tier(),
        )
        .with_sampling(sampling.clone());
        if let Some(scheduler) = eligible_batch_scheduler(&state, &sampling) {
            let permit = acquire_batch_admission_permit(&state).await?;
            return chat_batched_stream_response(
                scheduler,
                state.model_id().to_owned(),
                prompt,
                max_tokens,
                stream_options,
                cache_options,
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
    let generated = if json_object {
        let permit = acquire_inference_permit(&state).await?;
        generate_json_object(engine, prompt, max_tokens, sampling, cache_options, permit).await?
    } else if let Some(scheduler) = eligible_batch_scheduler(&state, &sampling) {
        let permit = acquire_batch_admission_permit(&state).await?;
        generate_batched_text(
            scheduler,
            prompt,
            max_tokens,
            request.stop_sequences(),
            cache_options,
            permit,
        )
        .await?
    } else {
        let permit = acquire_inference_permit(&state).await?;
        generate_text(
            Some(engine),
            prompt,
            max_tokens,
            request.stop_sequences(),
            sampling,
            cache_options,
            permit,
        )
        .await?
    };
    let response = if tools.enabled() {
        let parsed = tools.parse_output(generated.text()).map_err(|error| {
            OpenAiHttpError::internal(format!("invalid model tool call: {error}"))
        })?;
        ChatCompletionResponse::from_generation_with_tool_output(
            state.model_id().to_owned(),
            generated,
            request.response_service_tier(),
            parsed,
        )
    } else {
        ChatCompletionResponse::from_generation(
            state.model_id().to_owned(),
            generated,
            request.response_service_tier(),
        )
    };
    Ok(Json(response).into_response())
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
    let sampling = request.sampling_config().map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(error.to_string(), error.parameter())
    })?;
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
    validate_logit_bias_vocabulary(&sampling, engine.vocabulary_size())?;
    let cache_options = completion_cache_options(&state, &request);
    if let Some(prompt) = stream_prompt {
        let echo_prompt = request.echo().then(|| prompt.clone());
        let stream_options = CompletionStreamOptions::new(
            request.stop_sequences(),
            request.stream_include_usage(),
            request.stream_include_obfuscation(),
        )
        .with_echo_prompt(echo_prompt)
        .with_sampling(sampling.clone());
        if let Some(scheduler) = eligible_batch_scheduler(&state, &sampling) {
            let permit = acquire_batch_admission_permit(&state).await?;
            return completion_batched_stream_response(
                scheduler,
                state.model_id().to_owned(),
                prompt,
                max_tokens,
                stream_options,
                cache_options,
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
    let generated = if let Some(scheduler) = eligible_batch_scheduler(&state, &sampling) {
        let permit = acquire_batch_admission_permit(&state).await?;
        generate_batched_texts(
            scheduler,
            request.prompts().to_vec(),
            max_tokens,
            request.stop_sequences(),
            cache_options,
            permit,
        )
        .await?
    } else {
        let permit = acquire_inference_permit(&state).await?;
        generate_texts(
            Some(engine),
            request.prompts().to_vec(),
            max_tokens,
            request.stop_sequences(),
            sampling,
            cache_options,
            permit,
        )
        .await?
    };
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
    sampling: &ferrite_inference::sampling::SamplingConfig,
) -> Option<std::sync::Arc<crate::runtime::BatchScheduler>> {
    if !sampling.uses_fused_greedy_path() {
        None
    } else {
        state.batch_scheduler()
    }
}

fn validate_logit_bias_vocabulary(
    sampling: &ferrite_inference::sampling::SamplingConfig,
    vocabulary_size: usize,
) -> Result<(), OpenAiHttpError> {
    if let Some(token_id) = sampling
        .logit_bias()
        .keys()
        .find(|token_id| **token_id >= vocabulary_size)
    {
        return Err(OpenAiHttpError::invalid_request_with_param(
            format!(
                "logit_bias token {token_id} is out of bounds for vocabulary size {vocabulary_size}"
            ),
            "logit_bias",
        ));
    }
    Ok(())
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
