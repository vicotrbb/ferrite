use ferrite_server::long_chat_gate::{
    format_disconnect_probe_result, format_error_probe_result, format_plan,
    format_queue_probe_result, format_report, format_run_summary, format_scenario_result,
    format_scenarios, LongChatAssistantContextSource, LongChatDisconnectProbeResult,
    LongChatErrorProbeResult, LongChatGateConfig, LongChatProofArtifacts, LongChatQueueProbeResult,
    LongChatScenarioResult, LongChatTextIdentity,
};
use ferrite_server::throughput_client::{
    OpenAiEndpoint, RssSummary, StreamingFinishSummary, StreamingPromptCacheTraceSummary,
    StreamingTextSummary, StreamingTimingSummary, StreamingTokenIdsSummary, StreamingUsageSummary,
    ThroughputClientConfig, ThroughputResult,
};
use std::ffi::OsString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn defaults_to_required_long_chat_token_lengths_and_turns() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([OsString::from("ferrite-openai-long-chat-gate")])?;

    assert_eq!(config.token_lengths(), &[256, 512, 1024]);
    assert_eq!(config.turns(), 4);
    assert!(!config.execute());
    assert!(!config.error_probe());
    assert!(!config.disconnect_probe());
    assert!(!config.queue_probe());
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
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"{"state_anchor":"7291"}"#),
        OsString::from("--generated-context-state-capsule-placement"),
        OsString::from("follow-up"),
        OsString::from("--require-generated-response-contains"),
        OsString::from("continuity-marker"),
        OsString::from("--disconnect-reconnect-timeout-ms"),
        OsString::from("45000"),
        OsString::from("--proof-log"),
        OsString::from("target/proof/long-chat.log"),
        OsString::from("--proof-exit-code"),
        OsString::from("target/proof/long-chat.exit"),
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
        config.generated_context_state_capsule(),
        Some(r#"{"state_anchor":"7291"}"#)
    );
    assert_eq!(
        config.generated_context_state_capsule_placement().as_str(),
        "follow-up"
    );
    assert_eq!(
        config.required_generated_response_substrings(),
        &["continuity-marker"]
    );
    assert_eq!(
        config.disconnect_reconnect_timeout(),
        Duration::from_secs(45)
    );
    assert_eq!(
        config.proof_log_path().map(|path| path.to_string_lossy()),
        Some("target/proof/long-chat.log".into())
    );
    assert_eq!(
        config
            .proof_exit_code_path()
            .map(|path| path.to_string_lossy()),
        Some("target/proof/long-chat.exit".into())
    );
    Ok(())
}

#[test]
fn proof_artifacts_write_log_and_exit_code_files() -> Result<(), Box<dyn std::error::Error>> {
    let root = unique_temp_dir("ferrite-long-chat-artifacts");
    let log_path = root.join("nested").join("proof.log");
    let exit_code_path = root.join("nested").join("proof.exit");
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--proof-log"),
        log_path.as_os_str().to_owned(),
        OsString::from("--proof-exit-code"),
        exit_code_path.as_os_str().to_owned(),
    ])?;

    let mut artifacts = LongChatProofArtifacts::create(&config)?;
    artifacts.write_line("long_chat_result=ok")?;
    artifacts.write_exit_code(7)?;

    assert_eq!(std::fs::read_to_string(&log_path)?, "long_chat_result=ok\n");
    assert_eq!(std::fs::read_to_string(&exit_code_path)?, "7\n");
    std::fs::remove_dir_all(root)?;
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
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256,512,1024\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=12"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_execute_flag() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--execute"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:8080\nlong_chat_execute=true\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_server_address() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1:18080"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:18080\nlong_chat_planned_scenarios=4"
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
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_key=long-chat:prefix\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_prompt_cache_keys() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_keys=tenant-a:thread-1,tenant-b:thread-1\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=8"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_queue_probe() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
        OsString::from("--queue-probe"),
        OsString::from("--probe-max-tokens"),
        OsString::from("64"),
    ])?;

    assert!(config.queue_probe());
    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_keys=tenant-a:thread-1,tenant-b:thread-1\nlong_chat_addr=127.0.0.1:8080\nlong_chat_queue_probe_required=true\nlong_chat_probe_max_tokens=64\nlong_chat_planned_scenarios=8"
    );
    Ok(())
}

