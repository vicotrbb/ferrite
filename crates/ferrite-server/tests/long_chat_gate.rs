use ferrite_server::long_chat_gate::{
    format_disconnect_probe_result, format_error_probe_result, format_plan, format_report,
    format_run_summary, format_scenario_result, format_scenarios, LongChatAssistantContextSource,
    LongChatDisconnectProbeResult, LongChatErrorProbeResult, LongChatGateConfig,
    LongChatScenarioResult,
};
use ferrite_server::throughput_client::{
    OpenAiEndpoint, RssSummary, StreamingFinishSummary, StreamingTextSummary,
    StreamingTimingSummary, StreamingUsageSummary, ThroughputClientConfig, ThroughputResult,
};
use std::ffi::OsString;
use std::time::Duration;

#[test]
fn defaults_to_required_long_chat_token_lengths_and_turns() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([OsString::from("ferrite-openai-long-chat-gate")])?;

    assert_eq!(config.token_lengths(), &[256, 512, 1024]);
    assert_eq!(config.turns(), 4);
    assert!(!config.execute());
    assert!(!config.error_probe());
    assert!(!config.disconnect_probe());
    assert!(!config.require_cached_follow_ups());
    assert_eq!(
        config.models(),
        &[
            "Qwen2.5-0.5B-Instruct-Q4_K_M",
            "Qwen2.5-1.5B-Instruct-Q8_0",
            "Qwen2.5-1.5B-Instruct-Q6_K",
            "SmolLM2-1.7B-Instruct-Q4_K_M",
        ]
    );
    assert_eq!(config.planned_scenarios(), 48);
    assert_eq!(
        config.disconnect_reconnect_timeout(),
        Duration::from_secs(30)
    );
    assert_eq!(config.probe_max_tokens(), None);
    Ok(())
}

#[test]
fn parses_custom_long_chat_token_lengths_turns_and_models() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--execute"),
        OsString::from("--token-lengths"),
        OsString::from("128,256"),
        OsString::from("--turns"),
        OsString::from("5"),
        OsString::from("--models"),
        OsString::from("model-a,model-b"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1:18080"),
        OsString::from("--api-key"),
        OsString::from("secret"),
        OsString::from("--prompt"),
        OsString::from("first turn"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
        OsString::from("--follow-up"),
        OsString::from("second turn"),
        OsString::from("--stop"),
        OsString::from("<STOP>"),
        OsString::from("--rss-pid"),
        OsString::from("4242"),
        OsString::from("--error-probe"),
        OsString::from("--disconnect-probe"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
        OsString::from("--require-cached-follow-ups"),
        OsString::from("--expect-finish-reason"),
        OsString::from("stop"),
        OsString::from("--probe-max-tokens"),
        OsString::from("256"),
        OsString::from("--disconnect-reconnect-timeout-ms"),
        OsString::from("45000"),
    ])?;

    assert_eq!(config.token_lengths(), &[128, 256]);
    assert_eq!(config.turns(), 5);
    assert!(config.execute());
    assert_eq!(config.models(), &["model-a", "model-b"]);
    assert_eq!(config.planned_scenarios(), 20);
    assert_eq!(config.addr(), "127.0.0.1:18080");
    assert_eq!(config.api_key(), "secret");
    assert_eq!(config.prompt(), "first turn");
    assert_eq!(config.assistant_context(), "first answer");
    assert_eq!(config.follow_up(), "second turn");
    assert_eq!(config.prompt_cache_key(), Some("long-chat:prefix"));
    assert_eq!(config.stop(), Some("<STOP>"));
    assert_eq!(config.rss_pid(), Some(4242));
    assert!(config.error_probe());
    assert!(config.disconnect_probe());
    assert!(config.require_cached_follow_ups());
    assert_eq!(config.expected_finish_reason(), Some("stop"));
    assert_eq!(config.probe_max_tokens(), Some(256));
    assert_eq!(
        config.disconnect_reconnect_timeout(),
        Duration::from_secs(45)
    );
    Ok(())
}

