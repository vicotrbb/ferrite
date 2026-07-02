# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Token Window Probed 512

## Scope

This run repeats the strongest generated-context token-window candidates from
the 256-token sweep, `32` and `64`, at a 512-token completion budget. It adds
the required reconnect/error probes before the four-turn long-chat gate.

This proves the OpenAI-compatible HTTP long-chat proof path for the selected
windows. It does not prove default serving semantics or conversation quality for
real user chats.

## Environment

- Date: 2026-07-02 local time
- Commit: `e16c424`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-window-probe-qwen15-q8-512`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.234`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `7efa78c8b876973d25c2f1c03bf3399f6d1c7aefb1f61773081d6994a4e0e516`
- Long-chat gate binary SHA256:
  `039c61d988b26bdf829946e84ca6fae7b5398798af06ef979c7a538515ce8487`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-window-probed-512`

The release binaries were built inside the amd64 pod with:

```sh
cargo build -p ferrite-server --release --bins
```

The build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 42.07s
```

## Server Command

Each window used a fresh server process:

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:${port} \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768 \
  --inference-wait-ms 30000
```

Readiness used authenticated `GET /v1/models`. A first setup attempt incorrectly
used `/readyz`, which this server build does not expose; no long-chat gate ran
before that setup was corrected.

## Gate Command

Each window used:

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:${port} \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks." \
  --expect-finish-reason length \
  --probe-max-tokens 512 \
  --generated-context-max-tokens ${window} \
  --rss-pid ${server_pid} \
  --token-lengths 512 \
  --turns 4
```

## Probe Results

Both windows completed the reconnect/error probes:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=512
```

The 32-token run started at `2026-07-02T23:14:07Z` and completed at
`2026-07-02T23:29:31Z`. The 64-token run started at
`2026-07-02T23:29:31Z` and completed at `2026-07-02T23:45:24Z`.

## 32-Token Window Results

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 512 | 11172 | 136800 | 3.742663 | 3.466833 | 1955864576 | 1955864576 | 1955864576 |
| 2 | generated | 60 | 512 | 14160 | 137622 | 3.720329 | 3.379834 | 1955864576 | 1956388864 | 1956388864 |
| 3 | generated | 59 | 512 | 13792 | 140919 | 3.633293 | 3.315852 | 1956388864 | 1961500672 | 1961500672 |
| 4 | generated | 60 | 512 | 13866 | 136582 | 3.748642 | 3.409789 | 1961500672 | 1961500672 | 1961500672 |

Generated-turn averages:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 59.67 |
| TTFT/prefill ms | 13939.33 |
| Decode ms | 138374.33 |
| Decode tok/s | 3.700755 |
| Stream tok/s | 3.368492 |

The pod cgroup memory samples were:

| Sample | Bytes |
| --- | ---: |
| Before | 655863808 |
| After | 3220963328 |
| Peak | 5092933632 |

## 64-Token Window Results

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 512 | 11054 | 135733 | 3.772098 | 3.494843 | 1956282368 | 1956282368 | 1956282368 |
| 2 | generated | 93 | 512 | 21774 | 139030 | 3.682647 | 3.190201 | 1956282368 | 1958510592 | 1958510592 |
| 3 | generated | 92 | 512 | 21614 | 138947 | 3.684847 | 3.195030 | 1958510592 | 1959165952 | 1959165952 |
| 4 | generated | 94 | 512 | 21666 | 142977 | 3.580975 | 3.115806 | 1959165952 | 1959165952 | 1959165952 |

Generated-turn averages:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 93.00 |
| TTFT/prefill ms | 21684.67 |
| Decode ms | 140318.00 |
| Decode tok/s | 3.649490 |
| Stream tok/s | 3.167012 |

The pod cgroup memory samples were:

| Sample | Bytes |
| --- | ---: |
| Before | 1258565632 |
| After | 3466182656 |
| Peak | 5364563968 |

## Comparison With 512 Unwindowed Baseline

The prior unwindowed 512-token generated-context baseline averaged about
`543.00` prompt tokens, `146094.33` ms TTFT/prefill, `170028.00` ms decode,
`3.011327` decode tok/s, and `1.623005` stream tok/s across generated turns.

| Window | Prompt tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 32 | 59.67 | 13939.33 | 138374.33 | 3.700755 | 3.368492 |
| 64 | 93.00 | 21684.67 | 140318.00 | 3.649490 | 3.167012 |
| Unwindowed baseline | 543.00 | 146094.33 | 170028.00 | 3.011327 | 1.623005 |

Relative to the unwindowed generated-turn baseline:

| Window | Prompt tokens | TTFT/prefill | Decode ms | Decode tok/s | Stream tok/s |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 32 | -89.01% | -90.46% | -18.62% | +22.89% | +107.55% |
| 64 | -82.87% | -85.16% | -17.47% | +21.19% | +95.13% |

## Summary Markers

Both window runs reported:

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

The long attached `kubectl exec` stream reset once with websocket EOF while the
proof continued inside the pod. Both nodes returned `Ready`, the pod reported
`Running` with zero restarts, and the proof was recovered by polling logs and
process state out-of-band.

## Theory Read

The 512-token run strongly supports generated-context windowing as a prompt and
TTFT reduction mechanism. The smaller 32-token window remained fastest on this
prompt while preserving the required gate invariants, reconnect/error probes,
stream token IDs, `finish_reason=length`, and RSS sampling.

This still does not justify changing the public serving default. The next proof
needs conversation-continuity prompts and the 1024-token budget before turning
windowing into an HTTP policy.
