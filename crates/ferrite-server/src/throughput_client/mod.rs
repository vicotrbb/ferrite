mod config;
mod http;
mod streaming_metrics;

#[cfg(test)]
mod tests;

pub use config::{OpenAiEndpoint, ThroughputClientConfig};
pub use streaming_metrics::StreamingTimingSummary;

use std::{
    error::Error,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct ThroughputResult {
    pub completed_requests: usize,
    pub elapsed: Duration,
}

impl ThroughputResult {
    pub fn requests_per_second(&self) -> f64 {
        self.completed_requests as f64 / self.elapsed.as_secs_f64()
    }
}

pub async fn run_completion_benchmark(
    config: &ThroughputClientConfig,
) -> Result<ThroughputResult, Box<dyn Error>> {
    let request_body = request_body(config);
    let endpoint = config.endpoint();
    let stream = config.stream();
    let started = Instant::now();
    let mut completed_requests = 0;

    while completed_requests < config.requests() {
        let batch_size = config
            .concurrency()
            .min(config.requests().saturating_sub(completed_requests));
        let mut tasks = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let request_body = request_body.clone();
            let api_key = config.api_key().to_owned();
            let addr = config.addr();
            tasks.push(tokio::spawn(async move {
                http::send_openai_request(addr, &api_key, endpoint.path(), request_body.as_bytes())
                    .await
                    .map_err(|error| error.to_string())
            }));
        }

        for task in tasks {
            let response = task
                .await
                .map_err(|error| std::io::Error::other(format!("request task failed: {error}")))?
                .map_err(std::io::Error::other)?;
            http::validate_openai_response(endpoint, stream, &response)?;
            completed_requests += 1;
        }
    }

    Ok(ThroughputResult {
        completed_requests,
        elapsed: started.elapsed(),
    })
}

pub fn format_result(config: &ThroughputClientConfig, result: ThroughputResult) -> String {
    format!(
        "{}={}\nelapsed_ms={}\nrequests_per_second={:.6}",
        config.endpoint().metric_name(config.stream()),
        result.completed_requests,
        result.elapsed.as_millis(),
        result.requests_per_second()
    )
}

fn request_body(config: &ThroughputClientConfig) -> String {
    match config.endpoint() {
        OpenAiEndpoint::Completions => completion_request_body(config),
        OpenAiEndpoint::ChatCompletions => chat_completion_request_body(config),
    }
}

fn completion_request_body(config: &ThroughputClientConfig) -> String {
    let stream = stream_field(config);
    format!(
        r#"{{"model":{},"prompt":{},"max_tokens":{}{stream}}}"#,
        serde_json::Value::String(config.model().to_owned()),
        serde_json::Value::String(config.prompt().to_owned()),
        config.max_tokens()
    )
}

fn chat_completion_request_body(config: &ThroughputClientConfig) -> String {
    let stream = stream_field(config);
    format!(
        r#"{{"model":{},"messages":[{{"role":"user","content":{}}}],"max_tokens":{}{stream}}}"#,
        serde_json::Value::String(config.model().to_owned()),
        serde_json::Value::String(config.prompt().to_owned()),
        config.max_tokens()
    )
}

fn stream_field(config: &ThroughputClientConfig) -> &'static str {
    if config.stream() {
        r#","stream":true"#
    } else {
        ""
    }
}
