mod config;
mod http;
mod rss;
mod streaming_finish;
mod streaming_metrics;
mod streaming_usage;

#[cfg(test)]
mod tests;

pub use config::{OpenAiEndpoint, ThroughputClientConfig};
pub use rss::RssSummary;
pub use streaming_finish::StreamingFinishSummary;
pub use streaming_metrics::StreamingTimingSummary;
pub use streaming_usage::StreamingUsageSummary;

use std::{
    error::Error,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct ThroughputResult {
    pub completed_requests: usize,
    pub elapsed: Duration,
    pub streaming_finish: Option<StreamingFinishSummary>,
    pub streaming_timing: Option<StreamingTimingSummary>,
    pub streaming_usage: Option<StreamingUsageSummary>,
    pub rss: Option<RssSummary>,
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
    let (run, rss) = if let Some(pid) = config.rss_pid() {
        let (run, rss) = RssSummary::sample_around(pid, config.rss_idle_delay(), async {
            run_requests(config, &request_body, endpoint, stream).await
        })
        .await?;
        (run, Some(rss))
    } else {
        (
            run_requests(config, &request_body, endpoint, stream).await?,
            None,
        )
    };

    Ok(ThroughputResult {
        completed_requests: run.completed_requests,
        elapsed: started.elapsed(),
        streaming_finish: run.streaming_finish,
        streaming_timing: run.streaming_timing,
        streaming_usage: run.streaming_usage,
        rss,
    })
}

struct RequestRun {
    completed_requests: usize,
    streaming_finish: Option<StreamingFinishSummary>,
    streaming_timing: Option<StreamingTimingSummary>,
    streaming_usage: Option<StreamingUsageSummary>,
}

async fn run_requests(
    config: &ThroughputClientConfig,
    request_body: &str,
    endpoint: OpenAiEndpoint,
    stream: bool,
) -> Result<RequestRun, Box<dyn Error>> {
    let mut completed_requests = 0;
    let mut streaming_finish = None;
    let mut streaming_timing = None;
    let mut streaming_usage = None;

    while completed_requests < config.requests() {
        let batch_size = config
            .concurrency()
            .min(config.requests().saturating_sub(completed_requests));
        let mut tasks = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let request_body = request_body.to_owned();
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
            http::validate_openai_response(
                endpoint,
                stream,
                config.stream_usage(),
                response.raw(),
            )?;
            if stream && streaming_timing.is_none() {
                streaming_timing = response.streaming_timing();
            }
            if stream && streaming_finish.is_none() {
                streaming_finish = response.streaming_finish();
            }
            if stream && streaming_usage.is_none() {
                streaming_usage = response.streaming_usage();
            }
            completed_requests += 1;
        }
    }

    Ok(RequestRun {
        completed_requests,
        streaming_finish,
        streaming_timing,
        streaming_usage,
    })
}

pub fn format_result(config: &ThroughputClientConfig, result: ThroughputResult) -> String {
    let mut output = format!(
        "{}={}\nelapsed_ms={}\nrequests_per_second={:.6}",
        config.endpoint().metric_name(config.stream()),
        result.completed_requests,
        result.elapsed.as_millis(),
        result.requests_per_second()
    );
    if let Some(summary) = result.streaming_timing {
        output.push_str(&format!(
            "\nstreaming_token_events={}\nstreaming_time_to_first_token_ms={}\nstreaming_total_elapsed_ms={}\nstreaming_tokens_per_second={:.6}\nstreaming_token_latency_min_ms={}\nstreaming_token_latency_p50_ms={}\nstreaming_token_latency_p95_ms={}\nstreaming_token_latency_max_ms={}",
            summary.token_events(),
            summary.time_to_first_token().as_millis(),
            summary.total_elapsed().as_millis(),
            summary.tokens_per_second(),
            summary.min_token_latency().as_millis(),
            summary.p50_token_latency().as_millis(),
            summary.p95_token_latency().as_millis(),
            summary.max_token_latency().as_millis(),
        ));
    }
    if let Some(finish) = &result.streaming_finish {
        output.push_str(&format!("\nstreaming_finish_reason={}", finish.reason()));
    }
    if let Some(usage) = result.streaming_usage {
        output.push_str(&format!(
            "\nstreaming_usage_prompt_tokens={}\nstreaming_usage_completion_tokens={}\nstreaming_usage_total_tokens={}",
            usage.prompt_tokens(),
            usage.completion_tokens(),
            usage.total_tokens(),
        ));
    }
    if let Some(rss) = result.rss {
        output.push_str(&format!(
            "\nserver_rss_before_bytes={}\nserver_rss_after_bytes={}\nserver_rss_idle_bytes={}",
            rss.before_bytes(),
            rss.after_bytes(),
            rss.idle_bytes(),
        ));
    }
    output
}

fn request_body(config: &ThroughputClientConfig) -> String {
    match config.endpoint() {
        OpenAiEndpoint::Completions => completion_request_body(config),
        OpenAiEndpoint::ChatCompletions => chat_completion_request_body(config),
    }
}

fn completion_request_body(config: &ThroughputClientConfig) -> String {
    let stream = stream_field(config);
    let stream_options = stream_options_field(config);
    format!(
        r#"{{"model":{},"prompt":{},"max_tokens":{}{stream}{stream_options}}}"#,
        serde_json::Value::String(config.model().to_owned()),
        serde_json::Value::String(config.prompt().to_owned()),
        config.max_tokens()
    )
}

fn chat_completion_request_body(config: &ThroughputClientConfig) -> String {
    let stream = stream_field(config);
    let stream_options = stream_options_field(config);
    format!(
        r#"{{"model":{},"messages":[{{"role":"user","content":{}}}],"max_tokens":{}{stream}{stream_options}}}"#,
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

fn stream_options_field(config: &ThroughputClientConfig) -> &'static str {
    if config.stream_usage() {
        r#","stream_options":{"include_usage":true}"#
    } else {
        ""
    }
}
