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
    let all_timing_present = results
        .iter()
        .all(|result| result.throughput().streaming_timing.is_some());
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
    let run_complete = completed_scenarios == planned_scenarios
        && all_finish_reasons_present
        && all_usage_accounting_valid
        && all_token_limit_status_present
        && all_timing_present
        && (!rss_required || all_rss_present)
        && (!error_probe_required || error_probe_completed)
        && (!disconnect_probe_required || disconnect_probe_completed);

    format!(
        "long_chat_summary_planned_scenarios={planned_scenarios}\n\
long_chat_summary_completed_scenarios={completed_scenarios}\n\
long_chat_summary_all_finish_reasons_present={all_finish_reasons_present}\n\
long_chat_summary_all_usage_accounting_valid={all_usage_accounting_valid}\n\
long_chat_summary_all_token_limit_status_present={all_token_limit_status_present}\n\
long_chat_summary_any_token_limit_hit={any_token_limit_hit}\n\
long_chat_summary_all_timing_present={all_timing_present}\n\
long_chat_summary_rss_required={rss_required}\n\
long_chat_summary_all_rss_present={all_rss_present}\n\
long_chat_summary_error_probe_required={error_probe_required}\n\
long_chat_summary_error_probe_completed={error_probe_completed}\n\
long_chat_summary_disconnect_probe_required={disconnect_probe_required}\n\
long_chat_summary_disconnect_probe_completed={disconnect_probe_completed}\n\
long_chat_summary_run_complete={run_complete}"
    )
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
