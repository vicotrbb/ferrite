use super::{
    LongChatDisconnectProbeResult, LongChatErrorProbeResult, LongChatGateConfig,
    LongChatScenarioResult,
};

pub fn format_run_summary(
    config: &LongChatGateConfig,
    results: &[LongChatScenarioResult],
    error_probe: Option<&LongChatErrorProbeResult>,
    disconnect_probe: Option<&LongChatDisconnectProbeResult>,
) -> String {
    let planned_scenarios = config.planned_scenarios();
    let completed_scenarios = results.len();
    let all_finish_reasons_present = results
        .iter()
        .all(|result| result.throughput().streaming_finish.is_some());
    let all_usage_accounting_valid = results.iter().all(usage_accounting_valid);
    let all_token_limit_status_present = results
        .iter()
        .all(|result| result.hit_token_limit().is_some());
    let any_token_limit_hit = results
        .iter()
        .any(|result| result.hit_token_limit().unwrap_or(false));
    let prompt_cache_key_present = config.prompt_cache_key().is_some();
    let any_cached_prompt_tokens = results.iter().any(has_cached_prompt_tokens);
    let generated_follow_up_turns = results
        .iter()
        .filter(|result| is_generated_follow_up_turn(result))
        .count();
    let cached_generated_follow_up_turns = results
        .iter()
        .filter(|result| is_generated_follow_up_turn(result) && has_cached_prompt_tokens(result))
        .count();
    let uncached_generated_follow_up_turns =
        generated_follow_up_turns - cached_generated_follow_up_turns;
    let all_generated_follow_up_turns_cached = prompt_cache_key_present
        && generated_follow_up_turns > 0
        && generated_follow_up_turns == cached_generated_follow_up_turns;
    let cached_follow_ups_required = config.require_cached_follow_ups();
    let all_follow_up_turns_use_generated_context = results
        .iter()
        .all(|result| result.turn() == 1 || result.assistant_context_source().is_generated());
    let all_timing_present = results
        .iter()
        .all(|result| result.throughput().streaming_timing.is_some());
    let streaming_token_ids_required = config.stop().is_none();
    let all_streaming_token_id_summaries_present = results
        .iter()
        .all(|result| result.throughput().streaming_token_ids.is_some());
    let all_streaming_content_chunks_have_token_ids =
        results.iter().all(has_token_ids_for_all_content_chunks);
    let rss_required = config.rss_pid().is_some();
    let all_rss_present = results
        .iter()
        .all(|result| result.throughput().rss.is_some());
    let error_probe_required = config.error_probe();
    let error_probe_completed = error_probe
        .is_some_and(|probe| probe.unauthorized_status() == 401 && probe.reconnect_completed());
    let disconnect_probe_required = config.disconnect_probe();
    let disconnect_probe_completed = disconnect_probe
        .is_some_and(|probe| probe.aborted_after_generated_event() && probe.reconnect_completed());
    let disconnect_probe_reconnect_started_new_generation =
        disconnect_probe.is_some_and(|probe| probe.reconnect_started_new_generation());
    let run_complete = completed_scenarios == planned_scenarios
        && all_finish_reasons_present
        && all_usage_accounting_valid
        && all_token_limit_status_present
        && all_follow_up_turns_use_generated_context
        && (!cached_follow_ups_required || all_generated_follow_up_turns_cached)
        && all_timing_present
        && (!streaming_token_ids_required
            || (all_streaming_token_id_summaries_present
                && all_streaming_content_chunks_have_token_ids))
        && (!rss_required || all_rss_present)
        && (!error_probe_required || error_probe_completed)
        && (!disconnect_probe_required
            || (disconnect_probe_completed && disconnect_probe_reconnect_started_new_generation));

    format!(
        "long_chat_summary_planned_scenarios={planned_scenarios}\n\
long_chat_summary_completed_scenarios={completed_scenarios}\n\
long_chat_summary_all_finish_reasons_present={all_finish_reasons_present}\n\
long_chat_summary_all_usage_accounting_valid={all_usage_accounting_valid}\n\
long_chat_summary_all_token_limit_status_present={all_token_limit_status_present}\n\
long_chat_summary_any_token_limit_hit={any_token_limit_hit}\n\
long_chat_summary_prompt_cache_key_present={prompt_cache_key_present}\n\
long_chat_summary_cached_follow_ups_required={cached_follow_ups_required}\n\
long_chat_summary_any_cached_prompt_tokens={any_cached_prompt_tokens}\n\
long_chat_summary_generated_follow_up_turns={generated_follow_up_turns}\n\
long_chat_summary_cached_generated_follow_up_turns={cached_generated_follow_up_turns}\n\
long_chat_summary_uncached_generated_follow_up_turns={uncached_generated_follow_up_turns}\n\
long_chat_summary_all_generated_follow_up_turns_cached={all_generated_follow_up_turns_cached}\n\
long_chat_summary_all_follow_up_turns_use_generated_context={all_follow_up_turns_use_generated_context}\n\
long_chat_summary_all_timing_present={all_timing_present}\n\
long_chat_summary_streaming_token_ids_required={streaming_token_ids_required}\n\
long_chat_summary_all_streaming_token_id_summaries_present={all_streaming_token_id_summaries_present}\n\
long_chat_summary_all_streaming_content_chunks_have_token_ids={all_streaming_content_chunks_have_token_ids}\n\
long_chat_summary_rss_required={rss_required}\n\
long_chat_summary_all_rss_present={all_rss_present}\n\
long_chat_summary_error_probe_required={error_probe_required}\n\
long_chat_summary_error_probe_completed={error_probe_completed}\n\
long_chat_summary_disconnect_probe_required={disconnect_probe_required}\n\
long_chat_summary_disconnect_probe_completed={disconnect_probe_completed}\n\
long_chat_summary_disconnect_probe_reconnect_started_new_generation={disconnect_probe_reconnect_started_new_generation}\n\
long_chat_summary_run_complete={run_complete}"
    )
}

fn has_cached_prompt_tokens(result: &LongChatScenarioResult) -> bool {
    result
        .throughput()
        .streaming_usage
        .is_some_and(|usage| usage.cached_prompt_tokens() > 0)
}

fn is_generated_follow_up_turn(result: &LongChatScenarioResult) -> bool {
    result.turn() > 1 && result.assistant_context_source().is_generated()
}

fn has_token_ids_for_all_content_chunks(result: &LongChatScenarioResult) -> bool {
    result
        .throughput()
        .streaming_token_ids
        .is_some_and(|summary| summary.all_content_chunks_have_token_ids())
}

fn usage_accounting_valid(result: &LongChatScenarioResult) -> bool {
    let Some(finish) = &result.throughput().streaming_finish else {
        return false;
    };
    let Some(usage) = result.throughput().streaming_usage else {
        return false;
    };

    match finish.reason() {
        "length" => usage.completion_tokens() == result.token_length() as u64,
        "stop" => usage.completion_tokens() <= result.token_length() as u64,
        _ => false,
    }
}
