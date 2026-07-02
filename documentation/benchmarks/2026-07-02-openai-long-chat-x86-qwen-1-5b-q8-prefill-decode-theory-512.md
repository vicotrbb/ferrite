# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Prefill/Decode Theory Probe 512

## Scope

This run extends the x86_64 `Qwen2.5-1.5B-Instruct-Q8_0`
generated-context prefill/decode theory probe from 256 to 512 completion
tokens. It uses the same stream-observed timing fields added for the
prefix-reuse theory work.

This proves that the 512-token generated-context gate emits the new timing
fields and completes the OpenAI-compatible HTTP long-chat proof path. It does
not prove an internal engine prefill/decode split, and it does not prove that a
prefix cache would be safe or sufficient.

## Environment

- Date: 2026-07-02 local time
- Commit: `af87d10`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-theory-qwen15-q8-512`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.239`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- Memory limit: `8589934592` bytes
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`, LLVM
  `22.1.2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1724`
- Server port inside pod: `127.0.0.1:18154`
- Server RSS after model load: `1875324` KiB
- Pod cgroup memory current after model load: `2883518464` bytes
- Pod cgroup memory peak after build and proof: `4836999168` bytes
- Pod cgroup memory current after server stop: `962564096` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512-server.log`
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

The first `kubectl cp` model transfer failed with a broken pipe and left a
partial 1.2 GB model file with SHA256
`c2db517a0b92fb6d027e229b426e40b5716a821b8f3e36a0869472f9b4ea9a19`. Both
staging nodes briefly reported `NotReady`. The partial model was deleted, both
nodes returned to `Ready`, and the model transfer was retried. The final pod-side
SHA256 matched the expected Q8_0 model hash before build and proof.

During the long-chat gate, the `kubectl exec` stream reset twice while the gate
process continued inside the pod. The proof was recovered by polling the log,
process table, and exit file out-of-band. The gate eventually wrote exit `0` and
all summary markers were true.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 38.78s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18154 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-theory-qwen15-q8-512 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18154 \
    --api-key local-secret \
    --rss-pid 1724 \
    --probe-max-tokens 512 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512.exit'
```

The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-prefill-decode-512.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 512-token budget:

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

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, valid usage accounting, token-limit status,
generated-context status, stream timing, prefill/decode timing, per-token
latency summaries, and RSS samples.

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream ms | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 43 | 512 | 10003 | 136227 | 3.758417 | 146230 | 3.508149 | 1955315712 | 1955315712 | 1955315712 |
| 2 | generated | 553 | 512 | 150282 | 171084 | 2.992665 | 321367 | 1.596304 | 1955315712 | 1986510848 | 1986510848 |
| 3 | generated | 543 | 512 | 143512 | 169980 | 3.012102 | 313493 | 1.636400 | 1986510848 | 1986641920 | 1986641920 |
| 4 | generated | 533 | 512 | 144489 | 169020 | 3.029213 | 313510 | 1.636310 | 1986641920 | 1986641920 | 1986641920 |

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

This run strengthens the prefix-reuse theory, and it also shows that decode pace
degrades as generated context grows.

The seed turn used 43 prompt tokens and reached the first streamed token in
10003 ms. Generated-context turns used 533-553 prompt tokens and reached the
first streamed token in 143512-150282 ms. Average generated-context
stream-observed prefill was about 146094 ms, roughly 14.6x the seed prefill.

Post-first-token decode also slowed. Seed decode was 136227 ms at 3.758417
decode token events/sec. Generated-context decode averaged about 170028 ms at
about 3.01 decode token events/sec, roughly 20 percent slower by decode event
rate.

Interpretation:

- The first-token delay grows sharply with generated-context prompt size and is
  now the clearest target for prefix reuse.
- Decode degradation is material enough that prefix reuse should be measured as
  TTFT recovery first, not as a full throughput fix.
- The 1024-token timing rerun remains necessary before designing the bounded KV
  prefix-cache experiment.
