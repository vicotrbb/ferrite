# OpenAI Long-Chat Token-Limit Status

## Context

The Tier 1 OpenAI long-chat gate requires each multi-turn scenario to record
whether a turn hit the configured completion-token limit. Previous harness
output recorded `finish_reason`, usage counts, timing, and RSS samples, but did
not expose a dedicated machine-readable token-limit status.

## Change

`ferrite-openai-long-chat-gate` now derives token-limit status from the
streaming terminal finish reason and reported usage:

- `long_chat_result_hit_token_limit=true` when `finish_reason=length` and
  reported `completion_tokens` equals the scenario `max_tokens`;
- `long_chat_result_hit_token_limit=false` when a scenario finishes before the
  token budget, such as `finish_reason=stop`;
- no per-result token-limit field when the finish reason or usage data is
  missing.

The integrated run summary now also reports:

- `long_chat_summary_all_token_limit_status_present`;
- `long_chat_summary_any_token_limit_hit`.

`long_chat_summary_run_complete=true` now requires token-limit status to be
present for every scenario, matching the gate contract.

## Validation

```sh
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result:

```text
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Remaining Scope

This is harness instrumentation only. Existing benchmark notes remain valid for
their historical runs, but future long-chat proof runs should capture the new
token-limit status fields in their raw logs and benchmark summaries.
