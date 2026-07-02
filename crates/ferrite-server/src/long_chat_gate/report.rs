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
    let require_cached_follow_ups = if config.require_cached_follow_ups() {
        "\nlong_chat_require_cached_follow_ups=true"
    } else {
        ""
    };
    format!(
        "long_chat_models={models}\nlong_chat_token_lengths={token_lengths}\nlong_chat_turns={}{}{}\nlong_chat_planned_scenarios={}",
        config.turns(),
        prompt_cache_key,
        require_cached_follow_ups,
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
