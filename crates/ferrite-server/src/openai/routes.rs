use super::{
    auth::ensure_authorized,
    error::OpenAiHttpError,
    generation::{
        chat_stream_response, completion_stream_response, generate_text, generate_texts,
        ChatStreamOptions, CompletionStreamOptions,
    },
    json::OpenAiJson,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
        HealthResponse, ModelObject, ModelsResponse,
    },
};
use crate::{limits::TokenLimitError, state::ServerState};
use axum::{
    extract::{OriginalUri, Path, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    routing::post,
    Json, Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedSemaphorePermit;

pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models).options(openai_preflight))
        .route("/v1/models/:model", get(model).options(openai_preflight))
        .route(
            "/v1/chat/completions",
            post(chat_completions).options(openai_preflight),
        )
        .route(
            "/v1/completions",
            post(completions).options(openai_preflight),
        )
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(middleware::from_fn(add_openai_cors_headers))
        .with_state(state)
}

async fn health(State(state): State<ServerState>) -> Json<HealthResponse> {
    Json(HealthResponse::new(
        state.model_id().to_owned(),
        state.has_loaded_model(),
    ))
}

async fn models(
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

async fn model(
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

async fn openai_preflight() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    insert_openai_cors_headers(response.headers_mut());
    response
}

async fn add_openai_cors_headers(request: Request, next: Next) -> Response {
    let is_openai_route = request.uri().path().starts_with("/v1/");
    let mut response = next.run(request).await;
    if is_openai_route {
        insert_openai_cors_headers(response.headers_mut());
    }
    response
}

fn insert_openai_cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type"),
    );
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

fn ensure_supported_completion_request(request: &CompletionRequest) -> Result<(), OpenAiHttpError> {
    let unsupported = request.unsupported_fields();
    if unsupported.is_empty() {
        return Ok(());
    }

    Err(unsupported_field_error(
        "unsupported completion field(s)",
        &unsupported,
    ))
}

fn ensure_supported_chat_request(request: &ChatCompletionRequest) -> Result<(), OpenAiHttpError> {
    let unsupported = request.unsupported_fields();
    if unsupported.is_empty() {
        return Ok(());
    }

    Err(unsupported_field_error(
        "unsupported chat completion field(s)",
        &unsupported,
    ))
}

fn unsupported_field_error(label: &str, unsupported: &[String]) -> OpenAiHttpError {
    let message = format!("{label}: {}", unsupported.join(", "));
    match unsupported {
        [field] => OpenAiHttpError::invalid_request_with_param(message, field),
        _ => OpenAiHttpError::invalid_request(message),
    }
}

fn ensure_model(state: &ServerState, requested_model: &str) -> Result<(), OpenAiHttpError> {
    if requested_model.is_empty() {
        return Err(OpenAiHttpError::invalid_request("model is required"));
    }
    if requested_model == state.model_id() {
        Ok(())
    } else {
        Err(OpenAiHttpError::model_not_found(requested_model))
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

async fn acquire_inference_permit(
    state: &ServerState,
) -> Result<OwnedSemaphorePermit, OpenAiHttpError> {
    state.acquire_inference_permit().await.ok_or_else(|| {
        OpenAiHttpError::rate_limited("inference request queue is full; retry later")
    })
}

fn normalized_max_tokens(
    state: &ServerState,
    value: Option<usize>,
    param: Option<&'static str>,
) -> Result<usize, OpenAiHttpError> {
    state.token_limits().normalize(value).map_err(|error| {
        let message = token_limit_error_message(error, param);
        match param {
            Some(param) => OpenAiHttpError::invalid_request_with_param(message, param),
            None => OpenAiHttpError::invalid_request(message),
        }
    })
}

fn token_limit_error_message(error: TokenLimitError, param: Option<&str>) -> String {
    let field = param.unwrap_or("max_tokens");
    match error {
        TokenLimitError::RequestedMustBePositive => {
            format!("{field} must be greater than zero")
        }
        TokenLimitError::RequestedAboveHard { hard_max_tokens } => {
            format!("{field} must be less than or equal to {hard_max_tokens}")
        }
        _ => error.to_string(),
    }
}