#[test]
fn rejects_queue_probe_without_two_prompt_cache_keys() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--queue-probe"),
        OsString::from("--prompt-cache-key"),
        OsString::from("tenant-a:thread-1"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("--queue-probe requires at least two --prompt-cache-keys"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_combining_prompt_cache_key_and_prompt_cache_keys(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--prompt-cache-key"),
        OsString::from("tenant-a:thread-1"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("--prompt-cache-key cannot be combined with --prompt-cache-keys"),
        "{error}"
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
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_key=long-chat:prefix\nlong_chat_addr=127.0.0.1:8080\nlong_chat_require_cached_follow_ups=true\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_proof_artifact_paths() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--proof-log"),
        OsString::from("target/proof/long-chat.log"),
        OsString::from("--proof-exit-code"),
        OsString::from("target/proof/long-chat.exit"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_proof_log_path=target/proof/long-chat.log\nlong_chat_proof_exit_code_path=target/proof/long-chat.exit\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_stop_expectation() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--stop"),
        OsString::from("</s>"),
        OsString::from("--expect-finish-reason"),
        OsString::from("stop"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:8080\nlong_chat_stop_configured=true\nlong_chat_expected_finish_reason=stop\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_probe_metadata() -> Result<(), Box<dyn std::error::Error>> {
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
        OsString::from("--probe-max-tokens"),
        OsString::from("256"),
        OsString::from("--disconnect-reconnect-timeout-ms"),
        OsString::from("1500"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:8080\nlong_chat_rss_pid=4242\nlong_chat_error_probe_required=true\nlong_chat_disconnect_probe_required=true\nlong_chat_probe_max_tokens=256\nlong_chat_disconnect_reconnect_timeout_ms=1500\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_generated_response_requirements(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--require-generated-response-contains"),
        OsString::from("continuity-marker"),
        OsString::from("--require-generated-response-contains"),
        OsString::from("anchor-token"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_required_generated_response_substrings=continuity-marker,anchor-token\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_state_capsule() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"{"state_anchor":"7291"}"#),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_generated_context_state_capsule_configured=true\nlong_chat_generated_context_state_capsule_placement=assistant-context\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=4"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_plan_with_follow_up_state_capsule_placement(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"state_anchor=7291"#),
        OsString::from("--generated-context-state-capsule-placement"),
        OsString::from("follow-up"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_generated_context_state_capsule_configured=true\nlong_chat_generated_context_state_capsule_placement=follow-up\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=4"
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
fn expands_prompt_cache_keys_as_separate_long_chat_lanes() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
    ])?;

    assert_eq!(
        format_scenarios(&config),
        "long_chat_scenario=model:fixture-model,turn:1,max_tokens:256,prompt_cache_key:tenant-a:thread-1\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:256,prompt_cache_key:tenant-a:thread-1\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:256,prompt_cache_key:tenant-a:thread-1\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:256,prompt_cache_key:tenant-a:thread-1\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:256,prompt_cache_key:tenant-b:thread-1\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:256,prompt_cache_key:tenant-b:thread-1\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:256,prompt_cache_key:tenant-b:thread-1\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:256,prompt_cache_key:tenant-b:thread-1"
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
        "long_chat_models=fixture-model\nlong_chat_token_lengths=256,512\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:8080\nlong_chat_planned_scenarios=8\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:1,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:2,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:3,max_tokens:512\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:256\nlong_chat_scenario=model:fixture-model,turn:4,max_tokens:512"
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
fn passes_prompt_cache_keys_to_their_long_chat_lanes() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
    ])?;
    let scenarios = config.scenarios();

    let first_lane = config.throughput_config_with_assistant_context(&scenarios[0], "context")?;
    let second_lane = config.throughput_config_with_assistant_context(&scenarios[4], "context")?;

    assert_eq!(first_lane.prompt_cache_key(), Some("tenant-a:thread-1"));
    assert_eq!(second_lane.prompt_cache_key(), Some("tenant-b:thread-1"));
    Ok(())
}

#[test]
fn passes_prompt_cache_trace_to_long_chat_throughput_config(
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
        OsString::from("--prompt-cache-trace"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected scenario")?;
    let throughput = config.throughput_config_with_assistant_context(&scenario, "context")?;

    assert!(config.prompt_cache_trace());
    assert!(throughput.prompt_cache_trace());
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
        streaming_token_ids: Some(StreamingTokenIdsSummary::new(3, 3, 256)),
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
        "long_chat_result=model:fixture-model,turn:2,max_tokens:256\nlong_chat_result_assistant_context_source=generated\nlong_chat_result_completed_requests=1\nlong_chat_result_elapsed_ms=400\nlong_chat_result_finish_reason=length\nlong_chat_result_usage_prompt_tokens=16\nlong_chat_result_usage_cached_prompt_tokens=0\nlong_chat_result_usage_completion_tokens=256\nlong_chat_result_usage_total_tokens=272\nlong_chat_result_hit_token_limit=true\nlong_chat_result_streaming_token_events=3\nlong_chat_result_time_to_first_token_ms=100\nlong_chat_result_stream_observed_prefill_elapsed_ms=100\nlong_chat_result_first_token_timestamp_ms=100\nlong_chat_result_stream_observed_decode_elapsed_ms=70\nlong_chat_result_stream_observed_decode_tokens_per_second=28.571429\nlong_chat_result_streaming_total_elapsed_ms=170\nlong_chat_result_streaming_tokens_per_second=17.647059\nlong_chat_result_token_latency_min_ms=30\nlong_chat_result_token_latency_p50_ms=40\nlong_chat_result_token_latency_p95_ms=100\nlong_chat_result_token_latency_max_ms=100\nlong_chat_result_streaming_content_chunks=3\nlong_chat_result_streaming_token_id_chunks=3\nlong_chat_result_streaming_token_ids=256\nlong_chat_result_streaming_all_content_chunks_have_token_ids=true\nlong_chat_result_server_rss_before_bytes=1000\nlong_chat_result_server_rss_after_bytes=2000\nlong_chat_result_server_rss_idle_bytes=1500"
    );
    Ok(())
}

#[test]
fn formats_prompt_cache_key_in_long_chat_scenario_result() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
    ])?;
    let scenario = config
        .scenarios()
        .into_iter()
        .next()
        .ok_or("expected scenario")?;
    let throughput = ThroughputResult {
        completed_requests: 1,
        elapsed: Duration::from_millis(400),
        streaming_finish: Some(StreamingFinishSummary::new("length")),
        streaming_timing: None,
        streaming_text: None,
        streaming_token_ids: None,
        streaming_usage: Some(StreamingUsageSummary::new(16, 256, 272)),
        rss: None,
    };

    let result = LongChatScenarioResult::new(&scenario, throughput);

    assert!(
        format_scenario_result(&result).contains(
            "long_chat_result=model:fixture-model,turn:1,max_tokens:256,prompt_cache_key:tenant-a:thread-1"
        )
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
        streaming_token_ids: None,
        streaming_usage: Some(StreamingUsageSummary::new(16, 3, 19)),
        rss: None,
    };

    let result = LongChatScenarioResult::new(&scenario, throughput);

    assert!(format_scenario_result(&result).contains("long_chat_result_hit_token_limit=false"));
    Ok(())
}

#[test]
fn formats_non_disclosing_context_and_response_identity() -> Result<(), Box<dyn std::error::Error>>
{
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
    let result = LongChatScenarioResult::new_with_assistant_context_source_and_identity(
        &scenario,
        ThroughputResult {
            completed_requests: 1,
            elapsed: Duration::from_millis(400),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::from_chunks(["al", "pha"])),
            streaming_token_ids: None,
            streaming_usage: None,
            rss: None,
        },
        LongChatAssistantContextSource::Seed,
        LongChatTextIdentity::from_text("seed answer"),
    );

    let formatted = format_scenario_result(&result);

    assert!(formatted.contains("long_chat_result_assistant_context_bytes=11"));
    assert!(formatted.contains("long_chat_result_assistant_context_hash=fnv64:c4b44c97efd77876"));
    assert!(formatted.contains("long_chat_result_generated_response_bytes=5"));
    assert!(formatted.contains("long_chat_result_generated_response_chunks=2"));
    assert!(formatted.contains("long_chat_result_generated_response_hash=fnv64:8ac625bb85ed202b"));
    Ok(())
}

#[test]
fn formats_long_chat_error_probe_result() {
    let result = LongChatErrorProbeResult::new(401, true, true, 256);

    assert_eq!(
        format_error_probe_result(&result),
        "long_chat_error_probe_unauthorized_status=401\nlong_chat_error_probe_reconnect_completed=true\nlong_chat_error_probe_reconnect_generated_event=true\nlong_chat_error_probe_reconnect_started_new_generation=true\nlong_chat_error_probe_max_tokens=256"
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
fn formats_long_chat_queue_probe_result() {
    let result = LongChatQueueProbeResult::new(
        "tenant-a:thread-1".to_owned(),
        "tenant-b:thread-1".to_owned(),
        64,
    );

    assert_eq!(
        format_queue_probe_result(&result),
        "long_chat_queue_probe_holder_prompt_cache_key=tenant-a:thread-1\nlong_chat_queue_probe_contender_prompt_cache_key=tenant-b:thread-1\nlong_chat_queue_probe_holder_started_streaming=true\nlong_chat_queue_probe_holder_completed=true\nlong_chat_queue_probe_contender_status=200\nlong_chat_queue_probe_contender_completed=true\nlong_chat_queue_probe_contender_generated_event=true\nlong_chat_queue_probe_contender_started_after_holder=true\nlong_chat_queue_probe_max_tokens=64"
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
            let assistant_context = if scenario.turn() == 1 {
                "seed answer".to_owned()
            } else {
                format!("generated-{}", scenario.turn() - 1)
            };
            LongChatScenarioResult::new_with_assistant_context_source_and_identity(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("length")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(100),
                        Duration::from_millis(140),
                    ]),
                    streaming_text: Some(StreamingTextSummary::new(format!(
                        "generated-{}",
                        scenario.turn()
                    ))),
                    streaming_token_ids: Some(StreamingTokenIdsSummary::new(
                        2,
                        2,
                        scenario.token_length(),
                    )),
                    streaming_usage: Some(StreamingUsageSummary::new(
                        16,
                        scenario.token_length() as u64,
                        scenario.token_length() as u64 + 16,
                    )),
                    rss: Some(RssSummary::new(1000, 2000, 1500)),
                },
                source,
                LongChatTextIdentity::from_text(&assistant_context),
            )
        })
        .collect::<Vec<_>>();
    let error_probe = LongChatErrorProbeResult::new(401, true, true, 8);
    let disconnect_probe = LongChatDisconnectProbeResult::new(true, true, 8);

    assert_eq!(
        format_run_summary(
            &config,
            &results,
            Some(&error_probe),
            Some(&disconnect_probe),
            None
        ),
        "long_chat_summary_planned_scenarios=4\nlong_chat_summary_completed_scenarios=4\nlong_chat_summary_all_finish_reasons_present=true\nlong_chat_summary_all_usage_accounting_valid=true\nlong_chat_summary_all_token_limit_status_present=true\nlong_chat_summary_any_token_limit_hit=true\nlong_chat_summary_prompt_cache_key_present=false\nlong_chat_summary_cached_follow_ups_required=false\nlong_chat_summary_any_cached_prompt_tokens=false\nlong_chat_summary_generated_follow_up_turns=3\nlong_chat_summary_cached_generated_follow_up_turns=0\nlong_chat_summary_uncached_generated_follow_up_turns=3\nlong_chat_summary_all_generated_follow_up_turns_cached=false\nlong_chat_summary_generated_follow_up_context_required=true\nlong_chat_summary_all_follow_up_turns_use_generated_context=true\nlong_chat_summary_generated_context_identity_required=true\nlong_chat_summary_generated_context_identity_links=3\nlong_chat_summary_matching_generated_context_identity_links=3\nlong_chat_summary_all_generated_context_identity_links_present=true\nlong_chat_summary_all_generated_context_identities_match_previous_response=true\nlong_chat_summary_all_timing_present=true\nlong_chat_summary_streaming_token_ids_required=true\nlong_chat_summary_all_streaming_token_id_summaries_present=true\nlong_chat_summary_all_streaming_content_chunks_have_token_ids=true\nlong_chat_summary_rss_required=true\nlong_chat_summary_all_rss_present=true\nlong_chat_summary_error_probe_required=true\nlong_chat_summary_error_probe_completed=true\nlong_chat_summary_error_probe_reconnect_started_new_generation=true\nlong_chat_summary_disconnect_probe_required=true\nlong_chat_summary_disconnect_probe_completed=true\nlong_chat_summary_disconnect_probe_reconnect_started_new_generation=true\nlong_chat_summary_queue_probe_required=false\nlong_chat_summary_queue_probe_completed=false\nlong_chat_summary_queue_probe_contender_started_after_holder=false\nlong_chat_summary_run_complete=true"
    );
    Ok(())
}

#[test]
fn queue_probe_participates_in_long_chat_run_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("1"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--stop"),
        OsString::from("x"),
        OsString::from("--prompt-cache-keys"),
        OsString::from("tenant-a:thread-1,tenant-b:thread-1"),
        OsString::from("--queue-probe"),
    ])?;
    let results = config
        .scenarios()
        .iter()
        .map(|scenario| {
            LongChatScenarioResult::new_with_assistant_context_source_and_identity(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("stop")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(10),
                    ]),
                    streaming_text: None,
                    streaming_token_ids: None,
                    streaming_usage: Some(StreamingUsageSummary::new(4, 1, 5)),
                    rss: None,
                },
                LongChatAssistantContextSource::Seed,
                LongChatTextIdentity::from_text("seed"),
            )
        })
        .collect::<Vec<_>>();
    let queue_probe = LongChatQueueProbeResult::new(
        "tenant-a:thread-1".to_owned(),
        "tenant-b:thread-1".to_owned(),
        8,
    );

    let summary = format_run_summary(&config, &results, None, None, Some(&queue_probe));

    assert!(summary.contains("long_chat_summary_queue_probe_required=true"));
    assert!(summary.contains("long_chat_summary_queue_probe_completed=true"));
    assert!(summary.contains("long_chat_summary_queue_probe_contender_started_after_holder=true"));
    assert!(summary.contains("long_chat_summary_run_complete=true"));
    Ok(())
}

