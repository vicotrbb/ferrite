use super::{
    error::OpenAiHttpError,
    schema::{ChatCompletionRequest, CompletionRequest},
};
use crate::{limits::TokenLimitError, state::ServerState};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

pub(super) fn ensure_supported_completion_request(
    request: &CompletionRequest,
) -> Result<(), OpenAiHttpError> {
    let unsupported = request.unsupported_fields();
    if unsupported.is_empty() {
        return Ok(());
    }

    Err(unsupported_field_error(
        "unsupported completion field(s)",
        &unsupported,
    ))
}

pub(super) fn ensure_supported_chat_request(
    request: &ChatCompletionRequest,
) -> Result<(), OpenAiHttpError> {
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

pub(super) fn ensure_model(
    state: &ServerState,
    requested_model: &str,
) -> Result<(), OpenAiHttpError> {
    if requested_model.is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "model is required",
            "model",
        ));
    }
    if requested_model == state.model_id() {
        Ok(())
    } else {
        Err(OpenAiHttpError::model_not_found(requested_model))
    }
}

pub(super) fn required_engine(
    state: &ServerState,
) -> Result<Arc<crate::runtime::InferenceEngine>, OpenAiHttpError> {
    state.engine().ok_or_else(|| {
        OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        )
    })
}

pub(super) async fn acquire_inference_permit(
    state: &ServerState,
) -> Result<OwnedSemaphorePermit, OpenAiHttpError> {
    state.acquire_inference_permit().await.ok_or_else(|| {
        OpenAiHttpError::rate_limited("inference request queue is full; retry later")
    })
}

pub(super) async fn acquire_batch_admission_permit(
    state: &ServerState,
) -> Result<OwnedSemaphorePermit, OpenAiHttpError> {
    state.acquire_batch_admission_permit().await.ok_or_else(|| {
        OpenAiHttpError::rate_limited("batched inference queue is full; retry later")
    })
}

pub(super) fn normalized_max_tokens(
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
