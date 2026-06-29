mod support;

use std::time::Duration;
use support::throughput::{
    measure_concurrent_completion_requests, measure_sequential_completion_requests,
};

#[tokio::test]
async fn live_http_server_measures_sequential_completion_request_rate(
) -> Result<(), Box<dyn std::error::Error>> {
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
async fn live_http_server_measures_queued_concurrent_completion_request_rate(
) -> Result<(), Box<dyn std::error::Error>> {
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
