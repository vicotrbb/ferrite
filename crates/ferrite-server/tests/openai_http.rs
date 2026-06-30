mod support;

use support::http::{response_json, send_http_request, send_http_request_with_bearer};

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
