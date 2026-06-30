use super::*;
use std::time::Duration;
use std::{ffi::OsString, net::SocketAddr};

#[test]
fn parses_minimal_completion_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1:18080"),
        OsString::from("--model"),
        OsString::from("qwen2.5-1.5b-q8_0"),
    ])?;

    assert_eq!(config.addr(), SocketAddr::from(([127, 0, 0, 1], 18080)));
    assert_eq!(config.model(), "qwen2.5-1.5b-q8_0");
    assert_eq!(config.prompt(), "hello world");
    assert_eq!(config.requests(), 3);
    assert_eq!(config.concurrency(), 1);
    assert_eq!(config.max_tokens(), 1);
    assert_eq!(config.api_key(), "local-secret");
    assert!(!config.stream());
    Ok(())
}

#[test]
fn builds_openai_compatible_completion_request_body() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1:18080"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        completion_request_body(&config),
        r#"{"model":"fixture-model","prompt":"measure this","max_tokens":2}"#
    );
    Ok(())
}

#[test]
fn parses_chat_completion_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
    ])?;

    assert_eq!(config.endpoint(), OpenAiEndpoint::ChatCompletions);
    Ok(())
}

#[test]
fn builds_openai_compatible_chat_completion_request_body() -> Result<(), Box<dyn std::error::Error>>
{
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"measure this"}],"max_tokens":2}"#
    );
    Ok(())
}

#[test]
fn formats_chat_completion_result_metric_name() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 2,
        elapsed: std::time::Duration::from_millis(400),
        streaming_timing: None,
        streaming_usage: None,
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_chat_completion_requests=2\nelapsed_ms=400\nrequests_per_second=5.000000"
    );
    Ok(())
}

#[test]
fn parses_streaming_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
    ])?;

    assert!(config.stream());
    Ok(())
}

#[test]
fn parses_stream_usage_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
    ])?;

    assert!(config.stream());
    assert!(config.stream_usage());
    Ok(())
}

#[test]
fn rejects_stream_usage_without_streaming() -> Result<(), Box<dyn std::error::Error>> {
    let result = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream-usage"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("--stream-usage requires --stream"),
        "{error}"
    );
    Ok(())
}

#[test]
fn builds_openai_compatible_streaming_completion_request_body(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","prompt":"measure this","max_tokens":2,"stream":true}"#
    );
    Ok(())
}

#[test]
fn builds_openai_compatible_streaming_completion_usage_request_body(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","prompt":"measure this","max_tokens":2,"stream":true,"stream_options":{"include_usage":true}}"#
    );
    Ok(())
}

#[test]
fn builds_openai_compatible_streaming_chat_completion_request_body(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"measure this"}],"max_tokens":2,"stream":true}"#
    );
    Ok(())
}

#[test]
fn builds_openai_compatible_streaming_chat_usage_request_body(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"measure this"}],"max_tokens":2,"stream":true,"stream_options":{"include_usage":true}}"#
    );
    Ok(())
}

#[test]
fn formats_streaming_chat_completion_result_metric_name() -> Result<(), Box<dyn std::error::Error>>
{
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 2,
        elapsed: std::time::Duration::from_millis(400),
        streaming_timing: None,
        streaming_usage: None,
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_streaming_chat_completion_requests=2\nelapsed_ms=400\nrequests_per_second=5.000000"
    );
    Ok(())
}

#[test]
fn formats_streaming_timing_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_timing: StreamingTimingSummary::from_event_offsets(&[
            Duration::from_millis(100),
            Duration::from_millis(140),
            Duration::from_millis(170),
        ]),
        streaming_usage: None,
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_streaming_chat_completion_requests=1\nelapsed_ms=400\nrequests_per_second=2.500000\nstreaming_token_events=3\nstreaming_time_to_first_token_ms=100\nstreaming_total_elapsed_ms=170\nstreaming_tokens_per_second=17.647059\nstreaming_token_latency_min_ms=30\nstreaming_token_latency_p50_ms=40\nstreaming_token_latency_p95_ms=100\nstreaming_token_latency_max_ms=100"
    );
    Ok(())
}

#[test]
fn formats_streaming_usage_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_timing: None,
        streaming_usage: Some(StreamingUsageSummary::new(8, 32, 40)),
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_streaming_chat_completion_requests=1\nelapsed_ms=400\nrequests_per_second=2.500000\nstreaming_usage_prompt_tokens=8\nstreaming_usage_completion_tokens=32\nstreaming_usage_total_tokens=40"
    );
    Ok(())
}

#[test]
fn extracts_streaming_usage_from_sse_body() -> Result<(), Box<dyn std::error::Error>> {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"A\"}}],\"usage\":null}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":32,\"total_tokens\":40}}\n\n",
        "data: [DONE]\n\n",
    );

    let usage = StreamingUsageSummary::from_sse_body(body).ok_or("expected streaming usage")?;

    assert_eq!(usage.prompt_tokens(), 8);
    assert_eq!(usage.completion_tokens(), 32);
    assert_eq!(usage.total_tokens(), 40);
    Ok(())
}

#[test]
fn validates_streaming_response_done_event() -> Result<(), Box<dyn std::error::Error>> {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\"}]}\n\ndata: [DONE]\n\n";

    http::validate_openai_response(OpenAiEndpoint::Completions, true, false, response)?;
    Ok(())
}

