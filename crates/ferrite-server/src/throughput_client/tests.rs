use super::*;
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
    };

    assert_eq!(
        format_result(&config, result),
        "openai_http_chat_completion_requests=2\nelapsed_ms=400\nrequests_per_second=5.000000"
    );
    Ok(())
}
