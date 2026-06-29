use serde_json::Value;

use crate::support::http::response_json;

#[derive(Clone, Copy)]
pub struct StopSequenceExpectation {
    pub model_id: &'static str,
    pub completion_prompt_tokens: u64,
    pub chat_prompt_tokens: u64,
}

pub fn assert_stop_completion_response(
    response: &str,
    expectation: StopSequenceExpectation,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], expectation.model_id);
    assert_eq!(body["choices"][0]["text"], "");
    assert_eq!(
        body["usage"]["prompt_tokens"],
        expectation.completion_prompt_tokens
    );
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(
        body["usage"]["total_tokens"],
        expectation.completion_prompt_tokens + 1
    );
    Ok(())
}

pub fn assert_stop_chat_response(
    response: &str,
    expectation: StopSequenceExpectation,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], expectation.model_id);
    assert_eq!(body["choices"][0]["message"]["content"], "");
    assert_eq!(
        body["usage"]["prompt_tokens"],
        expectation.chat_prompt_tokens
    );
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(
        body["usage"]["total_tokens"],
        expectation.chat_prompt_tokens + 1
    );
    Ok(())
}

pub fn assert_stop_completion_stream(response: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert_stop_stream_headers(response);
    let events = sse_json_events(response)?;
    assert!(!events.is_empty(), "missing JSON SSE events: {response}");
    let content_chunks = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            choice["finish_reason"]
                .is_null()
                .then(|| choice["text"].as_str())?
        })
        .collect::<Vec<_>>();
    assert_eq!(content_chunks, Vec::<&str>::new());

    let stop_events = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "stop")
        .collect::<Vec<_>>();
    assert_eq!(stop_events.len(), 1);
    assert_eq!(stop_events[0]["choices"][0]["text"], "");
    Ok(())
}

pub fn assert_stop_chat_stream(response: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert_stop_stream_headers(response);
    let events = sse_json_events(response)?;
    assert!(!events.is_empty(), "missing JSON SSE events: {response}");
    let content_chunks = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null() && choice["delta"]["role"].is_null())
                .then(|| choice["delta"]["content"].as_str())?
        })
        .collect::<Vec<_>>();
    assert_eq!(content_chunks, Vec::<&str>::new());

    let stop_events = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "stop")
        .collect::<Vec<_>>();
    assert_eq!(stop_events.len(), 1);
    assert!(stop_events[0]["choices"][0]["delta"]["content"].is_null());
    Ok(())
}

fn assert_stop_stream_headers(response: &str) {
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
    assert!(
        response.contains("data: [DONE]"),
        "missing stream terminator: {response}"
    );
}

fn sse_json_events(response: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    response
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|payload| *payload != "[DONE]")
        .map(|payload| Ok(serde_json::from_str(payload)?))
        .collect()
}
