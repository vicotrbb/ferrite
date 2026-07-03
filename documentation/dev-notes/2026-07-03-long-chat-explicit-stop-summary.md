# 2026-07-03 Long-Chat Explicit Stop Summary

## Context

The current lifecycle-instrumented explicit-stop proof for Qwen2.5 0.5B Q4_K_M
completed all stop scenarios with `finish_reason=stop`, valid usage, RSS
samples, and error/disconnect probes. The run still reported
`long_chat_summary_run_complete=false`.

The failing proof artifact was:

`target/proof/local-qwen05-lifecycle-stop-gate-2026-07-03/long-chat-stop.log`

The relevant summary lines were:

```text
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_all_follow_up_turns_use_generated_context=false
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=false
```

## Root Cause

Normal long-chat runs require follow-up turns to use generated assistant
context. That invariant proves repeated generated-context continuity.

Explicit stop-sequence runs are different. The stop filter can remove all
visible generated text, leaving no generated assistant content to carry into
the next turn. In that proof shape, requiring generated follow-up context is
not a correctness invariant; it incorrectly marks a valid stop-behavior proof
as incomplete.

## Change

Commit `3e74d37` adds a distinct summary requirement:

```text
long_chat_summary_generated_follow_up_context_required=<true|false>
```

The run-complete check now requires generated follow-up context only when no
explicit `--stop` sequence is configured. Normal long-chat runs still require
generated follow-up context. Explicit stop-sequence runs can complete without
generated follow-up context as long as their stop, timing, RSS, usage,
reconnect/error, and disconnect checks pass.

## Tests

The fix was test-driven:

1. Added
   `explicit_stop_summary_can_complete_without_generated_follow_up_context`.
2. Confirmed it failed on the previous implementation because
   `long_chat_summary_run_complete=true` was missing.
3. Implemented the minimal summary rule.
4. Re-ran the focused test and the full long-chat gate suite.

Validation:

```sh
cargo fmt -- --check
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

The full suite passed with `55 passed`.

## Follow-Up Proof

The fixed runtime proof is documented in:

`documentation/benchmarks/2026-07-03-local-qwen-0-5b-lifecycle-stop-gate.md`

That run reports:

```text
long_chat_summary_generated_follow_up_context_required=false
long_chat_summary_all_follow_up_turns_use_generated_context=false
long_chat_summary_run_complete=true
```
