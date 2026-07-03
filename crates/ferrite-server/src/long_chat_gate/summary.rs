use super::{
    LongChatDisconnectProbeResult, LongChatErrorProbeResult, LongChatGateConfig,
    LongChatQueueProbeResult, LongChatScenarioResult, LongChatTextIdentity,
};
use std::collections::HashMap;

pub fn format_run_summary(
    config: &LongChatGateConfig,
    results: &[LongChatScenarioResult],
    error_probe: Option<&LongChatErrorProbeResult>,
    disconnect_probe: Option<&LongChatDisconnectProbeResult>,
    queue_probe: Option<&LongChatQueueProbeResult>,
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
    let prompt_cache_key_present =
        config.prompt_cache_key().is_some() || !config.prompt_cache_keys().is_empty();
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
    let generated_follow_up_context_required = config.stop().is_none();
    let all_follow_up_turns_use_generated_context = results
        .iter()
        .all(|result| result.turn() == 1 || result.assistant_context_source().is_generated());
    let generated_context_identity = summarize_generated_context_identity(results);
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
    let error_probe_reconnect_started_new_generation =
        error_probe.is_some_and(|probe| probe.reconnect_started_new_generation());
    let disconnect_probe_required = config.disconnect_probe();
    let disconnect_probe_completed = disconnect_probe
        .is_some_and(|probe| probe.aborted_after_generated_event() && probe.reconnect_completed());
    let disconnect_probe_reconnect_started_new_generation =
        disconnect_probe.is_some_and(|probe| probe.reconnect_started_new_generation());
    let queue_probe_required = config.queue_probe();
    let queue_probe_completed = queue_probe.is_some_and(LongChatQueueProbeResult::completed);
    let queue_probe_contender_started_after_holder =
        queue_probe.is_some_and(LongChatQueueProbeResult::contender_started_after_holder);
    let run_complete = completed_scenarios == planned_scenarios
        && all_finish_reasons_present
        && all_usage_accounting_valid
        && all_token_limit_status_present
        && (!generated_follow_up_context_required || all_follow_up_turns_use_generated_context)
        && (!generated_context_identity.required
            || generated_context_identity.all_links_present_and_matching())
        && (!cached_follow_ups_required || all_generated_follow_up_turns_cached)
        && all_timing_present
        && (!streaming_token_ids_required
            || (all_streaming_token_id_summaries_present
                && all_streaming_content_chunks_have_token_ids))
        && (!rss_required || all_rss_present)
        && (!error_probe_required
            || (error_probe_completed && error_probe_reconnect_started_new_generation))
        && (!disconnect_probe_required
            || (disconnect_probe_completed && disconnect_probe_reconnect_started_new_generation))
        && (!queue_probe_required
            || (queue_probe_completed && queue_probe_contender_started_after_holder));

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
long_chat_summary_generated_follow_up_context_required={generated_follow_up_context_required}\n\
long_chat_summary_all_follow_up_turns_use_generated_context={all_follow_up_turns_use_generated_context}\n\
long_chat_summary_generated_context_identity_required={}\n\
long_chat_summary_generated_context_identity_links={}\n\
long_chat_summary_matching_generated_context_identity_links={}\n\
long_chat_summary_all_generated_context_identity_links_present={}\n\
long_chat_summary_all_generated_context_identities_match_previous_response={}\n\
long_chat_summary_all_timing_present={all_timing_present}\n\
long_chat_summary_streaming_token_ids_required={streaming_token_ids_required}\n\
long_chat_summary_all_streaming_token_id_summaries_present={all_streaming_token_id_summaries_present}\n\
long_chat_summary_all_streaming_content_chunks_have_token_ids={all_streaming_content_chunks_have_token_ids}\n\
long_chat_summary_rss_required={rss_required}\n\
long_chat_summary_all_rss_present={all_rss_present}\n\
long_chat_summary_error_probe_required={error_probe_required}\n\
long_chat_summary_error_probe_completed={error_probe_completed}\n\
long_chat_summary_error_probe_reconnect_started_new_generation={error_probe_reconnect_started_new_generation}\n\
long_chat_summary_disconnect_probe_required={disconnect_probe_required}\n\
long_chat_summary_disconnect_probe_completed={disconnect_probe_completed}\n\
long_chat_summary_disconnect_probe_reconnect_started_new_generation={disconnect_probe_reconnect_started_new_generation}\n\
long_chat_summary_queue_probe_required={queue_probe_required}\n\
long_chat_summary_queue_probe_completed={queue_probe_completed}\n\
long_chat_summary_queue_probe_contender_started_after_holder={queue_probe_contender_started_after_holder}\n\
long_chat_summary_run_complete={run_complete}",
        generated_context_identity.required,
        generated_context_identity.links,
        generated_context_identity.matching_links,
        generated_context_identity.all_links_present(),
        generated_context_identity.all_links_present_and_matching(),
    )
}

fn has_cached_prompt_tokens(result: &LongChatScenarioResult) -> bool {
    result
        .throughput()
        .streaming_usage
        .as_ref()
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
    let Some(usage) = &result.throughput().streaming_usage else {
        return false;
    };

    match finish.reason() {
        "length" => usage.completion_tokens() == result.token_length() as u64,
        "stop" => usage.completion_tokens() <= result.token_length() as u64,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GeneratedContextIdentitySummary {
    required: bool,
    expected_links: usize,
    links: usize,
    matching_links: usize,
}

impl GeneratedContextIdentitySummary {
    fn all_links_present(self) -> bool {
        self.required && self.links == self.expected_links
    }

    fn all_links_present_and_matching(self) -> bool {
        self.all_links_present() && self.matching_links == self.expected_links
    }
}

fn summarize_generated_context_identity(
    results: &[LongChatScenarioResult],
) -> GeneratedContextIdentitySummary {
    let expected_links = results
        .iter()
        .filter(|result| is_generated_follow_up_turn(result))
        .count();
    let mut summary = GeneratedContextIdentitySummary {
        required: expected_links > 0,
        expected_links,
        links: 0,
        matching_links: 0,
    };
    let mut previous_response_by_lane =
        HashMap::<(String, usize, Option<String>), LongChatTextIdentity>::new();

    for result in results {
        let lane = (
            result.model().to_owned(),
            result.token_length(),
            result.prompt_cache_key().map(str::to_owned),
        );
        if is_generated_follow_up_turn(result) {
            let current = result.assistant_context_identity();
            let previous = previous_response_by_lane.get(&lane).copied();
            if let (Some(current), Some(previous)) = (current, previous) {
                summary.links += 1;
                if current == previous {
                    summary.matching_links += 1;
                }
            }
        }
        if let Some(text) = &result.throughput().streaming_text {
            previous_response_by_lane.insert(lane, LongChatTextIdentity::from_text(text.text()));
        }
    }

    summary
}
