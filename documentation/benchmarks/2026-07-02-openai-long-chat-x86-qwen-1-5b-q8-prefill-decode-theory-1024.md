# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Prefill/Decode Theory Probe 1024

## Scope

This run completes the x86_64 `Qwen2.5-1.5B-Instruct-Q8_0`
generated-context prefill/decode theory probe set at 1024 completion tokens. It
uses the stream-observed timing fields added for the prefix-reuse theory work
and runs through the OpenAI-compatible HTTP server.

This proves that the 1024-token generated-context gate emits the new timing
fields and completes the HTTP long-chat proof path. It does not prove an
internal engine prefill/decode split, and it does not prove that prefix caching
is correct, safe, or sufficient.

## Environment

- Date: 2026-07-02 local time
- Commit: `3e4b89644dec47593a28bfc9cf7a428f1d1d87fa`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-theory-qwen15-q8-1024`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.240`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Memory limit: `8589934592` bytes
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`, LLVM
  `22.1.2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1701`
- Server port inside pod: `127.0.0.1:18154`
- Server RSS after model load: `1875788` KiB
- Pod cgroup memory current after model load: `2254299136` bytes
- Pod cgroup memory peak after build and proof: `4233572352` bytes
- Pod cgroup memory current after server stop: `333262848` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024-server.log`
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

## Infrastructure Notes

The first release build exec stream failed during Kubernetes API instability.
No Cargo or Rust compiler process remained, and the release binaries did not
exist afterward, so the build was rerun cleanly.

During the gate run, the Kubernetes API and kubelet proxy intermittently
returned connection refused, `apiserver not ready`, websocket close `1006`, and
502 Bad Gateway responses while the in-pod gate process continued. The proof
was recovered by polling the process table, log, and exit file out-of-band. The
gate eventually wrote exit `0` and all summary markers were true.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The successful release build completed with:

```text
Finished `release` profile [optimized] target(s) in 40.55s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18154 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-1024 -- sh -lc \
  'cd /work/ferrite && nohup ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 1024 \
    --turns 4 \
    --addr 127.0.0.1:18154 \
    --api-key local-secret \
    --rss-pid 1701 \
    --probe-max-tokens 1024 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024.exit'
```

The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-1024.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 1024-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, valid usage accounting, token-limit status,
generated-context status, stream timing, prefill/decode timing, per-token
latency summaries, and RSS samples.

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream ms | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 43 | 1024 | 10160 | 315756 | 3.243004 | 325917 | 3.144969 | 1986543616 | 1986805760 | 1986805760 |
| 2 | generated | 1080 | 1024 | 325320 | 479782 | 2.134300 | 805102 | 1.273129 | 1986805760 | 2048495616 | 2048495616 |
| 3 | generated | 1054 | 1024 | 314029 | 453261 | 2.259181 | 767291 | 1.335868 | 2048495616 | 2049544192 | 2049544192 |
| 4 | generated | 1054 | 1024 | 317446 | 461076 | 2.220890 | 778523 | 1.316595 | 2049544192 | 2049806336 | 2049806336 |

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

This run completes the initial 256/512/1024 timing-theory set and strongly
supports designing a bounded prefix-cache experiment next.

The seed turn used 43 prompt tokens and reached the first streamed token in
10160 ms. Generated-context turns used 1054-1080 prompt tokens and reached the
first streamed token in 314029-325320 ms. Average generated-context
stream-observed prefill was about 318932 ms, roughly 31.4x the seed prefill.

Post-first-token decode slowed as well. Seed decode was 315756 ms at 3.243004
decode token events/sec. Generated-context decode averaged about 464706 ms at
about 2.20 decode token events/sec, roughly 32 percent slower by decode event
rate.

Across 256, 512, and 1024 tokens, generated-context prefill grows from about
7.0x to 14.6x to 31.4x the seed prefill. Decode also degrades from about
11 percent to 20 percent to 32 percent slower than seed decode event rate.

Interpretation:

- Prefix reuse is now the highest-value optimization theory for first-token
  latency.
- Prefix reuse should be judged first on TTFT recovery, not full stream
  throughput recovery.
- A separate decode-performance theory remains necessary because longer
  generated-context runs also slow post-first-token decode.
- The next slice should be a design note for token-prefix identity, invalidation,
  bounded KV retention, RSS gates, and proof gates before any cache code.
