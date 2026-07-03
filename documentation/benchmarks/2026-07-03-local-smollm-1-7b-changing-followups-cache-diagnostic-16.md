# Benchmark: Local SmolLM2 1.7B Changing Follow-Ups Cache Diagnostic 16

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Test the SmolLM2 EOS fixed-point cache theory with semantically changing
follow-up questions. The fixed-answer EOS lane reached exact prompt-cache hits
after the generated response stabilized. This diagnostic asks different capital
questions per turn to check whether exact hits still appear when generated
responses and prompt hashes keep changing.

## Environment

- Ferrite runtime code commit: `dccd6fff57e3c37e5786f64ae42bc9dff88a1736`
- Host: local macOS workspace
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: `release`
- Server: `127.0.0.1:18242`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Proof directory:
  `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/`
- Strict all-EOS attempt directory:
  `target/proof/local-smollm17-eos-changing-followups-16-2026-07-03/`
- Server binary SHA256:
  `0428e6f820b27f98bbc82bf7a2189e5586d378372a949117743e04f6849dc5a6`
- Gate binary SHA256:
  `f801c623ef7132ab75356010b896536a6ab04ddb82c2276f6dad70f58fbe2f7a`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

The local server was stopped after the run. A bind-specific listener check
returned `listener_present=false` for `127.0.0.1:18242`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18242 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18242 \
  --api-key local-secret \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --require-models SmolLM2-1.7B-Instruct-Q4_K_M \
  --prompt 'Question: What is the capital of France? Answer only with the city name.' \
  --assistant-context 'Paris.' \
  --follow-ups 'Question: What is the capital of France? Answer only with the city name.,Question: What is the capital of Germany? Answer only with the city name.,Question: What is the capital of Italy? Answer only with the city name.,Question: What is the capital of Spain? Answer only with the city name.' \
  --require-finish-sources eos \
  --token-lengths 16 \
  --require-token-lengths 16 \
  --turns 4 \
  --probe-max-tokens 16 \
  --rss-pid <server-pid> \
  --prompt-cache-key long-chat:changing-followups-cache-diagnostic-16 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --proof-log target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/long-chat.exit
```

The diagnostic intentionally did not set `--expect-finish-reason`. The earlier
strict all-EOS attempt used `--expect-finish-reason stop` and failed on turn 3
with:

```text
expected finish_reason stop, got length
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/long-chat.log` | 235 | `fa4a34022bc0809cd79631ad72d9778372f58c69a50c30c76008a1c0969c0670` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/gate.stdout` | 235 | `fa4a34022bc0809cd79631ad72d9778372f58c69a50c30c76008a1c0969c0670` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/server.log` | 7 | `430e58c167e786847ffad8f5d4f3d24c56019b7de83829fe42ba41c7b567b492` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-smollm17-changing-followups-cache-diagnostic-16-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`.

The strict all-EOS attempt exited `1`. Its proof log hash was
`15bd556f6ec07d82d34551846f5fba2afb5b7e578e6baa55c11e2de5d5d9466a` and
`gate.stderr` hash was
`778034d3b0083f381e8f809b8ccb43e7eb7639f6ff0323b509309f2c9113f3b1`.

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
```

## Turn Results

| Turn | Finish | Finish source | Cached / Prompt | Lookup | Prompt hash | Selected hash | Generated hash | TTFT ms | Token limit |
| ---: | --- | --- | ---: | --- | --- | --- | --- | ---: | --- |
| 1 | `stop` | `eos` | 0 / 48 | `miss` | `fnv64:29bca34202dc5f0a` | n/a | `fnv64:af63c74c8601c8dd` | 7902 | false |
| 2 | `stop` | `eos` | 22 / 46 | `shared_prefix_hit` | `fnv64:69824ec3212819fa` | `fnv64:29bca34202dc5f0a` | `fnv64:d975fd21291d28d9` | 3955 | false |
| 3 | `length` | `length` | 22 / 49 | `shared_prefix_hit` | `fnv64:ddbd1cc3509d39bd` | `fnv64:69824ec3212819fa` | `fnv64:07b9f98c303e945b` | 4550 | true |
| 4 | `stop` | `eos` | 23 / 62 | `shared_prefix_hit` | `fnv64:b8403071557d00a6` | `fnv64:ddbd1cc3509d39bd` | `fnv64:21d43b1c9ec8810e` | 6542 | false |

## Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_any_token_limit_hit=true
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_required_finish_sources=eos
long_chat_summary_required_finish_sources_present=true
long_chat_summary_run_complete=true
```

## Interpretation

This diagnostic strengthens the fixed-point cache theory. Changing follow-ups
changed every generated-response hash after turn 1 and changed every prompt
hash, so the cache stayed in `shared_prefix_hit` and never reached
`exact_hit`. The strict EOS version of this lane is not valid yet because turn
3 hit the token limit, so this result should not be counted as all-EOS closure.

The accepted claim is narrower: under the same model, cache key, prompt-cache
trace mode, generated follow-up context, and server process, the fixed-answer
lane reached exact hits only after prompt identity stabilized, while this
changing-answer diagnostic kept changing prompt identity and did not reach
exact hits.
