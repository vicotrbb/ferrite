mod config;
mod http;
mod rss;
mod streaming_finish;
mod streaming_metrics;
mod streaming_text;
mod streaming_token_ids;
mod streaming_usage;

#[cfg(test)]
mod tests;

pub use config::{OpenAiEndpoint, ThroughputClientConfig};
pub use rss::RssSummary;
pub use streaming_finish::StreamingFinishSummary;
pub use streaming_metrics::StreamingTimingSummary;
pub use streaming_text::StreamingTextSummary;
pub use streaming_token_ids::StreamingTokenIdsSummary;
pub use streaming_usage::{StreamingPromptCacheTraceSummary, StreamingUsageSummary};

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
    pub streaming_text: Option<StreamingTextSummary>,
    pub streaming_token_ids: Option<StreamingTokenIdsSummary>,
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
        streaming_text: run.streaming_text,
        streaming_token_ids: run.streaming_token_ids,
        streaming_usage: run.streaming_usage,
        rss,
    })
}

struct RequestRun {
    completed_requests: usize,
    streaming_finish: Option<StreamingFinishSummary>,
    streaming_timing: Option<StreamingTimingSummary>,
    streaming_text: Option<StreamingTextSummary>,
    streaming_token_ids: Option<StreamingTokenIdsSummary>,
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
    let mut streaming_text = None;
    let mut streaming_token_ids = None;
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
            let response_finish = response.streaming_finish();
            let response_text = response.streaming_text();
            let response_token_ids = response.streaming_token_ids();
            let response_usage = response.streaming_usage();
            validate_streaming_token_count(
                config,
                response_finish.as_ref(),
                response_usage.as_ref(),
            )?;
            if stream && streaming_timing.is_none() {
                streaming_timing = response.streaming_timing();
            }
            if stream && streaming_finish.is_none() {
                streaming_finish = response_finish;
            }
            if stream && streaming_text.is_none() {
                streaming_text = response_text;
            }
            if stream && streaming_token_ids.is_none() {
                streaming_token_ids = response_token_ids;
            }
            if stream && streaming_usage.is_none() {
                streaming_usage = response_usage;
            }
            completed_requests += 1;
        }
    }

    Ok(RequestRun {
        completed_requests,
        streaming_finish,
        streaming_timing,
        streaming_text,
        streaming_token_ids,
        streaming_usage,
    })
}

