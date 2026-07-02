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
    assert_eq!(config.rss_pid(), None);
    assert_eq!(config.stop(), None);
    assert_eq!(config.assistant_context(), None);
    assert_eq!(config.follow_up(), None);
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
fn builds_openai_compatible_completion_stop_request_body() -> Result<(), Box<dyn std::error::Error>>
{
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("measure this"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
        OsString::from("--stop"),
        OsString::from("###"),
    ])?;

    assert_eq!(
        completion_request_body(&config),
        r####"{"model":"fixture-model","prompt":"measure this","max_tokens":2,"stop":"###"}"####
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
fn builds_openai_compatible_chat_stop_request_body() -> Result<(), Box<dyn std::error::Error>> {
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
        OsString::from("--stop"),
        OsString::from("###"),
    ])?;

    assert_eq!(
        request_body(&config),
        r####"{"model":"fixture-model","messages":[{"role":"user","content":"measure this"}],"max_tokens":2,"stop":"###"}"####
    );
    Ok(())
}

#[test]
fn builds_openai_compatible_second_turn_chat_request_body() -> Result<(), Box<dyn std::error::Error>>
{
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--model"),
        OsString::from("fixture-model"),
        OsString::from("--prompt"),
        OsString::from("first question"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
        OsString::from("--follow-up"),
        OsString::from("second question"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
    ])?;

    assert_eq!(
        request_body(&config),
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"first question"},{"role":"assistant","content":"first answer"},{"role":"user","content":"second question"}],"max_tokens":2}"#
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
        streaming_finish: None,
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: None,
        rss: None,
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
fn parses_rss_sampling_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--rss-pid"),
        OsString::from("1234"),
        OsString::from("--rss-idle-ms"),
        OsString::from("1500"),
    ])?;

    assert_eq!(config.rss_pid(), Some(1234));
    assert_eq!(config.rss_idle_delay(), Duration::from_millis(1500));
    Ok(())
}

#[test]
fn rejects_zero_rss_idle_delay() {
    let result = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--rss-pid"),
        OsString::from("1234"),
        OsString::from("--rss-idle-ms"),
        OsString::from("0"),
    ]);

    assert!(result.is_err());
}

#[test]
fn parses_stop_sequence_benchmark_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stop"),
        OsString::from("###"),
    ])?;

    assert_eq!(config.stop(), Some("###"));
    Ok(())
}

#[test]
fn parses_second_turn_chat_context_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
        OsString::from("--follow-up"),
        OsString::from("second question"),
    ])?;

    assert_eq!(config.assistant_context(), Some("first answer"));
    assert_eq!(config.follow_up(), Some("second question"));
    Ok(())
}

#[test]
fn rejects_partial_second_turn_chat_context() {
    let result = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
    ]);

    assert!(result.is_err());
}

#[test]
fn rejects_second_turn_context_for_legacy_completions() {
    let result = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
        OsString::from("--follow-up"),
        OsString::from("second question"),
    ]);

    assert!(result.is_err());
}

#[test]
fn rejects_empty_stop_sequence() {
    let result = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stop"),
        OsString::from(""),
    ]);

    assert!(result.is_err());
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
        streaming_finish: None,
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: None,
        rss: None,
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
        streaming_finish: None,
        streaming_timing: StreamingTimingSummary::from_event_offsets(&[
            Duration::from_millis(100),
            Duration::from_millis(140),
            Duration::from_millis(170),
        ]),
        streaming_text: None,
        streaming_usage: None,
        rss: None,
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
        streaming_finish: None,
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: Some(StreamingUsageSummary::new(8, 32, 40).with_cached_prompt_tokens(5)),
        rss: None,
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_streaming_chat_completion_requests=1\nelapsed_ms=400\nrequests_per_second=2.500000\nstreaming_usage_prompt_tokens=8\nstreaming_usage_cached_prompt_tokens=5\nstreaming_usage_completion_tokens=32\nstreaming_usage_total_tokens=40"
    );
    Ok(())
}

#[test]
fn formats_streaming_finish_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--endpoint"),
        OsString::from("chat-completions"),
        OsString::from("--stream"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_finish: Some(StreamingFinishSummary::new("length")),
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: None,
        rss: None,
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_streaming_chat_completion_requests=1\nelapsed_ms=400\nrequests_per_second=2.500000\nstreaming_finish_reason=length"
    );
    Ok(())
}

#[test]
fn formats_rss_sampling_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--rss-pid"),
        OsString::from("1234"),
    ])?;
    let result = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_finish: None,
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: None,
        rss: Some(RssSummary::new(1000, 2000, 1500)),
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_completion_requests=1\nelapsed_ms=400\nrequests_per_second=2.500000\nserver_rss_before_bytes=1000\nserver_rss_after_bytes=2000\nserver_rss_idle_bytes=1500"
    );
    Ok(())
}

#[test]
fn parses_ps_rss_kib_output_as_bytes() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(RssSummary::parse_ps_rss_bytes("  2048\n")?, 2_097_152);
    Ok(())
}

#[test]
fn extracts_streaming_finish_reason_from_sse_body() -> Result<(), Box<dyn std::error::Error>> {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"A\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n",
    );

    let finish =
        StreamingFinishSummary::from_sse_body(body).ok_or("expected streaming finish reason")?;

    assert_eq!(finish.reason(), "length");
    Ok(())
}