#[test]
fn explicit_stop_summary_can_complete_without_generated_follow_up_context(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("1"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--stop"),
        OsString::from("1"),
        OsString::from("--expect-finish-reason"),
        OsString::from("stop"),
        OsString::from("--rss-pid"),
        OsString::from("4242"),
        OsString::from("--error-probe"),
        OsString::from("--disconnect-probe"),
    ])?;
    let results = config
        .scenarios()
        .iter()
        .map(|scenario| {
            LongChatScenarioResult::new_with_assistant_context_source_and_identity(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("stop")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(64),
                    ]),
                    streaming_text: None,
                    streaming_token_ids: None,
                    streaming_usage: Some(StreamingUsageSummary::new(18, 1, 19)),
                    rss: Some(RssSummary::new(1000, 2000, 1500)),
                },
                LongChatAssistantContextSource::Seed,
                LongChatTextIdentity::from_text("short context"),
            )
        })
        .collect::<Vec<_>>();
    let error_probe = LongChatErrorProbeResult::new(401, true, true, 32);
    let disconnect_probe = LongChatDisconnectProbeResult::new(true, true, 32);

    let summary = format_run_summary(
        &config,
        &results,
        Some(&error_probe),
        Some(&disconnect_probe),
        None,
    );

    assert!(summary.contains("long_chat_summary_all_finish_reasons_present=true"));
    assert!(summary.contains("long_chat_summary_all_usage_accounting_valid=true"));
    assert!(summary.contains("long_chat_summary_all_token_limit_status_present=true"));
    assert!(summary.contains("long_chat_summary_any_token_limit_hit=false"));
    assert!(summary.contains("long_chat_summary_all_follow_up_turns_use_generated_context=false"));
    assert!(summary.contains("long_chat_summary_all_timing_present=true"));
    assert!(summary.contains("long_chat_summary_streaming_token_ids_required=false"));
    assert!(summary.contains("long_chat_summary_all_rss_present=true"));
    assert!(summary.contains("long_chat_summary_error_probe_reconnect_started_new_generation=true"));
    assert!(summary
        .contains("long_chat_summary_disconnect_probe_reconnect_started_new_generation=true"));
    assert!(summary.contains("long_chat_summary_run_complete=true"));
    Ok(())
}