pub fn format_result(config: &ThroughputClientConfig, result: ThroughputResult) -> String {
    let mut output = format!(
        "openai_http_addr={}\nopenai_http_endpoint={}\nopenai_http_model={}\nopenai_http_max_tokens={}\nopenai_http_configured_requests={}\nopenai_http_concurrency={}\nopenai_http_stream={}\nopenai_http_stream_usage={}\n{}={}\nelapsed_ms={}\nrequests_per_second={:.6}",
        config.addr(),
        config.endpoint().path(),
        config.model(),
        config.max_tokens(),
        config.requests(),
        config.concurrency(),
        config.stream(),
        config.stream_usage(),
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
    if let Some(text) = &result.streaming_text {
        output.push_str(&format!("\nstreaming_text_bytes={}", text.byte_len()));
    }
    if let Some(token_ids) = result.streaming_token_ids {
        output.push_str(&format!(
            "\nstreaming_content_chunks={}\nstreaming_token_id_chunks={}\nstreaming_token_ids={}\nstreaming_all_content_chunks_have_token_ids={}",
            token_ids.content_chunks(),
            token_ids.token_id_chunks(),
            token_ids.token_ids(),
            token_ids.all_content_chunks_have_token_ids(),
        ));
    }
    if let Some(usage) = result.streaming_usage {
        output.push_str(&format!(
            "\nstreaming_usage_prompt_tokens={}\nstreaming_usage_cached_prompt_tokens={}\nstreaming_usage_completion_tokens={}\nstreaming_usage_total_tokens={}",
            usage.prompt_tokens(),
            usage.cached_prompt_tokens(),
            usage.completion_tokens(),
            usage.total_tokens(),
        ));
        if let Some(finish_source) = usage.finish_source() {
            output.push_str(&format!("\nstreaming_usage_finish_source={finish_source}"));
        }
        if let Some(trace) = usage.prompt_cache_trace() {
            output.push_str(&format!(
                "\nstreaming_usage_prompt_cache_lookup={}\nstreaming_usage_prompt_cache_prompt_token_hash={}\nstreaming_usage_prompt_cache_shared_prefix_tokens={}",
                trace.lookup(),
                trace.prompt_token_hash(),
                trace.shared_prefix_tokens(),
            ));
            if let Some(selected_entry_token_hash) = trace.selected_entry_token_hash() {
                output.push_str(&format!(
                    "\nstreaming_usage_prompt_cache_selected_entry_token_hash={selected_entry_token_hash}"
                ));
            }
        }
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

fn validate_streaming_token_count(
    config: &ThroughputClientConfig,
    finish: Option<&StreamingFinishSummary>,
    usage: Option<&StreamingUsageSummary>,
) -> Result<(), Box<dyn Error>> {
    if !config.stream() || !config.stream_usage() {
        return Ok(());
    }

    let Some(finish) = finish else {
        return Ok(());
    };
    let Some(usage) = usage else {
        return Ok(());
    };

    if finish.reason() == "length" && usage.completion_tokens() != config.max_tokens() as u64 {
        return Err(format!(
            "streaming usage completion_tokens {} did not match requested max_tokens {}",
            usage.completion_tokens(),
            config.max_tokens()
        )
        .into());
    }

    Ok(())
}

fn request_body(config: &ThroughputClientConfig) -> String {
    match config.endpoint() {
        OpenAiEndpoint::Completions => completion_request_body(config),
        OpenAiEndpoint::ChatCompletions => chat_completion_request_body(config),
    }
}

fn completion_request_body(config: &ThroughputClientConfig) -> String {
    let stop = stop_field(config);
    let prompt_cache_key = prompt_cache_key_field(config);
    let stream = stream_field(config);
    let stream_options = stream_options_field(config);
    format!(
        r#"{{"model":{},"prompt":{},"max_tokens":{}{stop}{prompt_cache_key}{stream}{stream_options}}}"#,
        serde_json::Value::String(config.model().to_owned()),
        serde_json::Value::String(config.prompt().to_owned()),
        config.max_tokens()
    )
}

fn chat_completion_request_body(config: &ThroughputClientConfig) -> String {
    let stop = stop_field(config);
    let prompt_cache_key = prompt_cache_key_field(config);
    let prompt_cache_trace = prompt_cache_trace_field(config);
    let stream = stream_field(config);
    let stream_options = stream_options_field(config);
    format!(
        r#"{{"model":{},"messages":{},"max_tokens":{}{stop}{prompt_cache_key}{prompt_cache_trace}{stream}{stream_options}}}"#,
        serde_json::Value::String(config.model().to_owned()),
        chat_messages(config),
        config.max_tokens()
    )
}

fn chat_messages(config: &ThroughputClientConfig) -> String {
    match (config.assistant_context(), config.follow_up()) {
        (Some(assistant_context), Some(follow_up)) => format!(
            r#"[{{"role":"user","content":{}}},{{"role":"assistant","content":{}}},{{"role":"user","content":{}}}]"#,
            serde_json::Value::String(config.prompt().to_owned()),
            serde_json::Value::String(assistant_context.to_owned()),
            serde_json::Value::String(follow_up.to_owned())
        ),
        _ => format!(
            r#"[{{"role":"user","content":{}}}]"#,
            serde_json::Value::String(config.prompt().to_owned())
        ),
    }
}

fn stream_field(config: &ThroughputClientConfig) -> &'static str {
    if config.stream() {
        r#","stream":true"#
    } else {
        ""
    }
}

fn stop_field(config: &ThroughputClientConfig) -> String {
    config
        .stop()
        .map(|stop| format!(r#","stop":{}"#, serde_json::Value::String(stop.to_owned())))
        .unwrap_or_default()
}

fn prompt_cache_key_field(config: &ThroughputClientConfig) -> String {
    config
        .prompt_cache_key()
        .map(|key| {
            format!(
                r#","prompt_cache_key":{}"#,
                serde_json::Value::String(key.to_owned())
            )
        })
        .unwrap_or_default()
}

fn prompt_cache_trace_field(config: &ThroughputClientConfig) -> &'static str {
    if config.prompt_cache_trace() {
        r#","metadata":{"ferrite_cache_trace":"true"}"#
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
