mod support;

use ferrite_server::throughput_client::{ThroughputClientConfig, run_completion_benchmark};
use std::{ffi::OsString, time::Duration};
use support::throughput::{
    measure_concurrent_completion_requests, measure_sequential_completion_requests,
};

#[tokio::test]
async fn live_http_server_measures_sequential_completion_request_rate()
-> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","prompt":"hello","max_tokens":1}}"#,
        support::MODEL_ID
    );

    let measurement =
        measure_sequential_completion_requests(server.addr(), request_body.as_bytes(), 5).await?;

    assert_eq!(measurement.completed_requests, 5);
    assert!(measurement.elapsed.as_nanos() > 0);
    assert!(measurement.requests_per_second().is_finite());
    assert!(measurement.requests_per_second() > 0.0);
    Ok(())
}

#[tokio::test]
async fn live_http_server_measures_queued_concurrent_completion_request_rate()
-> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start_configured(|state| {
        state.with_inference_wait_timeout(Duration::from_secs(2))
    })
    .await?;
    let request_body = format!(
        r#"{{"model":"{}","prompt":"hello","max_tokens":1}}"#,
        support::MODEL_ID
    );

    let measurement =
        measure_concurrent_completion_requests(server.addr(), request_body.as_bytes(), 3).await?;

    assert_eq!(measurement.completed_requests, 3);
    assert!(measurement.elapsed.as_nanos() > 0);
    assert!(measurement.requests_per_second().is_finite());
    assert!(measurement.requests_per_second() > 0.0);
    Ok(())
}

#[tokio::test]
async fn throughput_client_tracks_repeated_requests_per_configured_prompt()
-> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start_configured(|state| {
        state.with_inference_wait_timeout(Duration::from_secs(2))
    })
    .await?;
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--addr"),
        OsString::from(server.addr().to_string()),
        OsString::from("--model"),
        OsString::from(support::MODEL_ID),
        OsString::from("--prompt"),
        OsString::from("hello"),
        OsString::from("--prompt"),
        OsString::from("winner"),
        OsString::from("--requests"),
        OsString::from("4"),
        OsString::from("--concurrency"),
        OsString::from("2"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
    ])?;

    let result = run_completion_benchmark(&config).await?;

    assert_eq!(result.completed_requests, 4);
    let timings = result
        .streaming_timing
        .ok_or("expected streaming timing summary")?;
    assert_eq!(timings.request_count(), 4);
    let token_ids = result
        .streaming_token_ids
        .ok_or("expected streaming token IDs")?;
    assert_eq!(token_ids.prompt_token_id_traces().map(<[_]>::len), Some(2));
    assert_eq!(token_ids.all_prompt_traces_stable(), Some(true));
    Ok(())
}

#[tokio::test]
async fn throughput_client_tracks_mixed_output_budgets_per_prompt()
-> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start_configured(|state| {
        state.with_inference_wait_timeout(Duration::from_secs(2))
    })
    .await?;
    let config = ThroughputClientConfig::parse([
        OsString::from("ferrite-openai-throughput"),
        OsString::from("--addr"),
        OsString::from(server.addr().to_string()),
        OsString::from("--model"),
        OsString::from(support::MODEL_ID),
        OsString::from("--prompt"),
        OsString::from("hello"),
        OsString::from("--prompt"),
        OsString::from("winner"),
        OsString::from("--max-tokens"),
        OsString::from("1"),
        OsString::from("--max-tokens"),
        OsString::from("2"),
        OsString::from("--requests"),
        OsString::from("4"),
        OsString::from("--concurrency"),
        OsString::from("2"),
        OsString::from("--stream"),
        OsString::from("--stream-usage"),
    ])?;

    let result = run_completion_benchmark(&config).await?;

    let usage = result.streaming_usage.ok_or("expected streaming usage")?;
    assert_eq!(usage.cohort_request_count(), 4);
    assert_eq!(usage.cohort_completion_tokens(), 6);
    let token_ids = result
        .streaming_token_ids
        .ok_or("expected streaming token IDs")?;
    assert_eq!(token_ids.prompt_token_id_traces().map(<[_]>::len), Some(2));
    assert_eq!(token_ids.all_prompt_traces_stable(), Some(true));
    Ok(())
}
