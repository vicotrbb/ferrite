mod support;

use ferrite_server::limits::TokenLimits;
use std::time::{Duration, Instant};
use support::http::{
    abort_http_stream_after_marker, response_json, send_http_request, send_http_request_with_bearer,
};

#[tokio::test]
async fn live_http_server_accepts_openai_style_model_list() -> Result<(), Box<dyn std::error::Error>>
{
    let server = support::LiveServer::start().await?;
    let response = send_http_request(server.addr(), "GET", "/v1/models", &[]).await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["id"], support::MODEL_ID);
    assert_eq!(body["data"][0]["object"], "model");
    assert_eq!(body["data"][0]["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn live_http_server_accepts_openai_style_model_retrieve(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let path = format!("/v1/models/{}", support::MODEL_ID);
    let response = send_http_request(server.addr(), "GET", &path, &[]).await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["id"], support::MODEL_ID);
    assert_eq!(body["object"], "model");
    assert_eq!(body["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn live_http_server_retrieves_encoded_slash_model_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "HuggingFaceTB/SmolLM2-135M-Instruct";
    let server = support::LiveServer::start_with_model_id(model_id).await?;
    let response = send_http_request(
        server.addr(),
        "GET",
        "/v1/models/HuggingFaceTB%2FSmolLM2-135M-Instruct",
        &[],
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["id"], model_id);
    assert_eq!(body["object"], "model");
    assert_eq!(body["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn live_http_server_accepts_openai_style_chat_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":1}}"#,
        support::MODEL_ID
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], support::MODEL_ID);
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn live_http_server_accepts_openai_style_legacy_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","prompt":"hello","max_tokens":1}}"#,
        support::MODEL_ID
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], support::MODEL_ID);
    assert_eq!(body["choices"][0]["text"], "winner");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn live_http_server_streams_openai_style_chat_chunks(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":1,"stream":true}}"#,
        support::MODEL_ID
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "unexpected response headers: {response}"
    );
    assert!(response.contains("data: {\"id\":\"chatcmpl-ferrite-"));
    assert!(response.contains("\"object\":\"chat.completion.chunk\""));
    assert!(response.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(response.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn live_http_server_releases_inference_permit_after_streaming_tcp_disconnect(
) -> Result<(), Box<dyn std::error::Error>> {
    let token_limits = TokenLimits::new(16, 4096)?;
    let server =
        support::LiveServer::start_configured(|state| state.with_token_limits(token_limits))
            .await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":4096,"stream":true}}"#,
        support::MODEL_ID
    );

    let partial_response = abort_http_stream_after_marker(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
        "\"delta\":{\"content\":\"",
    )
    .await?;
    assert!(
        partial_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {partial_response}"
    );
    assert!(server.state().try_acquire_inference_permit().is_none());

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if server.state().try_acquire_inference_permit().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(
                "streaming TCP disconnect kept the inference permit after generated content".into(),
            );
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Ok(())
}

#[tokio::test]
async fn live_http_server_releases_inference_permit_after_tcp_disconnect_before_generated_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let token_limits = TokenLimits::new(16, 4096)?;
    let server =
        support::LiveServer::start_configured(|state| state.with_token_limits(token_limits))
            .await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":4096,"stream":true}}"#,
        support::MODEL_ID
    );

    let partial_response = abort_http_stream_after_marker(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
        "\"delta\":{\"role\":\"assistant\",\"content\":\"\"}",
    )
    .await?;
    assert!(
        partial_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {partial_response}"
    );
    assert!(
        !partial_response.contains("\"delta\":{\"content\":\""),
        "test must disconnect before generated content: {partial_response}"
    );
    assert!(server.state().try_acquire_inference_permit().is_none());

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if server.state().try_acquire_inference_permit().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(
                "streaming TCP disconnect kept the inference permit before generated content"
                    .into(),
            );
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Ok(())
}

#[tokio::test]
async fn live_http_server_streams_openai_style_legacy_completion_chunks(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","prompt":"hello","max_tokens":1,"stream":true}}"#,
        support::MODEL_ID
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "unexpected response headers: {response}"
    );
    assert!(response.contains("data: {\"id\":\"cmpl-ferrite-"));
    assert!(response.contains("\"object\":\"text_completion\""));
    assert!(response.contains("\"text\":\"winner\""));
    assert!(response.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn live_http_server_accepts_matching_bearer_token() -> Result<(), Box<dyn std::error::Error>>
{
    let server = support::LiveServer::start_with_api_key("local-secret").await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":1}}"#,
        support::MODEL_ID
    );
    let response = send_http_request_with_bearer(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
        "local-secret",
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}
