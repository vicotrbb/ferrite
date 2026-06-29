mod support;

use std::{path::PathBuf, time::Duration};

use support::throughput::{
    measure_concurrent_completion_requests_with_expectation,
    measure_sequential_completion_requests_with_expectation, CompletionResponseExpectation,
};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_measures_qwen_1_5b_q8_sequential_completion_request_rate(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);

    let measurement = measure_sequential_completion_requests_with_expectation(
        server.addr(),
        request_body.as_bytes(),
        3,
        CompletionResponseExpectation {
            model_id: REAL_MODEL_ID,
            text: "\n",
        },
    )
    .await?;

    assert_eq!(measurement.completed_requests, 3);
    assert!(measurement.elapsed.as_nanos() > 0);
    assert!(measurement.requests_per_second().is_finite());
    assert!(measurement.requests_per_second() > 0.0);
    println!(
        "qwen_1_5b_q8_sequential_http_completion_requests={} elapsed_ms={} requests_per_second={:.6}",
        measurement.completed_requests,
        measurement.elapsed.as_millis(),
        measurement.requests_per_second()
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_measures_qwen_1_5b_q8_queued_completion_request_rate(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model_configured(
        REAL_MODEL_ID,
        model_path,
        |state| state.with_inference_wait_timeout(Duration::from_secs(180)),
    )
    .await?;
    let request_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);

    let measurement = measure_concurrent_completion_requests_with_expectation(
        server.addr(),
        request_body.as_bytes(),
        3,
        CompletionResponseExpectation {
            model_id: REAL_MODEL_ID,
            text: "\n",
        },
    )
    .await?;

    assert_eq!(measurement.completed_requests, 3);
    assert!(measurement.elapsed.as_nanos() > 0);
    assert!(measurement.requests_per_second().is_finite());
    assert!(measurement.requests_per_second() > 0.0);
    println!(
        "qwen_1_5b_q8_queued_http_completion_requests={} elapsed_ms={} requests_per_second={:.6}",
        measurement.completed_requests,
        measurement.elapsed.as_millis(),
        measurement.requests_per_second()
    );
    Ok(())
}

fn qwen_1_5b_q8_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_QWEN_1_5B_Q8_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing Qwen2.5-1.5B Q8_0 model artifact: {}",
            model_path.display()
        )
        .into());
    }
    Ok(model_path)
}

fn default_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_MODEL_PATH)
}
