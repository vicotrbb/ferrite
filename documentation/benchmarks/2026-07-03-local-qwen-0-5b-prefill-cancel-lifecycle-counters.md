# Benchmark: Local Qwen 0.5B Prefill Cancel Lifecycle Counters

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding
`prompt_cancellation_closed_polls` and `disconnect_to_finish_ms` to
`openai_stream_lifecycle`.

The previous lifecycle-backed run showed that the abandoned request was
cancelled at `prompt_evaluation`, but the log could not distinguish time spent
before first observing the closed stream from time spent unwinding after
observing it.

## Environment

- Ferrite commit: `9cfd700`
- Host: local macOS workspace
- Server: `127.0.0.1:18223`
- Server PID for RSS sampling: `15699`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-lifecycle-counters-2026-07-03/`
- Server binary SHA256:
  `b8e7ec10b7dfebcb519b51196fe86365da57360815472378b7c2e1a82c7dea44`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18223`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18223 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 8 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Probe

The abort request used:

- endpoint: `POST /v1/chat/completions`
- `stream: true`
- `max_tokens: 1`
- prompt shape: system message plus one user message
- user prompt length: `155399` characters

The client read until the initial assistant-role SSE event, waited about
`509 ms`, then closed the socket before any generated content arrived.

Immediately after closing the first socket, the script sent a short streaming
reconnect request:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-counters-2026-07-03/prefill-cancel-lifecycle.json` | 32 lines | `688767bc63785c33c2e0d62c82a4bdf338f063b735b8ca85eea68b16b1b581b5` |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-counters-2026-07-03/server.log` | 2 lines | `55a9e6d271d3b889fb4353954fde7dcef23f735a64840da0484596abe9f80f88` |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-counters-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.835 ms` |
| Delay after role marker before socket close | `508.883 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `3.345 ms` |
| Reconnect first generated event | `6369.323 ms` |
| Reconnect done | `6406.210 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `165808 KiB` |
| Immediately after abort close | `191216 KiB` |
| After reconnect | `436368 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6455 disconnect_to_finish_ms=0
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=463 disconnect_to_finish_ms=none
```

## Interpretation

The new lifecycle counters are present in a real-model OpenAI-compatible server
run. The abandoned request again produced no generated chunks and no generated
token IDs. It was cancelled at `prompt_evaluation`, and
`prompt_cancellation_closed_polls=1` shows that the prompt-cancellation path
observed the closed stream.

The key new signal is `disconnect_to_finish_ms=0`: once Ferrite observed the
closed stream, cancellation returned immediately enough to round to zero
milliseconds. The remaining six-second reconnect delay happened before the
server observed the closed stream, not after cancellation had already been
observed.

This changes the next optimization target. More cancellation work should first
measure when the server observes the closed stream relative to request start
and prompt-evaluation progress. It should not yet move directly to matvec-level
preemption, because the current evidence says post-observation unwinding is not
the slow part.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- `disconnect_to_finish_ms=0` is rounded to milliseconds and does not prove
  nanosecond-level immediate cancellation.
- The log still does not report when the stream was first observed closed
  relative to request start.
- The log still does not report prompt token index or transformer layer index
  at cancellation.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Add `disconnect_observed_elapsed_ms` to the lifecycle line. That field should
report the elapsed request time when the stream is first observed closed. It
will separate transport or stream-closure detection latency from cancellation
return latency.