#[test]
fn rejects_invalid_long_chat_probe_max_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--probe-max-tokens"),
        OsString::from("0"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--probe-max-tokens"), "{error}");
    Ok(())
}

#[test]
fn rejects_required_cached_follow_ups_without_prompt_cache_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--require-cached-follow-ups"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("--require-cached-follow-ups requires --prompt-cache-key"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_too_few_long_chat_turns() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--turns"),
        OsString::from("3"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--turns"), "{error}");
    assert!(error.to_string().contains("at least 4"), "{error}");
    Ok(())
}

#[test]
fn rejects_empty_long_chat_token_lengths() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from(""),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--token-lengths"), "{error}");
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512,1024"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256,512,1024\nlong_chat_turns=4\nlong_chat_planned_scenarios=12"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_prompt_cache_key() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_key=long-chat:prefix\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_required_cached_follow_ups(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
        OsString::from("--require-cached-follow-ups"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_key=long-chat:prefix\nlong_chat_require_cached_follow_ups=true\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn expands_ordered_long_chat_gate_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--models"),
        OsString::from("model-a,model-b"),
    ])?;

    let scenarios = config.scenarios();

    assert_eq!(scenarios.len(), 16);
    assert_eq!(scenarios[0].model(), "model-a");
    assert_eq!(scenarios[0].turn(), 1);
    assert_eq!(scenarios[0].token_length(), 256);
    assert_eq!(scenarios[7].model(), "model-a");
    assert_eq!(scenarios[7].turn(), 4);
    assert_eq!(scenarios[7].token_length(), 512);
    assert_eq!(scenarios[8].model(), "model-b");
    assert_eq!(scenarios[8].turn(), 1);
    assert_eq!(scenarios[8].token_length(), 256);
    assert_eq!(scenarios[15].model(), "model-b");
    assert_eq!(scenarios[15].turn(), 4);
    assert_eq!(scenarios[15].token_length(), 512);
    Ok(())
}

#[test]
fn formats_long_chat_gate_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
    ])?;

    assert_eq!(
        format_scenarios(&config),
        "long_chat_scenario=model:fixture-model,turn:1,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:512"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_report_with_plan_and_scenarios() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
    ])?;

    assert_eq!(
        format_report(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256,512\nlong_chat_turns=4\nlong_chat_planned_scenarios=8\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:512"
    );
    Ok(())
}

#[test]
fn rejects_empty_long_chat_models() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from(""),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--models"), "{error}");
    Ok(())
}

#[test]
fn builds_streaming_chat_throughput_args_for_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1:18080"),
        OsString::from("--api-key"),
        OsString::from("secret"),
        OsString::from("--prompt"),
        OsString::from("first turn"),
        OsString::from("--assistant-context"),
        OsString::from("first answer"),
        OsString::from("--follow-up"),
        OsString::from("second turn"),
        OsString::from("--stop"),
        OsString::from("<STOP>"),
        OsString::from("--rss-pid"),
        OsString::from("4242"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected scenario")?;

    let args = config.throughput_args(&scenario);
    let throughput = ThroughputClientConfig::parse(args)?;

    assert_eq!(throughput.endpoint(), OpenAiEndpoint::ChatCompletions);
    assert_eq!(throughput.addr().to_string(), "127.0.0.1:18080");
    assert_eq!(throughput.model(), "fixture-model");
    assert_eq!(throughput.prompt(), "first turn");
    assert_eq!(throughput.assistant_context(), Some("first answer"));
    assert_eq!(throughput.follow_up(), Some("second turn"));
    assert_eq!(throughput.stop(), Some("<STOP>"));
    assert_eq!(throughput.max_tokens(), 256);
    assert_eq!(throughput.requests(), 1);
    assert_eq!(throughput.concurrency(), 1);
    assert_eq!(throughput.rss_pid(), Some(4242));
    assert_eq!(throughput.api_key(), "secret");
    assert!(throughput.stream());
    assert!(throughput.stream_usage());
    Ok(())
}

