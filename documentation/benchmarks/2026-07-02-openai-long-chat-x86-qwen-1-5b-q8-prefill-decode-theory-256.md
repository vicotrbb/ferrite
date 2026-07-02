# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Prefill/Decode Theory Probe 256

## Scope

This run reruns the x86_64 `Qwen2.5-1.5B-Instruct-Q8_0` generated-context
long-chat gate at 256 completion tokens after adding stream-observed
prefill/decode timing fields. It is a measurement-only theory probe for the
long-chat prefix-reuse hypothesis.

This proves that the new timing fields are emitted during a real
OpenAI-compatible HTTP long-chat run. It does not prove an internal engine
prefill/decode split, and it does not prove that a prefix cache would be safe or
effective.

## Environment

- Date: 2026-07-02 local time
- Commit: `a1cb315b5ef2b56219d248e8c007187d5f1e8c8a`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-theory-qwen15-q8-256`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.223`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Memory limit: `8589934592` bytes
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`, LLVM
  `22.1.2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1708`
- Server port inside pod: `127.0.0.1:18154`
- Server RSS after model load: `1876160` KiB
- Pod cgroup memory current after model load: `4093931520` bytes
- Pod cgroup memory peak after build and proof: `5991579648` bytes
- Pod cgroup memory current after server stop: `2172637184` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256-server.log`
  (`0` bytes)

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Binary SHA256 values:

```text
9eb8e2d7069e70875395063eec26fd4ad9b15911aded3e2e4462215b2aa7292e  target/release/ferrite-server
9e18bd5f003e9f4c503d3267f6938c218ab29672f3c0958861205c071a0e6fed  target/release/ferrite-openai-long-chat-gate
```

The Kubernetes API briefly returned `connection refused` immediately after the
release build, while `homelab-01` temporarily reported `NotReady`. The API and
node recovered before the server and gate run. The pod stayed Running/Ready, and
the gate proof completed with exit code `0`.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-256 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 40.58s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18154 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

The first background wrapper wrote the PID file from the wrong shell working
directory, but the server started successfully. PID `1708` was recorded from
`ps` inside the pod before the gate run.

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-256 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 256 \
    --turns 4 \
    --addr 127.0.0.1:18154 \
    --api-key local-secret \
    --rss-pid 1708 \
    --probe-max-tokens 256 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256.exit'
```

The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-256.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 256-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, valid usage accounting, token-limit status,
generated-context status, stream timing, prefill/decode timing, per-token
latency summaries, and RSS samples.

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream ms | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 43 | 256 | 9972 | 63488 | 4.032249 | 73460 | 3.498468 | 1940193280 | 1940193280 | 1940193280 |
| 2 | generated | 287 | 256 | 70787 | 71688 | 3.571007 | 142475 | 1.803816 | 1940193280 | 1955397632 | 1955397632 |
| 3 | generated | 287 | 256 | 70570 | 71532 | 3.578792 | 142102 | 1.808550 | 1955397632 | 1955397632 | 1955397632 |
| 4 | generated | 282 | 256 | 69271 | 71170 | 3.596996 | 140441 | 1.829941 | 1955397632 | 1955397632 | 1955397632 |

Each turn reported:

```text
long_chat_result_hit_token_limit=true
```

The generated-context status progressed as intended:

```text
long_chat_result_assistant_context_source=seed
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
```

The new timing fields were present in every scenario:

```text
long_chat_result_stream_observed_prefill_elapsed_ms=...
long_chat_result_first_token_timestamp_ms=...
long_chat_result_stream_observed_decode_elapsed_ms=...
long_chat_result_stream_observed_decode_tokens_per_second=...
```

## Summary Markers

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Theory Read

This run supports the prefix-reuse theory enough to justify the next design
slice, but it does not prove the final optimization.

The seed turn used 43 prompt tokens and reached the first streamed token in
9972 ms. Generated-context turns used 282-287 prompt tokens and reached the
first streamed token in 69271-70787 ms. Average generated-context
stream-observed prefill was about 70209 ms, roughly 7.0x the seed prefill.

Post-first-token decode also changed, but less dramatically. Seed decode was
63488 ms at 4.032249 decode token events/sec. Generated-context decode averaged
about 71463 ms at about 3.58 decode token events/sec.

Interpretation:

- The large first-token delay increase remains the dominant regression signal
  for generated-context turns.
- Decode pace also degrades by roughly 11 percent, so prefix reuse alone may not
  recover all lost throughput.
- The next implementation slice should first design token-prefix identity and
  cache invalidation rules, then prototype bounded KV prefix reuse behind an
  explicit opt-in or internal experiment flag.