#[test]
fn error_probe_without_generated_reconnect_event_makes_summary_incomplete(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--error-probe"),
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
            let assistant_context = if scenario.turn() == 1 {
                "seed answer".to_owned()
            } else {
                format!("generated-{}", scenario.turn() - 1)
            };
            LongChatScenarioResult::new_with_assistant_context_source_and_identity(
                scenario,
                ThroughputResult {
                    completed_requests: 1,
                    elapsed: Duration::from_millis(400),
                    streaming_finish: Some(StreamingFinishSummary::new("length")),
                    streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                        Duration::from_millis(100),
                        Duration::from_millis(140),
                    ]),
                    streaming_text: Some(StreamingTextSummary::new(format!(
                        "generated-{}",
                        scenario.turn()
                    ))),
                    streaming_token_ids: Some(StreamingTokenIdsSummary::new(
                        2,
                        2,
                        scenario.token_length(),
                    )),
                    streaming_usage: Some(StreamingUsageSummary::new(
                        16,
                        scenario.token_length() as u64,
                        scenario.token_length() as u64 + 16,
                    )),
                    rss: None,
                },
                source,
                LongChatTextIdentity::from_text(&assistant_context),
            )
        })
        .collect::<Vec<_>>();
    let error_probe = LongChatErrorProbeResult::new(401, true, false, 8);

    let summary = format_run_summary(&config, &results, Some(&error_probe), None, None);

    assert!(summary.contains("long_chat_summary_error_probe_completed=true"));
    assert!(
        summary.contains("long_chat_summary_error_probe_reconnect_started_new_generation=false")
    );
    assert!(summary.contains("long_chat_summary_run_complete=false"));
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
                    streaming_token_ids: None,
                    streaming_usage: Some(usage),
                    rss: None,
                },
                source,
            )
        })
        .collect::<Vec<_>>();

    let summary = format_run_summary(&config, &results, None, None, None);

    assert!(summary.contains("long_chat_summary_prompt_cache_key_present=true"));
    assert!(summary.contains("long_chat_summary_any_cached_prompt_tokens=true"));
    assert!(summary.contains("long_chat_summary_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_cached_generated_follow_up_turns=3"));
    assert!(summary.contains("long_chat_summary_uncached_generated_follow_up_turns=0"));
    assert!(summary.contains("long_chat_summary_all_generated_follow_up_turns_cached=true"));
    Ok(())
}