#[test]
fn rejects_streaming_response_without_sse_content_type() -> Result<(), Box<dyn std::error::Error>> {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\"}]}\n\ndata: [DONE]\n\n";

    let error = validate_stream_error(response, false)?;

    assert!(error.to_string().contains("text/event-stream"), "{error}");
    Ok(())
}

#[test]
fn rejects_streaming_response_with_duplicate_done_events() -> Result<(), Box<dyn std::error::Error>>
{
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\"}]}\n\ndata: [DONE]\n\ndata: [DONE]\n\n";

    let error = validate_stream_error(response, false)?;

    assert!(
        error.to_string().contains("exactly one streaming done"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_streaming_response_without_json_data_chunk() -> Result<(), Box<dyn std::error::Error>> {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: [DONE]\n\n";

    let error = validate_stream_error(response, false)?;

    assert!(
        error
            .to_string()
            .contains("missing streaming JSON data event"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_streaming_usage_response_without_usage_chunk() -> Result<(), Box<dyn std::error::Error>>
{
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\"}],\"usage\":null}\n\ndata: [DONE]\n\n";

    let error = validate_stream_error(response, true)?;

    assert!(
        error.to_string().contains("missing streaming usage"),
        "{error}"
    );
    Ok(())
}

fn validate_stream_error(
    response: &str,
    expect_stream_usage: bool,
) -> Result<Box<dyn std::error::Error>, Box<dyn std::error::Error>> {
    match http::validate_openai_response(
        OpenAiEndpoint::Completions,
        true,
        expect_stream_usage,
        response,
    ) {
        Ok(()) => Err("expected streaming validation error".into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn derives_streaming_timing_from_incremental_response_snapshots(
) -> Result<(), Box<dyn std::error::Error>> {
    let base = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
    let role_event = r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;
    let first_token_event = r#"data: {"choices":[{"delta":{"content":"A"}}]}"#;
    let second_token_event = r#"data: {"choices":[{"delta":{"content":"B"}}]}"#;
    let third_token_event = r#"data: {"choices":[{"text":"C"}]}"#;
    let done_event = "data: [DONE]";
    let snapshots = [
        (base.to_owned(), Duration::from_millis(10)),
        (format!("{base}{role_event}\n\n"), Duration::from_millis(20)),
        (
            format!("{base}{role_event}\n\n{first_token_event}\n\n"),
            Duration::from_millis(50),
        ),
        (
            format!("{base}{role_event}\n\n{first_token_event}\n\n{second_token_event}\n\n"),
            Duration::from_millis(80),
        ),
        (
            format!(
                "{base}{role_event}\n\n{first_token_event}\n\n{second_token_event}\n\n{third_token_event}\n\n{done_event}\n\n"
            ),
            Duration::from_millis(140),
        ),
    ];

    let summary = http::streaming_timing_from_response_snapshots(
        snapshots
            .iter()
            .map(|(response, offset)| (response.as_bytes(), *offset)),
    )
    .ok_or("expected streaming timing summary")?;

    assert_eq!(summary.token_events(), 3);
    assert_eq!(summary.time_to_first_token(), Duration::from_millis(50));
    assert_eq!(summary.total_elapsed(), Duration::from_millis(140));
    assert_eq!(summary.min_token_latency(), Duration::from_millis(30));
    assert_eq!(summary.p50_token_latency(), Duration::from_millis(50));
    assert_eq!(summary.p95_token_latency(), Duration::from_millis(60));
    assert_eq!(summary.max_token_latency(), Duration::from_millis(60));
    Ok(())
}

#[test]
fn waits_for_completed_sse_event_before_recording_streaming_timing(
) -> Result<(), Box<dyn std::error::Error>> {
    let base = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
    let token_event = r#"data: {"choices":[{"delta":{"content":"A"}}]}"#;
    let snapshots = [
        (base.to_owned(), Duration::from_millis(10)),
        (format!("{base}{token_event}"), Duration::from_millis(50)),
        (
            format!("{base}{token_event}\n\n"),
            Duration::from_millis(80),
        ),
    ];

    let summary = http::streaming_timing_from_response_snapshots(
        snapshots
            .iter()
            .map(|(response, offset)| (response.as_bytes(), *offset)),
    )
    .ok_or("expected streaming timing summary")?;

    assert_eq!(summary.token_events(), 1);
    assert_eq!(summary.time_to_first_token(), Duration::from_millis(80));
    assert_eq!(summary.total_elapsed(), Duration::from_millis(80));
    Ok(())
}

#[test]
fn summarizes_streaming_token_arrival_latencies() -> Result<(), Box<dyn std::error::Error>> {
    let summary = StreamingTimingSummary::from_event_offsets(&[
        Duration::from_millis(100),
        Duration::from_millis(140),
        Duration::from_millis(170),
        Duration::from_millis(260),
    ])
    .ok_or("expected timing summary")?;

    assert_eq!(summary.token_events(), 4);
    assert_eq!(summary.time_to_first_token(), Duration::from_millis(100));
    assert_eq!(summary.total_elapsed(), Duration::from_millis(260));
    assert_eq!(summary.min_token_latency(), Duration::from_millis(30));
    assert_eq!(summary.p50_token_latency(), Duration::from_millis(40));
    assert_eq!(summary.p95_token_latency(), Duration::from_millis(100));
    assert_eq!(summary.max_token_latency(), Duration::from_millis(100));
    assert!((summary.tokens_per_second() - 15.384615).abs() < 0.000001);
    Ok(())
}
