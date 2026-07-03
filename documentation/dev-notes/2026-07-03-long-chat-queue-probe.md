# Long Chat Queue Probe

Date: 2026-07-03

## Goal

Add a bounded proof-harness probe for queued OpenAI-compatible streaming
requests across two prompt-cache keys.

## Context

`documentation/theories/2026-07-03-mixed-cache-key-isolation.md` identifies
queued or concurrent clients as the next unproven risk after the sequential
mixed-key proof. The existing gate could prove two cache-key lanes in sequence,
but it could not deliberately start a second client while the first stream was
already generating.

## Changes

- Added `--queue-probe` to `ferrite-openai-long-chat-gate`.
- Required `--queue-probe` to be paired with at least two
  `--prompt-cache-keys`.
- Added a focused `long_chat_gate::queue_probe` module.
- The queue probe starts a holder streaming request with key A, waits until it
  observes generated stream content, then starts a contender streaming request
  with key B.
- The probe requires both streams to complete as OpenAI SSE responses with
  generated events.
- Added queue-probe result output and summary markers:
  `long_chat_summary_queue_probe_required`,
  `long_chat_summary_queue_probe_completed`, and
  `long_chat_summary_queue_probe_contender_started_after_holder`.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate queue_probe -- --nocapture
error[E0432]: unresolved imports `format_queue_probe_result`, `LongChatQueueProbeResult`
error[E0599]: no method named `queue_probe` found for struct `LongChatGateConfig`
error[E0061]: this function takes 4 arguments but 5 arguments were supplied
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate queue_probe -- --nocapture
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out

cargo fmt --check

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 65 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p ferrite-server --lib
test result: ok. 392 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo clippy -p ferrite-server --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.11s

git diff --check
```

## Limits

This is a proof-harness capability. It now has one local real-model proof with
Qwen2.5-0.5B Q4_K_M:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-queue-probe-256.md`
- `long_chat_summary_queue_probe_completed=true`
- `long_chat_summary_queue_probe_contender_started_after_holder=true`
- `long_chat_summary_run_complete=true`

The local proof does not replace a bounded staging run with Qwen2.5-1.5B Q8_0
and x86_64 AVX2. Kubernetes staging was unreachable during this run.

It also does not prove cache eviction, many-client behavior, or varied follow-up
wording.

## Next Proof

Run the same Qwen2.5-1.5B Q8_0 semantic capsule shape on staging with:

```text
--queue-probe
--prompt-cache-keys ferrite:qwen15:q8:queue:a:256:2026-07-03,ferrite:qwen15:q8:queue:b:256:2026-07-03
```

Start with `--probe-max-tokens 64` so the holder and contender streams remain
bounded before expanding to the full 256-token mixed-key gate.
