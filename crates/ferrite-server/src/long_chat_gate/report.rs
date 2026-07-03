use super::LongChatGateConfig;

pub fn format_plan(config: &LongChatGateConfig) -> String {
    let models = config.models().join(",");
    let required_models = if config.required_models().is_empty() {
        String::new()
    } else {
        format!(
            "\nlong_chat_required_models={}",
            config.required_models().join(",")
        )
    };
    let token_lengths = config
        .token_lengths()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let prompt_cache_key = config
        .prompt_cache_key()
        .map(|key| format!("\nlong_chat_prompt_cache_key={key}"))
        .unwrap_or_default();
    let prompt_cache_keys = if config.prompt_cache_keys().is_empty() {
        String::new()
    } else {
        format!(
            "\nlong_chat_prompt_cache_keys={}",
            config.prompt_cache_keys().join(",")
        )
    };
    let generated_context_max_chars = config
        .generated_context_max_chars()
        .map(|chars| format!("\nlong_chat_generated_context_max_chars={chars}"))
        .unwrap_or_default();
    let generated_context_max_tokens = config
        .generated_context_max_tokens()
        .map(|tokens| format!("\nlong_chat_generated_context_max_tokens={tokens}"))
        .unwrap_or_default();
    let generated_context_state_capsule = if config.generated_context_state_capsule().is_some() {
        "\nlong_chat_generated_context_state_capsule_configured=true"
    } else {
        ""
    };
    let generated_context_state_capsule_placement =
        if config.generated_context_state_capsule().is_some() {
            format!(
                "\nlong_chat_generated_context_state_capsule_placement={}",
                config.generated_context_state_capsule_placement().as_str()
            )
        } else {
            String::new()
        };
    let required_generated_response_substrings =
        if config.required_generated_response_substrings().is_empty() {
            String::new()
        } else {
            format!(
                "\nlong_chat_required_generated_response_substrings={}",
                config.required_generated_response_substrings().join(",")
            )
        };
    let addr = format!("\nlong_chat_addr={}", config.addr());
    let execute = if config.execute() {
        "\nlong_chat_execute=true"
    } else {
        ""
    };
    let require_cached_follow_ups = if config.require_cached_follow_ups() {
        "\nlong_chat_require_cached_follow_ups=true"
    } else {
        ""
    };
    let stop_configured = if config.stop().is_some() {
        "\nlong_chat_stop_configured=true"
    } else {
        ""
    };
    let expected_finish_reason = config
        .expected_finish_reason()
        .map(|reason| format!("\nlong_chat_expected_finish_reason={reason}"))
        .unwrap_or_default();
    let rss_pid = config
        .rss_pid()
        .map(|pid| format!("\nlong_chat_rss_pid={pid}"))
        .unwrap_or_default();
    let proof_log_path = config
        .proof_log_path()
        .map(|path| format!("\nlong_chat_proof_log_path={}", path.to_string_lossy()))
        .unwrap_or_default();
    let proof_exit_code_path = config
        .proof_exit_code_path()
        .map(|path| {
            format!(
                "\nlong_chat_proof_exit_code_path={}",
                path.to_string_lossy()
            )
        })
        .unwrap_or_default();
    let error_probe_required = if config.error_probe() {
        "\nlong_chat_error_probe_required=true"
    } else {
        ""
    };
    let disconnect_probe_required = if config.disconnect_probe() {
        "\nlong_chat_disconnect_probe_required=true"
    } else {
        ""
    };
    let queue_probe_required = if config.queue_probe() {
        "\nlong_chat_queue_probe_required=true"
    } else {
        ""
    };
    let required_probes = if config.required_probes().is_empty() {
        String::new()
    } else {
        format!(
            "\nlong_chat_required_probes={}",
            config
                .required_probes()
                .iter()
                .map(|probe| probe.as_str())
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let probe_max_tokens = config
        .probe_max_tokens()
        .map(|tokens| format!("\nlong_chat_probe_max_tokens={tokens}"))
        .unwrap_or_default();
    let required_token_lengths = if config.required_token_lengths().is_empty() {
        String::new()
    } else {
        format!(
            "\nlong_chat_required_token_lengths={}",
            format_usize_list(config.required_token_lengths())
        )
    };
    let disconnect_reconnect_timeout = if config.disconnect_probe() {
        format!(
            "\nlong_chat_disconnect_reconnect_timeout_ms={}",
            config.disconnect_reconnect_timeout().as_millis()
        )
    } else {
        String::new()
    };
    format!(
        "long_chat_models={models}{}\nlong_chat_token_lengths={token_lengths}\nlong_chat_turns={}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\nlong_chat_planned_scenarios={}",
        required_models,
        config.turns(),
        prompt_cache_key,
        prompt_cache_keys,
        generated_context_max_chars,
        generated_context_max_tokens,
        generated_context_state_capsule,
        generated_context_state_capsule_placement,
        required_generated_response_substrings,
        proof_log_path,
        proof_exit_code_path,
        addr,
        execute,
        require_cached_follow_ups,
        stop_configured,
        expected_finish_reason,
        rss_pid,
        error_probe_required,
        disconnect_probe_required,
        queue_probe_required,
        required_probes,
        probe_max_tokens,
        required_token_lengths,
        disconnect_reconnect_timeout,
        config.planned_scenarios()
    )
}

fn format_usize_list(values: &[usize]) -> String {
    values
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

pub fn format_scenarios(config: &LongChatGateConfig) -> String {
    config
        .scenarios()
        .iter()
        .map(|scenario| {
            let prompt_cache_key = scenario
                .prompt_cache_key()
                .map(|key| format!(",prompt_cache_key:{key}"))
                .unwrap_or_default();
            format!(
                "long_chat_scenario=model:{},turn:{},max_tokens:{}{}",
                scenario.model(),
                scenario.turn(),
                scenario.token_length(),
                prompt_cache_key
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_report(config: &LongChatGateConfig) -> String {
    format!("{}\n{}", format_plan(config), format_scenarios(config))
}
