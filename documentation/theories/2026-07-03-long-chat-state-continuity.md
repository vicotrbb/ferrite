# Long Chat State Continuity Theories

Date: 2026-07-03 UTC, 2026-07-02 local time

## Scope

These probes used Ferrite's OpenAI-compatible HTTP server with:

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Node: `homelab-01`, `x86_64`, AVX2
- Server: `ferrite-server`
- Gate: `ferrite-openai-long-chat-gate`
- Endpoint path: `/v1/models` and `/v1/chat/completions`
- Streaming response length: 256 tokens per turn
- Conversation length: 4 turns
- Generated assistant context window: 64 tokens
- Required continuity anchor: `7291`

This is not the full long-chat milestone. It only compares two state-anchor
placement theories at the previously failing 64-token generated-context window.

## Theory A: Follow-Up State Capsule Alone

Hypothesis: placing the state capsule in the generated follow-up prompt gives
the anchor enough user-message authority to survive the 64-token generated
context window.

Command shape:

```text
ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --require-cached-follow-ups \
  --models qwen2.5-1.5b-q8 \
  --token-lengths 256 \
  --turns 4 \
  --prompt-cache-key followup-state-capsule-64 \
  --expect-finish-reason length \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule state_anchor=7291 \
  --generated-context-state-capsule-placement follow-up \
  --require-generated-response-contains 7291 \
  --rss-pid 1676
```

Result: failed.

Evidence:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_result=model:qwen2.5-1.5b-q8,turn:1,max_tokens:256
long_chat_result_finish_reason=length
long_chat_result_streaming_token_ids=256
long_chat_result_streaming_all_content_chunks_have_token_ids=true
turn 2 generated response missing required substring 7291
```

Interpretation: capsule placement alone is not enough. The model can ignore a
state capsule when the task text asks only to continue the topic. This falsifies
the theory that moving the same capsule from assistant context to follow-up text
is sufficient.

## Theory B: Follow-Up Capsule With Response Contract

Hypothesis: the anchor survives if the follow-up prompt makes the state anchor
part of the response contract, not only hidden state context.

Command shape:

```text
ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --require-cached-follow-ups \
  --models qwen2.5-1.5b-q8 \
  --token-lengths 256 \
  --turns 4 \
  --prompt-cache-key followup-contract-64-clean \
  --expect-finish-reason length \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "state_anchor=7291;response_contract=include_state_anchor_in_first_sentence" \
  --generated-context-state-capsule-placement follow-up \
  --follow-up "Continue with the operational risks for a long streaming chat. In the first sentence, include the exact text state_anchor=7291." \
  --require-generated-response-contains 7291 \
  --rss-pid 1828
```

Result: passed.

Evidence:

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
EXIT=0
```

Per-turn observations:

```text
turn 1: prompt_tokens=60, cached_prompt_tokens=0, completion_tokens=256, streaming_tokens_per_second=3.307920, rss_after=1949163520
turn 2: prompt_tokens=140, cached_prompt_tokens=12, completion_tokens=256, streaming_tokens_per_second=2.646405, rss_after=1963712512
turn 3: prompt_tokens=138, cached_prompt_tokens=12, completion_tokens=256, streaming_tokens_per_second=2.648788, rss_after=1971838976
turn 4: prompt_tokens=138, cached_prompt_tokens=138, completion_tokens=256, streaming_tokens_per_second=3.897011, rss_after=1971838976
```

Interpretation: a response contract is materially stronger than state text
alone for this small model. The result proves the server and gate can carry
generated context, cache follow-up turns, stream token IDs, sample RSS, recover
from the disconnect probe, and enforce the anchor over four turns. It does not
prove natural conversation memory quality, because the anchor is explicitly
requested in the follow-up instruction.

## Operational Finding

One proof attempt lost its Kubernetes exec stream with:

```text
read tcp ...: read: connection reset by peer
```

The gate process survived inside the pod, but its output stream was lost. A
subsequent logged run initially got:

```text
expected reconnect probe status 200, got 429
```

Root cause: the killed client left the single-generation server with an in-flight
request. Restarting `ferrite-server` cleared the contaminated proof environment.

Future long-running proof gates should write stdout and exit code to files
inside the pod, then poll those files. This avoids losing proof evidence when
the Kubernetes API stream resets.

## Next Theories

1. Contract minimization: reduce the explicit contract until it stops passing.
   Start with "include `7291` once" and compare against "remember state anchor".
2. Role placement: add a proof-only system-message capsule path and compare it
   with assistant-context and follow-up placements.
3. Capsule syntax: compare plain key/value, JSON, fenced block, and XML-ish
   tags while holding response contract constant.
4. Window pressure: rerun passing contract at 32, 64, 128, and 256 generated
   context tokens to find where latency, RSS, and anchor stability shift.
5. Full milestone: extend from 256-token responses to 512 and 1024 tokens after
   raising the server hard max for the proof pod.
6. Benchmark harness: evaluate `llama-benchy` for OpenAI-compatible endpoint
   measurements across prompt processing, token generation, depth, concurrency,
   JSON/CSV output, and per-token throughput time series. This should complement
   Ferrite's correctness gate; it should not replace the long-chat gate because
   it currently targets `/v1/chat/completions` benchmark metrics rather than
   repeated multi-turn state assertions.
