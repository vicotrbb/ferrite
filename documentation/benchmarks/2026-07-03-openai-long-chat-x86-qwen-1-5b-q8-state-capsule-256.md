# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 State Capsule 256

## Scope

This run tests the proof-only `--generated-context-state-capsule` long-chat gate
option against the OpenAI-compatible HTTP server. It uses Qwen2.5-1.5B Q8_0 on
x86_64 AVX2 with 32-token and 64-token generated-context windows at a 256-token
completion budget.

This is a theory probe for structured continuity anchors. It is not a serving
default and does not change Ferrite's public HTTP behavior.

## Environment

- Date: 2026-07-03 UTC, 2026-07-02 local time
- Commit: `f202a84`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-state-capsule-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.209`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `fa81c19000f49b7c707475c27f70c019b6bddea2def82cba5825e04d2c1d62cb`
- Long-chat gate binary SHA256:
  `9119a256817884470b3cfcc94cce6ca324aaf9f3d2ae37d950c537907815b37a`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-state-capsule-256`

The release binaries were built inside the amd64 pod with:

```sh
cargo build -p ferrite-server --release --bins
```

The build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 2m 13s
```

## Gate Command

Each window used a fresh server process and this gate shape:

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr "127.0.0.1:${port}" \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the assistant message and any Ferrite state capsule as the source of the continuity anchor for this CPU inference test." \
  --assistant-context "Initial context: CPU inference continuity proof starts now." \
  --follow-up "Write one short CPU inference risk. Copy the state_anchor value from the Ferrite state capsule if present, and end with exactly that value. Do not invent a new anchor." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens "${window}" \
  --generated-context-state-capsule '{"state_anchor":"7291","rule":"End every answer with exactly this anchor."}' \
  --require-generated-response-contains 7291 \
  --rss-pid "${server_pid}" \
  --token-lengths 256 \
  --turns 4
```

## Run Summary

```text
run_start window=32 port=18432 at=2026-07-03T01:03:09Z
run_complete window=32 status=0 at=2026-07-03T01:12:39Z
run_start window=64 port=18464 at=2026-07-03T01:12:39Z
run_complete window=64 status=1 at=2026-07-03T01:19:05Z
```

The 32-token window passed the full gate. The 64-token window completed the
error probe, disconnect/reconnect probe, and turn 1, then failed at turn 2:

```text
turn 2 generated response missing required substring 7291
```

## 32-Token Window Results

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 77 | 256 | 17884 | 64309 | 3.980758 | 3.126752 | 1942396928 | 1942396928 | 1942396928 |
| 2 | generated | 129 | 256 | 30413 | 66424 | 3.853997 | 2.653902 | 1942396928 | 1945149440 | 1945149440 |
| 3 | generated | 129 | 256 | 30550 | 66308 | 3.860724 | 2.653322 | 1945149440 | 1945149440 | 1945149440 |
| 4 | generated | 128 | 256 | 31002 | 67222 | 3.808248 | 2.616451 | 1945149440 | 1945149440 | 1945149440 |

Generated-turn averages:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 128.67 |
| TTFT/prefill ms | 30655.00 |
| Decode ms | 66651.33 |
| Decode tok/s | 3.840990 |
| Stream tok/s | 2.641225 |

The 32-token run reported:

```text
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

The pod cgroup memory samples for the 32-token run were:

| Sample | Bytes |
| --- | ---: |
| Before | 2985775104 |
| After | 3010584576 |
| Peak | 4975042560 |

## 64-Token Window Result

The 64-token window completed probes and turn 1:

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 77 | 256 | 18140 | 64890 | 3.945083 | 3.095230 | 1944354816 | 1944354816 | 1944354816 |

It then failed before recording turn 2 metrics:

```text
turn 2 generated response missing required substring 7291
```

The pod cgroup memory samples for the 64-token run were:

| Sample | Bytes |
| --- | ---: |
| Before | 3276111872 |
| After | 3308089344 |
| Peak | 5206036480 |

## Comparison With Prior Continuity Probes

The prior numeric-anchor run without a state capsule passed both 32-token and
64-token generated-context windows at the same 256-token completion budget. Its
32-token generated-turn averages were `84.33` prompt tokens and `19696.33` ms
TTFT/prefill.

The state-capsule 32-token run passed, but its generated-turn averages increased
to `128.67` prompt tokens and `30655.00` ms TTFT/prefill. The state capsule adds
prompt cost.

The state-capsule 64-token run failed at turn 2 even though the prior 64-token
numeric-anchor run passed. That suggests the failure is not simply "more
context is better." A larger retained generated window may preserve prose that
competes with, dilutes, or distracts from the compact capsule instruction.

## Theory Read

This result keeps the structured-anchor theory alive, but narrows it:

- A proof-only state capsule can preserve a short anchor through a 32-token
  generated-context window.
- The current capsule placement and wording are not robust across window sizes.
- Larger generated-context windows can be worse for this prompt, likely because
  they retain more uncontrolled generated prose next to the capsule.

The next experiment should not simply increase the window. It should test
capsule placement and authority:

- put the capsule in the follow-up user message rather than assistant context;
- shorten the capsule to `state_anchor=7291`;
- move the generated prose after the instruction with a clear delimiter;
- test a 32-token capsule-only mode that omits generated prose entirely;
- add semantic checks after substring checks pass.

## Cleanup

After copying the raw proof directory locally, the temporary Kubernetes pod was
deleted and `kubectl --context staging get pod ferrite-avx2-state-capsule-qwen15-q8 --ignore-not-found`
returned no pod.