#[test]
fn passes_prompt_cache_key_to_long_chat_throughput_config() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected scenario")?;

    let args = config.throughput_args(&scenario);
    let throughput = ThroughputClientConfig::parse(args)?;

    assert_eq!(config.prompt_cache_key(), Some("long-chat:prefix"));
    assert_eq!(throughput.prompt_cache_key(), Some("long-chat:prefix"));
    Ok(())
}

#[test]
fn rejects_invalid_long_chat_rss_pid() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--rss-pid"),
        OsString::from("0"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--rss-pid"), "{error}");
    Ok(())
}

#[test]
fn builds_typed_throughput_configs_for_all_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("model-a,model-b"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    let throughput_configs = config.throughput_configs()?;

    assert_eq!(throughput_configs.len(), 16);
    assert_eq!(throughput_configs[0].model(), "model-a");
    assert_eq!(throughput_configs[0].max_tokens(), 256);
    assert_eq!(throughput_configs[7].model(), "model-a");
    assert_eq!(throughput_configs[7].max_tokens(), 512);
    assert_eq!(throughput_configs[8].model(), "model-b");
    assert_eq!(throughput_configs[8].max_tokens(), 256);
    assert_eq!(throughput_configs[15].model(), "model-b");
    assert_eq!(throughput_configs[15].max_tokens(), 512);
    assert!(throughput_configs
        .iter()
        .all(|config| config.endpoint() == OpenAiEndpoint::ChatCompletions));
    assert!(throughput_configs.iter().all(|config| config.stream()));
    assert!(throughput_configs
        .iter()
        .all(|config| config.stream_usage()));
    Ok(())
}

#[test]
fn formats_long_chat_scenario_result() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .nth(1)
        .ok_or("expected second scenario")?;
    let throughput = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_finish: Some(StreamingFinishSummary::new("length")),
        streaming_timing: StreamingTimingSummary::from_event_offsets(&[
            Duration::from_millis(100),
            Duration::from_millis(140),
            Duration::from_millis(170),
        ]),
        streaming_text: None,
        streaming_usage: Some(StreamingUsageSummary::new(16, 256, 272)),
        rss: Some(RssSummary::new(1000, 2000, 1500)),
    };

    let result = LongChatScenarioResult::new_with_assistant_context_source(
        &scenario,
        throughput,
        LongChatAssistantContextSource::Generated,
    );

    assert_eq!(
        format_scenario_result(&result),
        "long_chat_result=model:fixture-model,turn:2,max_tokens:256\nlong_chat_result_assistant_context_source=generated\nlong_chat_result_completed_requests=1\nlong_chat_result_elapsed_ms=400\nlong_chat_result_finish_reason=length\nlong_chat_result_usage_prompt_tokens=16\nlong_chat_result_usage_cached_prompt_tokens=0\nlong_chat_result_usage_completion_tokens=256\nlong_chat_result_usage_total_tokens=272\nlong_chat_result_hit_token_limit=true\nlong_chat_result_streaming_token_events=3\nlong_chat_result_time_to_first_token_ms=100\nlong_chat_result_stream_observed_prefill_elapsed_ms=100\nlong_chat_result_first_token_timestamp_ms=100\nlong_chat_result_stream_observed_decode_elapsed_ms=70\nlong_chat_result_stream_observed_decode_tokens_per_second=28.571429\nlong_chat_result_streaming_total_elapsed_ms=170\nlong_chat_result_streaming_tokens_per_second=17.647059\nlong_chat_result_token_latency_min_ms=30\nlong_chat_result_token_latency_p50_ms=40\nlong_chat_result_token_latency_p95_ms=100\nlong_chat_result_token_latency_max_ms=100\nlong_chat_result_server_rss_before_bytes=1000\nlong_chat_result_server_rss_after_bytes=2000\nlong_chat_result_server_rss_idle_bytes=1500"
    );
    Ok(())
}

