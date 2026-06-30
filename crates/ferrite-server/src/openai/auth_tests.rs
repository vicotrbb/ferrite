use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn protected_openai_routes_require_matching_bearer_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = get_models(None).await?;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.json["error"]["type"], "authentication_error");

    let response = get_models(Some("Bearer wrong")).await?;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.json["error"]["type"], "authentication_error");

    let response = get_models(Some("Bearer local-secret")).await?;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json["object"], "list");
    Ok(())
}

#[tokio::test]
async fn protected_openai_routes_accept_case_insensitive_bearer_scheme(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = get_models(Some("bearer local-secret")).await?;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json["object"], "list");
    Ok(())
}

#[tokio::test]
async fn protected_openai_routes_accept_repeated_bearer_separator_spaces(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = get_models(Some("Bearer   local-secret")).await?;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json["object"], "list");
    Ok(())
}

#[tokio::test]
async fn unknown_openai_routes_require_matching_bearer_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/responses")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "authentication_error");
    Ok(())
}

#[tokio::test]
async fn wrong_method_openai_routes_require_matching_bearer_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/completions")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "authentication_error");
    Ok(())
}

#[tokio::test]
async fn protected_generation_routes_authenticate_before_json_extraction(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"model":"fixture-model","messages":"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "authentication_error");
    Ok(())
}

#[tokio::test]
async fn health_route_does_not_require_bearer_token() -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["status"], "ok");
    Ok(())
}

async fn get_models(
    authorization: Option<&str>,
) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let mut builder = Request::builder().uri("/v1/models");
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    let response = app.oneshot(builder.body(Body::empty())?).await?;
    let status = response.status();
    let json = to_json(response.into_body()).await?;
    Ok(JsonResponse { status, json })
}

struct JsonResponse {
    status: StatusCode,
    json: Value,
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
