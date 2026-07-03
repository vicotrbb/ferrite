# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Continuity Window 256

## Scope

This run tests the OpenAI-compatible HTTP long-chat proof path with explicit
generated-response continuity assertions. It uses the current 32-token and
64-token generated-context window candidates at a 256-token completion budget.

This is a correctness probe for the serving path, not a default-policy decision.
It proves that a compact numeric continuity anchor survived the selected
windows in this prompt. It also proves that a longer arbitrary marker was too
brittle for the same prompt shape.

## Environment

- Date: 2026-07-03 UTC, 2026-07-02 local time
- Commit: `f061841`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-continuity-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.226`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod:
  `/work/ferrite/target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `d68365ba3235140741675ebda3c9255667cfe9bb87b37c3f477e31836f7ac751`
- Long-chat gate binary SHA256:
  `916386f6cb579b314c0a7c8a78a96248575332640a3116aad08ec31fe8a13f3f`
- Full-marker raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-continuity-window-256-clean`
- Numeric-anchor raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-continuity-window-numeric-256`

The release binaries were built inside the amd64 pod with:

```sh
cargo build -p ferrite-server --release --bins
```

The build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 1m 12s
```

## Server Command

Each window used a fresh server process:

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:${port} \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --model /work/ferrite/target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 384 \
  --inference-wait-ms 30000
```

Readiness used authenticated `GET /v1/models`.

## Gate Command

The generated-response assertion was added by commit `f061841` and is exposed as
`--require-generated-response-contains TEXT`. It is evaluated only for follow-up
turns that use generated assistant context.

Each numeric-anchor window used:

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:${port} \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the assistant message as the source of the continuity marker for this CPU inference test." \
  --assistant-context "Continuity marker: 7291. Always end with that exact marker." \
  --follow-up "Write one short CPU inference risk and end with the exact continuity marker from the assistant message. Do not invent a new marker." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens ${window} \
  --require-generated-response-contains 7291 \
  --rss-pid ${server_pid} \
  --token-lengths 256 \
  --turns 4
```

## Negative Full-Marker Probe

Before the numeric-anchor run, the same proof shape was run with the longer
marker `FERRITE-CONTINUITY-7291`.

Both clean windows failed at turn 2:

```text
turn 2 generated response missing required substring FERRITE-CONTINUITY-7291
```

The 32-token full-marker run started at `2026-07-03T00:10:41Z` and completed
with status `1` at `2026-07-03T00:16:41Z`. The 64-token full-marker run started
at `2026-07-03T00:16:41Z` and completed with status `1` at
`2026-07-03T00:22:48Z`.

| Window | Turn 1 prompt tokens | Turn 1 TTFT ms | Turn 1 decode ms | Turn 1 stream tok/s | Turn 1 RSS after | Failure |
| ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 32 | 75 | 17563 | 64722 | 3.123264 | 1942106112 | turn 2 missing full marker |
| 64 | 75 | 17572 | 65121 | 3.107866 | 1941905408 | turn 2 missing full marker |

This is valid negative evidence against using long arbitrary marker text as the
only continuity assertion. It is not evidence that generated-context windowing
cannot preserve useful state.

## Numeric Anchor Results

The numeric-anchor run completed both windows:

```text
run_start window=32 port=18332 at=2026-07-03T00:24:15Z
run_complete window=32 status=0 at=2026-07-03T00:33:03Z
run_start window=64 port=18364 at=2026-07-03T00:33:03Z
run_complete window=64 status=0 at=2026-07-03T00:42:19Z
```

Both runs completed the reconnect/error probes:

```text
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
```

## 32-Token Window Numeric Results

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 68 | 256 | 16044 | 64384 | 3.976082 | 3.195339 | 1941303296 | 1941565440 | 1941565440 |
| 2 | generated | 84 | 256 | 19582 | 65124 | 3.930958 | 3.033991 | 1941565440 | 1944842240 | 1944842240 |
| 3 | generated | 85 | 256 | 19860 | 65118 | 3.931297 | 3.024291 | 1944842240 | 1944842240 | 1944842240 |
| 4 | generated | 84 | 256 | 19647 | 65567 | 3.904363 | 3.015879 | 1944842240 | 1944842240 | 1944842240 |

Generated-turn averages:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 84.33 |
| TTFT/prefill ms | 19696.33 |
| Decode ms | 65269.67 |
| Decode tok/s | 3.922206 |
| Stream tok/s | 3.024720 |

The pod cgroup memory samples were:

| Sample | Bytes |
| --- | ---: |
| Before | 2084749312 |
| After | 3905458176 |
| Peak | 7054684160 |

## 64-Token Window Numeric Results

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 68 | 256 | 16269 | 64833 | 3.948594 | 3.168829 | 1941381120 | 1941381120 | 1941381120 |
| 2 | generated | 116 | 256 | 27613 | 66605 | 3.843528 | 2.727679 | 1941381120 | 1944133632 | 1944133632 |
| 3 | generated | 116 | 256 | 27526 | 66370 | 3.857157 | 2.737049 | 1944133632 | 1944133632 | 1944133632 |
| 4 | generated | 116 | 256 | 27582 | 66862 | 3.828726 | 2.721151 | 1944133632 | 1944133632 | 1944133632 |

Generated-turn averages:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 116.00 |
| TTFT/prefill ms | 27573.67 |
| Decode ms | 66612.33 |
| Decode tok/s | 3.843137 |
| Stream tok/s | 2.728626 |

The pod cgroup memory samples were:

| Sample | Bytes |
| --- | ---: |
| Before | 1959419904 |
| After | 3816243200 |
| Peak | 7054684160 |

## Summary Markers

Both numeric-anchor window runs reported:

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

## Infrastructure Notes

An earlier full-marker attempt used a bad shell launch shape and started
duplicate drivers against overlapping ports. That contaminated directory is not
used as proof. The clean full-marker and numeric-anchor directories listed above
used separate ports and fresh server processes.

After copying the artifacts locally, the temporary Kubernetes pod was deleted
and `kubectl get pod ferrite-avx2-continuity-qwen15-q8 --ignore-not-found`
returned no pod.

## Theory Read

The useful result is the split, not just the pass: short structured anchors can
survive very small generated-context windows, while longer arbitrary labels can
fail immediately under the same generated-context mechanism.

That points to a better design direction than simply increasing retained
assistant tokens. Ferrite should test compact model-facing state capsules,
structured anchors, and explicit continuity summaries. The server should not
silently make this a public OpenAI-compatible HTTP default until the 1024-token
long-chat gate and harder multi-turn semantic probes pass.