#[test]
fn summarizes_generated_context_identity_continuity() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
    ])?;
    let mut turn = 0;
    let results = config.run_with_executor(|throughput| {
        turn += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                Duration::from_millis(1),
                Duration::from_millis(2),
            ]),
            streaming_text: Some(StreamingTextSummary::new(format!("generated-{turn}"))),
            streaming_token_ids: Some(StreamingTokenIdsSummary::new(1, 1, throughput.max_tokens())),
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    let summary = format_run_summary(&config, &results, None, None, None);

    assert!(summary.contains("long_chat_summary_generated_context_identity_required=true"));
    assert!(summary.contains("long_chat_summary_generated_context_identity_links=3"));
    assert!(summary.contains("long_chat_summary_matching_generated_context_identity_links=3"));
    assert!(summary.contains("long_chat_summary_all_generated_context_identity_links_present=true"));
    assert!(summary.contains(
        "long_chat_summary_all_generated_context_identities_match_previous_response=true"
    ));
    assert!(summary.contains("long_chat_summary_run_complete=true"));
    Ok(())
}

#[test]
fn summary_is_incomplete_when_generated_context_identity_is_missing(
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
                    streaming_text: Some(StreamingTextSummary::new(format!(
                        "generated-{}",
                        scenario.turn()
                    ))),
                    streaming_token_ids: Some(StreamingTokenIdsSummary::new(
                        1,
                        1,
                        scenario.token_length(),
                    )),
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

    let summary = format_run_summary(&config, &results, None, None, None);

    assert!(summary.contains("long_chat_summary_generated_context_identity_required=true"));
    assert!(summary.contains("long_chat_summary_generated_context_identity_links=0"));
    assert!(
        summary.contains("long_chat_summary_all_generated_context_identity_links_present=false")
    );
    assert!(summary.contains("long_chat_summary_run_complete=false"));
    Ok(())
}