#[test]
fn formats_long_chat_stop_result_as_not_hitting_token_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected scenario")?;
    let throughput = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_finish: Some(StreamingFinishSummary::new("stop")),
        streaming_timing: None,
        streaming_text: None,
        streaming_usage: Some(StreamingUsageSummary::new(16, 3, 19)),
        rss: None,
    };

    let result = LongChatScenarioResult::new(&scenario, throughput);

    assert!(format_scenario_result(&result).contains("long_chat_result_hit_token_limit=false"));
    Ok(())
}

#[test]
fn formats_long_chat_error_probe_result() {
    let result = LongChatErrorProbeResult::new(401, true, 256);

    assert_eq!(
        format_error_probe_result(&result),
        "long_chat_error_probe_unauthorized_status=401\nlong_chat_error_probe_reconnect_completed=true\nlong_chat_error_probe_max_tokens=256"
    );
}

#[test]
fn formats_long_chat_disconnect_probe_result() {
    let result = LongChatDisconnectProbeResult::new(true, true, 256);

    assert_eq!(
        format_disconnect_probe_result(&result),
        "long_chat_disconnect_probe_aborted_after_generated_event=true\nlong_chat_disconnect_probe_reconnect_completed=true\nlong_chat_disconnect_probe_reconnect_generated_event=true\nlong_chat_disconnect_probe_reconnect_started_new_generation=true\nlong_chat_disconnect_probe_max_tokens=256"
    );
}

#[test]
fn formats_integrated_long_chat_run_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--rss-pid"),
        OsString::from("4242"),
        OsString::from("--error-probe"),
        OsString::from("--disconnect-probe"),
    ])?;
    let results = config
        .scenarios()
        .iter()
        .map(|scenario| {
            let source = if scenario.turn() == 1 {
                LongChatAssistantContextSource::Seed
            } else {
                LongChatAssistantContextSource::Generated
            };
            LongChatScenarioResult::new_with_assistant_context_source(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("length")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(100),
                        Duration::from_millis(140),
                    ]),
                    streaming_text: None,
                    streaming_usage: Some(StreamingUsageSummary::new(
                        16,
                        scenario.token_length() as u64,
                        scenario.token_length() as u64 + 16,
                    )),
                    rss: Some(RssSummary::new(1000, 2000, 1500)),
                },
                source,
            )
        })
        .collect::<Vec<_>>();
    let error_probe = LongChatErrorProbeResult::new(401, true, 8);
    let disconnect_probe = LongChatDisconnectProbeResult::new(true, true, 8);

    assert_eq!(
        format_run_summary(
            &config,
            &results,
            Some(&error_probe),
            Some(&disconnect_probe)
        ),
        "long_chat_summary_planned_scenarios=4\nlong_chat_summary_completed_scenarios=4\nlong_chat_summary_all_finish_reasons_present=true\nlong_chat_summary_all_usage_accounting_valid=true\nlong_chat_summary_all_token_limit_status_present=true\nlong_chat_summary_any_token_limit_hit=true\nlong_chat_summary_prompt_cache_key_present=false\nlong_chat_summary_cached_follow_ups_required=false\nlong_chat_summary_any_cached_prompt_tokens=false\nlong_chat_summary_generated_follow_up_turns=3\nlong_chat_summary_cached_generated_follow_up_turns=0\nlong_chat_summary_uncached_generated_follow_up_turns=3\nlong_chat_summary_all_generated_follow_up_turns_cached=false\nlong_chat_summary_all_follow_up_turns_use_generated_context=true\nlong_chat_summary_all_timing_present=true\nlong_chat_summary_rss_required=true\nlong_chat_summary_all_rss_present=true\nlong_chat_summary_error_probe_required=true\nlong_chat_summary_error_probe_completed=true\nlong_chat_summary_disconnect_probe_required=true\nlong_chat_summary_disconnect_probe_completed=true\nlong_chat_summary_disconnect_probe_reconnect_started_new_generation=true\nlong_chat_summary_run_complete=true"
    );
    Ok(())
}

