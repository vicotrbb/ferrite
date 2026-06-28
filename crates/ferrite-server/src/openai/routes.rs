use super::{
    error::OpenAiHttpError,
    prompt::render_chat_prompt,
    schema::{
        ChatCompletionRequest, CompletionRequest, HealthResponse, ModelObject, ModelsResponse,
    },
};
use crate::state::ServerState;
use axum::{extract::State, routing::get, routing::post, Json, Router};

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
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<()>, OpenAiHttpError> {
    if request.stream() {
        return Err(OpenAiHttpError::not_implemented(
            "chat completion streaming is not implemented yet",
        ));
    }
    let _prompt = render_chat_prompt(request.messages())?;
    Err(OpenAiHttpError::not_implemented(
        "chat completion generation is not wired to Ferrite inference yet",
    ))
}

async fn completions(Json(request): Json<CompletionRequest>) -> Result<Json<()>, OpenAiHttpError> {
    if request.stream() {
        return Err(OpenAiHttpError::not_implemented(
            "completion streaming is not implemented yet",
        ));
    }
    if request.prompt().trim().is_empty() {
        return Err(OpenAiHttpError::invalid_request(
            "prompt must contain non-whitespace text",
        ));
    }
    Err(OpenAiHttpError::not_implemented(
        "completion generation is not wired to Ferrite inference yet",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_reports_ready_model() -> Result<(), Box<dyn std::error::Error>> {
        let app = router(ServerState::new("test-model".to_owned()));
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_json(response.into_body()).await?;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["model"], "test-model");
        assert_eq!(body["ready"], true);
        Ok(())
    }

    #[tokio::test]
    async fn models_endpoint_returns_openai_list_shape() -> Result<(), Box<dyn std::error::Error>> {
        let app = router(ServerState::new("test-model".to_owned()));
        let response = app
            .oneshot(Request::builder().uri("/v1/models").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_json(response.into_body()).await?;
        assert_eq!(body["object"], "list");
        assert_eq!(body["data"][0]["id"], "test-model");
        assert_eq!(body["data"][0]["object"], "model");
        Ok(())
    }

    #[tokio::test]
    async fn chat_endpoint_returns_openai_error_until_generation_is_wired(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let app = router(ServerState::new("test-model".to_owned()));
        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"model":"test-model","messages":[{"role":"user","content":"Hello"}]}"#,
            ))?;
        let response = app.oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = to_json(response.into_body()).await?;
        assert_eq!(body["error"]["type"], "server_error");
        Ok(())
    }

    async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
        let bytes = to_bytes(body, usize::MAX).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
