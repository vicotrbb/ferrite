use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{error::Error, fmt};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpenAiErrorBody {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<String>,
    code: Option<String>,
}

impl OpenAiErrorBody {
    pub fn new(message: impl Into<String>, error_type: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: error_type.into(),
            param: None,
            code: None,
        }
    }

    pub fn with_code(
        message: impl Into<String>,
        error_type: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            error_type: error_type.into(),
            param: None,
            code: Some(code.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpenAiErrorResponse {
    error: OpenAiErrorBody,
}

impl OpenAiErrorResponse {
    pub fn new(error: OpenAiErrorBody) -> Self {
        Self { error }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiHttpError {
    status: StatusCode,
    body: OpenAiErrorResponse,
}

impl OpenAiHttpError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::new(message, "invalid_request_error")),
        }
    }

    pub fn model_not_found(model: impl AsRef<str>) -> Self {
        let model = model.as_ref();
        Self {
            status: StatusCode::NOT_FOUND,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::with_code(
                format!("model {model} is not loaded"),
                "invalid_request_error",
                "model_not_found",
            )),
        }
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::new(message, "server_error")),
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::new(message, "server_error")),
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::new(message, "rate_limit_error")),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: OpenAiErrorResponse::new(OpenAiErrorBody::new(message, "server_error")),
        }
    }
}

impl IntoResponse for OpenAiHttpError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl fmt::Display for OpenAiHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.body.error.message)
    }
}

impl Error for OpenAiHttpError {}
