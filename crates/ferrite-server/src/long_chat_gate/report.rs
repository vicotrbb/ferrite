use super::LongChatGateConfig;

pub fn format_plan(config: &LongChatGateConfig) -> String {
    let models = config.models().join(",");
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
    let probe_max_tokens = config
        .probe_max_tokens()
        .map(|tokens| format!("\nlong_chat_probe_max_tokens={tokens}"))
        .unwrap_or_default();
    let disconnect_reconnect_timeout = if config.disconnect_probe() {
        format!(
            "\nlong_chat_disconnect_reconnect_timeout_ms={}",
            config.disconnect_reconnect_timeout().as_millis()
        )
    } else {
        String::new()
    };
    format!(
        "long_chat_models={models}\nlong_chat_token_lengths={token_lengths}\nlong_chat_turns={}{}{}{}{}{}{}{}{}{}{}{}\nlong_chat_planned_scenarios={}",
        config.turns(),
        prompt_cache_key,
        addr,
        execute,
        require_cached_follow_ups,
        stop_configured,
        expected_finish_reason,
        rss_pid,
        error_probe_required,
        disconnect_probe_required,
        probe_max_tokens,
        disconnect_reconnect_timeout,
        config.planned_scenarios()
    )
}

pub fn format_scenarios(config: &LongChatGateConfig) -> String {
    config
        .scenarios()
        .iter()
        .map(|scenario| {
            format!(
                "long_chat_scenario=model:{},turn:{},max_tokens:{}",
                scenario.model(),
                scenario.turn(),
                scenario.token_length()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_report(config: &LongChatGateConfig) -> String {
    format!("{}\n{}", format_plan(config), format_scenarios(config))
}