#[test]
fn formats_cache_observability_in_long_chat_run_summary() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
    ])?;
    let results = config
        .scenarios()
        .iter()
        .map(|scenario| {
            let source = if scenario.turn() == 1 {
                LongChatAssistantContextSource::Seed
            } else {
                LongChatAssistantContextSource::Generated
            };
            let usage = StreamingUsageSummary::new(
                16,
                scenario.token_length() as u64,
                scenario.token_length() as u64 + 16,
            )
            .with_cached_prompt_tokens(if scenario.turn() == 1 { 0 } else { 8 });

            LongChatScenarioResult::new_with_assistant_context_source(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("length")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(100),
                        Duration::from_millis(140),
                    ]),
                    streaming_text: None,
                    streaming_usage: Some(usage),
                    rss: None,
                },
                source,
            )
        })
        .collect::<Vec<_>>();

    let summary = format_run_summary(&config, &results, None, None);

    assert!(summary.contains("long_chat_summary_prompt_cache_key_present=true"));
    assert!(summary.contains("long_chat_summary_any_cached_prompt_tokens=true"));
    assert!(summary.contains("long_chat_summary_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_cached_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_uncached_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_all_generated_follow_up_turns_cached=true"));
    Ok(())
}

#[test]
fn cache_summary_does_not_treat_missing_generated_follow_ups_as_cached(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected seed scenario")?;
    let result = LongChatScenarioResult::new_with_assistant_context_source(
        &scenario,
        ThroughputResult {
            completed_requests: 1,
            elapsed: Duration::from_millis(400),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                Duration::from_millis(100),
                Duration::from_millis(140),
            ]),
            streaming_text: None,
            streaming_usage: Some(StreamingUsageSummary::new(16, 256, 272)),
            rss: None,
        },
        LongChatAssistantContextSource::Seed,
    );

    let summary = format_run_summary(&config, &[result], None, None);

    assert!(summary.contains("long_chat_summary_prompt_cache_key_present=true"));
    assert!(summary.contains("long_chat_summary_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_cached_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_uncached_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_all_generated_follow_up_turns_cached=false"));
    Ok(())
}

#[test]
fn required_cached_follow_ups_make_summary_incomplete_without_cache_hits(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-key"),
        OsString::from("long-chat:prefix"),
        OsString::from("--require-cached-follow-ups"),
    ])?;
    let results = config
        .scenarios()
        .iter()
        .map(|scenario| {
            let source = if scenario.turn() == 1 {
                LongChatAssistantContextSource::Seed
            } else {
                LongChatAssistantContextSource::Generated
            };
            LongChatScenarioResult::new_with_assistant_context_source(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("length")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(100),
                        Duration::from_millis(140),
                    ]),
                    streaming_text: None,
                    streaming_usage: Some(StreamingUsageSummary::new(
                        16,
                        scenario.token_length() as u64,
                        scenario.token_length() as u64 + 16,
                    )),
                    rss: None,
                },
                source,
            )
        })
        .collect::<Vec<_>>();

    let summary = format_run_summary(&config, &results, None, None);

    assert!(summary.contains("long_chat_summary_cached_follow_ups_required=true"));
    assert!(summary.contains("long_chat_summary_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_cached_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_uncached_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_all_generated_follow_up_turns_cached=false"));
    assert!(summary.contains("long_chat_summary_run_complete=false"));
    Ok(())
}