#[test]
fn formats_prompt_cache_trace_in_long_chat_scenario_result(
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
    let usage = StreamingUsageSummary::new(16, 256, 272)
        .with_cached_prompt_tokens(8)
        .with_prompt_cache_trace(
            StreamingPromptCacheTraceSummary::new(
                "shared_prefix_hit".to_owned(),
                "fnv64:0000000000001234".to_owned(),
                5,
            )
            .with_selected_entry_token_hash("fnv64:0000000000004567".to_owned()),
        );
    let result = LongChatScenarioResult::new_with_assistant_context_source(
        &scenario,
        ThroughputResult {
            completed_requests: 1,
            elapsed: Duration::from_millis(400),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: None,
            streaming_token_ids: None,
            streaming_usage: Some(usage),
            rss: None,
        },
        LongChatAssistantContextSource::Generated,
    );
    let formatted = format_scenario_result(&result);

    assert!(formatted.contains("long_chat_result_prompt_cache_lookup=shared_prefix_hit"));
    assert!(formatted
        .contains("long_chat_result_prompt_cache_prompt_token_hash=fnv64:0000000000001234"));
    assert!(formatted.contains("long_chat_result_prompt_cache_shared_prefix_tokens=5"));
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
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(16, 256, 272)),
            rss: None,
        },
        LongChatAssistantContextSource::Seed,
    );

    let summary = format_run_summary(&config, &[result], None, None, None);

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
                    streaming_token_ids: None,
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

    let summary = format_run_summary(&config, &results, None, None, None);

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
            streaming_token_ids: None,
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
                streaming_token_ids: None,
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
            streaming_token_ids: None,
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
    let observed_identities = results
        .iter()
        .map(|result| {
            let identity = result
                .assistant_context_identity()
                .ok_or("expected assistant context identity")?;
            Ok((
                result.token_length(),
                result.turn(),
                identity.byte_len(),
                identity.formatted_hash(),
            ))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    assert_eq!(
        observed_identities,
        [
            (256, 1, 11, "fnv64:c4b44c97efd77876".to_owned()),
            (512, 1, 11, "fnv64:c4b44c97efd77876".to_owned()),
            (256, 2, 15, "fnv64:89ae2a6c06d3ddfc".to_owned()),
            (512, 2, 15, "fnv64:94eefc896813f749".to_owned()),
            (256, 3, 15, "fnv64:89ae2d6c06d3e315".to_owned()),
            (512, 3, 15, "fnv64:94eef9896813f230".to_owned()),
            (256, 4, 15, "fnv64:89ae2c6c06d3e162".to_owned()),
            (512, 4, 15, "fnv64:94eefa896813f3e3".to_owned()),
        ]
    );
    Ok(())
}

#[test]
fn validates_required_substrings_in_generated_follow_up_responses(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--require-generated-response-contains"),
        OsString::from("continuity-marker"),
    ])?;
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        calls += 1;
        let text = if calls == 1 {
            "seed response"
        } else {
            "generated response with continuity-marker"
        };
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(calls, 4);
    Ok(())
}