#[test]
fn extracts_streaming_usage_from_sse_body() -> Result<(), Box<dyn std::error::Error>> {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"A\"}}],\"usage\":null}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":8,\"prompt_tokens_details\":{\"cached_tokens\":5,\"audio_tokens\":0},\"completion_tokens\":32,\"total_tokens\":40}}\n\n",
        "data: [DONE]\n\n",
    );

    let usage = StreamingUsageSummary::from_sse_body(body).ok_or("expected streaming usage")?;

    assert_eq!(usage.prompt_tokens(), 8);
    assert_eq!(usage.cached_prompt_tokens(), 5);
    assert_eq!(usage.completion_tokens(), 32);
    assert_eq!(usage.total_tokens(), 40);
    Ok(())
}

#[test]
fn extracts_streaming_text_from_chat_and_completion_sse_bodies(
) -> Result<(), Box<dyn std::error::Error>> {
    let chat_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let completion_body = concat!(
        "data: {\"choices\":[{\"text\":\"Hello\",\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"text\":\" world\",\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"text\":\"\",\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n",
    );

    let chat_text =
        StreamingTextSummary::from_sse_body(chat_body).ok_or("expected chat streaming text")?;
    let completion_text = StreamingTextSummary::from_sse_body(completion_body)
        .ok_or("expected completion streaming text")?;

    assert_eq!(chat_text.text(), "Hello world");
    assert_eq!(completion_text.text(), "Hello world");
    Ok(())
}

#[test]
fn accepts_length_streaming_usage_matching_requested_max_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
        OsString::from("--max-tokens"),
        OsString::from("32"),
    ])?;

    validate_streaming_token_count(
        &config,
        Some(&StreamingFinishSummary::new("length")),
        Some(StreamingUsageSummary::new(8, 32, 40)),
    )?;
    Ok(())
}

#[test]
fn rejects_length_streaming_usage_below_requested_max_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
        OsString::from("--max-tokens"),
        OsString::from("32"),
    ])?;

    let result = validate_streaming_token_count(
        &config,
        Some(&StreamingFinishSummary::new("length")),
        Some(StreamingUsageSummary::new(8, 31, 39)),
    );
    let error = match result {
        Ok(()) => return Err("expected streaming token-count validation error".into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("completion_tokens"), "{error}");
    assert!(error.to_string().contains("max_tokens"), "{error}");
    Ok(())
}

#[test]
fn accepts_stop_streaming_usage_below_requested_max_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
        OsString::from("--max-tokens"),
        OsString::from("32"),
    ])?;

    validate_streaming_token_count(
        &config,
        Some(&StreamingFinishSummary::new("stop")),
        Some(StreamingUsageSummary::new(8, 4, 12)),
    )?;
    Ok(())
}

#[test]
fn validates_streaming_response_done_event() -> Result<(), Box<dyn std::error::Error>> {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\",\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n";

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
fn rejects_streaming_response_without_finish_reason() -> Result<(), Box<dyn std::error::Error>> {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\"}]}\n\ndata: [DONE]\n\n";

    let error = validate_stream_error(response, false)?;

    assert!(
        error
            .to_string()
            .contains("missing streaming finish_reason"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_streaming_usage_response_without_usage_chunk() -> Result<(), Box<dyn std::error::Error>>
{
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"text\":\"hi\",\"finish_reason\":\"length\"}],\"usage\":null}\n\ndata: [DONE]\n\n";

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
    assert_eq!(
        summary.stream_observed_prefill_elapsed(),
        Duration::from_millis(80)
    );
    assert_eq!(summary.first_token_timestamp(), Duration::from_millis(80));
    assert_eq!(summary.stream_observed_decode_elapsed(), Duration::ZERO);
    assert_eq!(summary.stream_observed_decode_tokens_per_second(), 0.0);
    assert_eq!(summary.total_elapsed(), Duration::from_millis(80));
    Ok(())
}

#[test]
fn derives_streaming_timing_from_terminal_stop_event_without_visible_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let base = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
    let stop_event = r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#;
    let done_event = "data: [DONE]";
    let snapshots = [
        (base.to_owned(), Duration::from_millis(10)),
        (
            format!("{base}{stop_event}\n\n{done_event}\n\n"),
            Duration::from_millis(90),
        ),
    ];

    let summary = http::streaming_timing_from_response_snapshots(
        snapshots
            .iter()
            .map(|(response, offset)| (response.as_bytes(), *offset)),
    )
    .ok_or("expected streaming timing summary")?;

    assert_eq!(summary.token_events(), 1);
    assert_eq!(summary.time_to_first_token(), Duration::from_millis(90));
    assert_eq!(summary.total_elapsed(), Duration::from_millis(90));
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
    assert_eq!(
        summary.stream_observed_prefill_elapsed(),
        Duration::from_millis(100)
    );
    assert_eq!(summary.first_token_timestamp(), Duration::from_millis(100));
    assert_eq!(
        summary.stream_observed_decode_elapsed(),
        Duration::from_millis(160)
    );
    assert_eq!(summary.total_elapsed(), Duration::from_millis(260));
    assert_eq!(summary.min_token_latency(), Duration::from_millis(30));
    assert_eq!(summary.p50_token_latency(), Duration::from_millis(40));
    assert_eq!(summary.p95_token_latency(), Duration::from_millis(100));
    assert_eq!(summary.max_token_latency(), Duration::from_millis(100));
    assert!((summary.tokens_per_second() - 15.384615).abs() < 0.000001);
    assert!((summary.stream_observed_decode_tokens_per_second() - 18.75).abs() < 0.000001);
    Ok(())
}