#[test]
fn runs_long_chat_gate_with_injected_executor() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;
    let mut observed = Vec::new();

    let results = config.run_with_executor(|throughput| {
        observed.push((throughput.model().to_owned(), throughput.max_tokens()));
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10 * throughput.max_tokens() as u64),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(
        observed,
        [
            ("fixture-model".to_owned(), 256),
            ("fixture-model".to_owned(), 512),
            ("fixture-model".to_owned(), 256),
            ("fixture-model".to_owned(), 512),
            ("fixture-model".to_owned(), 256),
            ("fixture-model".to_owned(), 512),
            ("fixture-model".to_owned(), 256),
            ("fixture-model".to_owned(), 512),
        ]
    );
    assert_eq!(results.len(), 8);
    assert_eq!(results[0].model(), "fixture-model");
    assert_eq!(results[0].turn(), 1);
    assert_eq!(results[0].token_length(), 256);
    assert_eq!(
        results[0].throughput().streaming_usage,
        Some(StreamingUsageSummary::new(8, 256, 264))
    );
    assert_eq!(results[7].turn(), 4);
    assert_eq!(results[7].token_length(), 512);
    Ok(())
}

#[test]
fn observes_long_chat_results_as_each_scenario_finishes() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;
    let mut observed = Vec::new();

    let results = config.run_with_executor_and_observer(
        |throughput| {
            Ok(ThroughputResult {
                completed_requests: throughput.requests(),
                elapsed: Duration::from_millis(10 * throughput.max_tokens() as u64),
                streaming_finish: Some(StreamingFinishSummary::new("length")),
                streaming_timing: None,
                streaming_text: None,
                streaming_usage: Some(StreamingUsageSummary::new(
                    8,
                    throughput.max_tokens() as u64,
                    throughput.max_tokens() as u64 + 8,
                )),
                rss: None,
            })
        },
        |result| {
            observed.push((result.turn(), result.token_length()));
            Ok(())
        },
    )?;

    assert_eq!(
        observed,
        [
            (1, 256),
            (1, 512),
            (2, 256),
            (2, 512),
            (3, 256),
            (3, 512),
            (4, 256),
            (4, 512),
        ]
    );
    assert_eq!(results.len(), observed.len());
    Ok(())
}

#[test]
fn carries_generated_assistant_text_between_turns_per_token_length(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
    ])?;
    let mut observed_contexts = Vec::new();

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push((
            throughput.max_tokens(),
            throughput.assistant_context().map(str::to_owned),
        ));
        let turn_index_for_length = observed_contexts
            .iter()
            .filter(|(max_tokens, _)| *max_tokens == throughput.max_tokens())
            .count();
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(format!(
                "generated-{}-{turn_index_for_length}",
                throughput.max_tokens()
            ))),
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 8);
    assert_eq!(
        observed_contexts,
        [
            (256, Some("seed answer".to_owned())),
            (512, Some("seed answer".to_owned())),
            (256, Some("generated-256-1".to_owned())),
            (512, Some("generated-512-1".to_owned())),
            (256, Some("generated-256-2".to_owned())),
            (512, Some("generated-512-2".to_owned())),
            (256, Some("generated-256-3".to_owned())),
            (512, Some("generated-512-3".to_owned())),
        ]
    );
    Ok(())
}

#[test]
fn rejects_unexpected_long_chat_finish_reason() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--expect-finish-reason"),
        OsString::from("stop"),
    ])?;

    let result = config.run_with_executor(|throughput| {
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    });
    let error = match result {
        Ok(results) => {
            return Err(format!("expected finish reason mismatch, got {results:?}").into())
        }
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("expected finish_reason stop, got length"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_missing_long_chat_finish_reason_when_expected() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--expect-finish-reason"),
        OsString::from("stop"),
    ])?;

    let result = config.run_with_executor(|throughput| {
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: None,
            streaming_timing: None,
            streaming_text: None,
            streaming_usage: None,
            rss: None,
        })
    });
    let error = match result {
        Ok(results) => {
            return Err(format!("expected missing finish reason error, got {results:?}").into())
        }
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("expected finish_reason stop, got none"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_empty_long_chat_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--prompt"),
        OsString::from(""),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--prompt"), "{error}");
    Ok(())
}