#[test]
fn rejects_missing_required_substring_in_generated_follow_up_response(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--require-generated-response-contains"),
        OsString::from("continuity-marker"),
    ])?;
    let mut calls = 0usize;

    let result = config.run_with_executor(|throughput| {
        calls += 1;
        let text = if calls == 1 {
            "seed response"
        } else {
            "generated response without the marker"
        };
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
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
            return Err(format!("expected response substring mismatch, got {results:?}").into())
        }
        Err(error) => error,
    };

    assert_eq!(calls, 2);
    assert!(
        error
            .to_string()
            .contains("turn 2 generated response missing required substring continuity-marker"),
        "{error}"
    );
    Ok(())
}

#[test]
fn can_window_generated_assistant_context_before_follow_up_turns(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--generated-context-max-chars"),
        OsString::from("4"),
    ])?;
    let mut observed_contexts = Vec::new();
    let generated = ["alpha-beta", "gamma-delta", "epsilon-zeta", "final"];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push(throughput.assistant_context().map(str::to_owned));
        let text = generated[calls].to_owned();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(
        observed_contexts,
        [
            Some("seed answer".to_owned()),
            Some("beta".to_owned()),
            Some("elta".to_owned()),
            Some("zeta".to_owned()),
        ]
    );
    assert!(format_plan(&config).contains("long_chat_generated_context_max_chars=4"));
    Ok(())
}

#[test]
fn can_window_generated_assistant_context_by_streaming_chunks(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--generated-context-max-tokens"),
        OsString::from("2"),
    ])?;
    let mut observed_contexts = Vec::new();
    let generated = [
        vec!["one", " two", " three"],
        vec!["four", " five", " six"],
        vec!["seven", " eight", " nine"],
        vec!["ten"],
    ];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push(throughput.assistant_context().map(str::to_owned));
        let chunks = generated[calls].clone();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::from_chunks(chunks)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(
        observed_contexts,
        [
            Some("seed answer".to_owned()),
            Some(" two three".to_owned()),
            Some(" five six".to_owned()),
            Some(" eight nine".to_owned()),
        ]
    );
    assert!(format_plan(&config).contains("long_chat_generated_context_max_tokens=2"));
    Ok(())
}

#[test]
fn can_add_state_capsule_to_generated_follow_up_contexts_only(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"{"state_anchor":"7291"}"#),
    ])?;
    let mut observed_contexts = Vec::new();
    let generated = ["alpha", "beta", "gamma", "delta"];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push(throughput.assistant_context().map(str::to_owned));
        let text = generated[calls].to_owned();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(
        observed_contexts,
        [
            Some("seed answer".to_owned()),
            Some(
                "Ferrite state capsule:\n{\"state_anchor\":\"7291\"}\n\nGenerated assistant context:\nalpha"
                    .to_owned()
            ),
            Some(
                "Ferrite state capsule:\n{\"state_anchor\":\"7291\"}\n\nGenerated assistant context:\nbeta"
                    .to_owned()
            ),
            Some(
                "Ferrite state capsule:\n{\"state_anchor\":\"7291\"}\n\nGenerated assistant context:\ngamma"
                    .to_owned()
            ),
        ]
    );
    Ok(())
}

#[test]
fn state_capsule_wrapped_assistant_context_preserves_generated_identity_summary(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"{"state_anchor":"7291"}"#),
    ])?;
    let generated = ["alpha", "beta", "gamma", "delta"];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        let text = generated[calls].to_owned();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: StreamingTimingSummary::from_event_offsets(&[
                Duration::from_millis(1),
                Duration::from_millis(2),
            ]),
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: Some(StreamingTokenIdsSummary::new(1, 1, throughput.max_tokens())),
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    let summary = format_run_summary(&config, &results, None, None, None);

    assert!(summary.contains("long_chat_summary_generated_context_identity_links=3"));
    assert!(summary.contains("long_chat_summary_matching_generated_context_identity_links=3"));
    assert!(summary.contains(
        "long_chat_summary_all_generated_context_identities_match_previous_response=true"
    ));
    assert!(summary.contains("long_chat_summary_run_complete=true"));
    Ok(())
}

