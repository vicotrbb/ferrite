use super::{
    routes::router,
    test_support::{post_responses_json, remove_fixture_model, to_json, write_chat_fixture_model},
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn responses_endpoint_generates_standard_non_streaming_text_shape(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{
                        "model":"fixture-model",
                        "input":"hello",
                        "max_output_tokens":1,
                        "metadata":{"trace_id":"local-1"}
                    }"#,
                ))?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("resp-")));
    assert_eq!(body["object"], "response");
    assert_eq!(body["status"], "incomplete");
    assert_eq!(body["incomplete_details"]["reason"], "max_output_tokens");
    assert!(body["instructions"].is_null());
    assert_eq!(body["output"][0]["type"], "message");
    assert_eq!(body["output"][0]["role"], "assistant");
    assert_eq!(body["output"][0]["content"][0]["type"], "output_text");
    assert_eq!(body["output"][0]["content"][0]["text"], "winner");
    assert_eq!(body["usage"]["output_tokens"], 1);
    assert_eq!(
        body["usage"]["output_tokens_details"]["reasoning_tokens"],
        0
    );
    assert_eq!(body["metadata"]["trace_id"], "local-1");
    assert_eq!(body["tools"], serde_json::json!([]));
    Ok(())
}

#[tokio::test]
async fn responses_endpoint_accepts_bounded_message_arrays(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{
                        "model":"fixture-model",
                        "input":[
                            {"role":"user","content":[{"type":"input_text","text":"hello"}]}
                        ],
                        "max_output_tokens":1
                    }"#,
                ))?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["output"][0]["content"][0]["text"], "winner");
    Ok(())
}

#[tokio::test]
async fn responses_endpoint_reports_cached_input_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine)
        .with_prefix_cache_enabled(true)
        .with_batched_decode(2)?;
    let app = router(state);
    let payload = r#"{
        "model":"fixture-model",
        "input":"hello",
        "prompt_cache_key":"tenant-a:response-1",
        "max_output_tokens":1
    }"#;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))?,
        )
        .await?;
    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    let first_body = to_json(first.into_body()).await?;
    let second_status = second.status();
    let second_body = to_json(second.into_body()).await?;
    assert_eq!(
        first_body["usage"]["input_tokens_details"]["cached_tokens"],
        0
    );
    assert_eq!(second_status, StatusCode::OK, "{second_body}");
    assert!(
        second_body["usage"]["input_tokens_details"]["cached_tokens"]
            .as_u64()
            .is_some_and(|tokens| tokens > 0)
    );
    Ok(())
}

#[tokio::test]
async fn responses_endpoint_rejects_streaming_and_stateful_or_multimodal_input(
) -> Result<(), Box<dyn std::error::Error>> {
    let streaming =
        post_responses_json(r#"{"model":"fixture-model","input":"hello","stream":true}"#).await?;
    assert_eq!(streaming.status, StatusCode::BAD_REQUEST);
    assert_eq!(streaming.json["error"]["param"], "stream");

    let stateful = post_responses_json(
        r#"{
            "model":"fixture-model",
            "input":"hello",
            "previous_response_id":"resp_remote",
            "background":true,
            "store":true
        }"#,
    )
    .await?;
    assert_eq!(stateful.status, StatusCode::BAD_REQUEST);
    let message = stateful.json["error"]["message"]
        .as_str()
        .unwrap_or_default();
    assert!(message.contains("previous_response_id"), "{message}");
    assert!(message.contains("background"), "{message}");
    assert!(message.contains("store"), "{message}");

    let multimodal = post_responses_json(
        r#"{
            "model":"fixture-model",
            "input":[{"role":"user","content":[{"type":"input_image","image_url":"https://example.test/image.png"}]}]
        }"#,
    )
    .await?;
    assert_eq!(multimodal.status, StatusCode::BAD_REQUEST);
    assert_eq!(multimodal.json["error"]["param"], "input");
    Ok(())
}

#[tokio::test]
async fn responses_endpoint_rejects_tools_and_non_text_output_formats(
) -> Result<(), Box<dyn std::error::Error>> {
    for (payload, parameter) in [
        (
            r#"{
                "model":"fixture-model",
                "input":"hello",
                "tools":[{"type":"function","name":"lookup","parameters":{"type":"object"}}]
            }"#,
            "tools",
        ),
        (
            r#"{
                "model":"fixture-model",
                "input":"hello",
                "text":{"format":{"type":"json_object"}}
            }"#,
            "text",
        ),
    ] {
        let response = post_responses_json(payload).await?;
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert_eq!(response.json["error"]["param"], parameter);
    }
    Ok(())
}

#[tokio::test]
async fn responses_endpoint_authenticates_and_supports_cors_preflight(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new("fixture-model".to_owned()).with_api_key("local-secret");
    let unauthorized = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"model":"fixture-model","input":"hello"}"#))?,
        )
        .await?;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let preflight = router(state)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/v1/responses")
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "authorization, content-type",
                )
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        preflight.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "*"
    );
    Ok(())
}
