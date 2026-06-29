use super::{
    error::OpenAiHttpError,
    generation::{chat_stream_response, completion_stream_response, generate_text, generate_texts},
    json::OpenAiJson,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
        HealthResponse, ModelObject, ModelsResponse,
    },
};
use crate::{limits::TokenLimitError, state::ServerState};
use axum::{
    extract::{OriginalUri, Path, State},
    http::HeaderMap,
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
        .route("/v1/models", get(models))
        .route("/v1/models/:model", get(model))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
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
    let permit = acquire_inference_permit(&state).await?;
    if request.stream() {
        return Ok(chat_stream_response(
            required_engine(&state)?,
            state.model_id().to_owned(),
            prompt,
            max_tokens,
            request.stream_include_usage(),
            request.response_service_tier(),
            permit,
        ));
    }
    let generated = generate_text(state.engine(), prompt, max_tokens, permit).await?;
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
    let permit = acquire_inference_permit(&state).await?;
    if request.stream() {
        let Some(prompt) = request.single_prompt() else {
            return Err(OpenAiHttpError::invalid_request(
                "streaming completions require exactly one text prompt",
            ));
        };
        return Ok(completion_stream_response(
            required_engine(&state)?,
            state.model_id().to_owned(),
            prompt.to_owned(),
            max_tokens,
            request.stream_include_usage(),
            permit,
        ));
    }
    let generated = generate_texts(
        state.engine(),
        request.prompts().to_vec(),
        max_tokens,
        permit,
    )
    .await?;
    Ok(Json(CompletionResponse::from_generations(
        state.model_id().to_owned(),
        generated,
    ))
    .into_response())
}

async fn method_not_allowed() -> OpenAiHttpError {
    OpenAiHttpError::method_not_allowed()
}

async fn not_found(OriginalUri(uri): OriginalUri) -> OpenAiHttpError {
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

fn ensure_authorized(state: &ServerState, headers: &HeaderMap) -> Result<(), OpenAiHttpError> {
    let Some(api_key) = state.api_key() else {
        return Ok(());
    };
    let Some(header) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Err(OpenAiHttpError::authentication_required(
            "missing Authorization bearer token",
        ));
    };
    let Ok(header) = header.to_str() else {
        return Err(OpenAiHttpError::authentication_required(
            "invalid Authorization bearer token",
        ));
    };
    if bearer_token_matches(header, api_key) {
        Ok(())
    } else {
        Err(OpenAiHttpError::authentication_required(
            "invalid Authorization bearer token",
        ))
    }
}

fn bearer_token_matches(header: &str, api_key: &str) -> bool {
    let Some((scheme, token)) = header.split_once(' ') else {
        return false;
    };
    scheme.eq_ignore_ascii_case("Bearer") && token == api_key
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