#[test]
fn can_add_state_capsule_to_generated_follow_up_prompt_instead_of_assistant_context(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--follow-up"),
        OsString::from("second turn"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"state_anchor=7291"#),
        OsString::from("--generated-context-state-capsule-placement"),
        OsString::from("follow-up"),
    ])?;
    let mut observed_contexts = Vec::new();
    let mut observed_follow_ups = Vec::new();
    let generated = ["alpha", "beta", "gamma", "delta"];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push(throughput.assistant_context().map(str::to_owned));
        observed_follow_ups.push(throughput.follow_up().map(str::to_owned));
        let text = generated[calls].to_owned();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(
        observed_contexts,
        [
            Some("seed answer".to_owned()),
            Some("alpha".to_owned()),
            Some("beta".to_owned()),
            Some("gamma".to_owned()),
        ]
    );
    assert_eq!(
        observed_follow_ups,
        [
            Some("second turn".to_owned()),
            Some(
                "Ferrite state capsule:\nstate_anchor=7291\n\nFollow-up instruction:\nsecond turn"
                    .to_owned()
            ),
            Some(
                "Ferrite state capsule:\nstate_anchor=7291\n\nFollow-up instruction:\nsecond turn"
                    .to_owned()
            ),
            Some(
                "Ferrite state capsule:\nstate_anchor=7291\n\nFollow-up instruction:\nsecond turn"
                    .to_owned()
            ),
        ]
    );
    Ok(())
}

#[test]
fn can_use_state_capsule_as_generated_follow_up_context_without_retained_prose(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--models"),
        OsString::from("fixture-model"),
        OsString::from("--token-lengths"),
        OsString::from("256"),
        OsString::from("--turns"),
        OsString::from("4"),
        OsString::from("--assistant-context"),
        OsString::from("seed answer"),
        OsString::from("--follow-up"),
        OsString::from("second turn"),
        OsString::from("--generated-context-state-capsule"),
        OsString::from(r#"state_anchor=7291"#),
        OsString::from("--generated-context-state-capsule-placement"),
        OsString::from("assistant-context-only"),
    ])?;
    let mut observed_contexts = Vec::new();
    let mut observed_follow_ups = Vec::new();
    let generated = [
        "alpha retained prose",
        "beta retained prose",
        "gamma retained prose",
        "delta retained prose",
    ];
    let mut calls = 0usize;

    let results = config.run_with_executor(|throughput| {
        observed_contexts.push(throughput.assistant_context().map(str::to_owned));
        observed_follow_ups.push(throughput.follow_up().map(str::to_owned));
        let text = generated[calls].to_owned();
        calls += 1;
        Ok(ThroughputResult {
            completed_requests: throughput.requests(),
            elapsed: Duration::from_millis(10),
            streaming_finish: Some(StreamingFinishSummary::new("length")),
            streaming_timing: None,
            streaming_text: Some(StreamingTextSummary::new(text)),
            streaming_token_ids: None,
            streaming_usage: Some(StreamingUsageSummary::new(
                8,
                throughput.max_tokens() as u64,
                throughput.max_tokens() as u64 + 8,
            )),
            rss: None,
        })
    })?;

    assert_eq!(results.len(), 4);
    assert_eq!(
        observed_contexts,
        [
            Some("seed answer".to_owned()),
            Some("Ferrite state capsule:\nstate_anchor=7291".to_owned()),
            Some("Ferrite state capsule:\nstate_anchor=7291".to_owned()),
            Some("Ferrite state capsule:\nstate_anchor=7291".to_owned()),
        ]
    );
    assert_eq!(
        observed_follow_ups,
        [
            Some("second turn".to_owned()),
            Some("second turn".to_owned()),
            Some("second turn".to_owned()),
            Some("second turn".to_owned()),
        ]
    );
    Ok(())
}

#[test]
fn rejects_invalid_state_capsule_placement() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--generated-context-state-capsule-placement"),
        OsString::from("system"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("--generated-context-state-capsule-placement"),
        "{error}"
    );
    Ok(())
}

#[test]
fn rejects_combined_generated_context_char_and_token_windows(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--generated-context-max-chars"),
        OsString::from("128"),
        OsString::from("--generated-context-max-tokens"),
        OsString::from("32"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("cannot be combined with --generated-context-max-tokens"),
        "{error}"
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
            streaming_token_ids: None,
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
            streaming_token_ids: None,
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

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("{name}-{}-{nanos}", std::process::id()))
}
