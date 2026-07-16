use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

#[tokio::test]
async fn openai_cors_preflight_does_not_require_bearer_token()
-> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/v1/chat/completions")
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "authorization, content-type",
                )
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let headers = response.headers();
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert!(
        headers[header::ACCESS_CONTROL_ALLOW_METHODS]
            .to_str()?
            .contains("POST")
    );
    assert!(
        headers[header::ACCESS_CONTROL_ALLOW_HEADERS]
            .to_str()?
            .contains("authorization")
    );
    Ok(())
}

#[tokio::test]
async fn openai_model_retrieve_preflight_supports_provider_style_model_ids()
-> Result<(), Box<dyn std::error::Error>> {
    for path in [
        "/v1/models/Qwen/Qwen2.5-0.5B-Instruct-Q4_K_M",
        "/v1/models/Qwen%2FQwen2.5-0.5B-Instruct-Q4_K_M",
    ] {
        let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri(path)
                    .header(header::ORIGIN, "http://localhost:3000")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .header(
                        header::ACCESS_CONTROL_REQUEST_HEADERS,
                        "authorization, content-type",
                    )
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NO_CONTENT, "{path}");
        let headers = response.headers();
        assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
        assert!(
            headers[header::ACCESS_CONTROL_ALLOW_METHODS]
                .to_str()?
                .contains("GET")
        );
        assert!(
            headers[header::ACCESS_CONTROL_ALLOW_HEADERS]
                .to_str()?
                .contains("authorization")
        );
    }
    Ok(())
}

#[tokio::test]
async fn protected_openai_routes_include_cors_response_header()
-> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()).with_api_key("local-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::AUTHORIZATION, "Bearer local-secret")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    Ok(())
}
