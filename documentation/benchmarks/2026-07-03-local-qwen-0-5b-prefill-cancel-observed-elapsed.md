# Benchmark: Local Qwen 0.5B Prefill Cancel Observed Elapsed

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding
`disconnect_observed_elapsed_ms` to `openai_stream_lifecycle`.

The previous counter-enabled run proved that `disconnect_to_finish_ms=0` once
Ferrite observed the closed stream. This run records when that observation
happens relative to request start.

## Environment

- Ferrite commit: `2fabbfd`
- Host: local macOS workspace
- Server: `127.0.0.1:18224`
- Server PID for RSS sampling: `17806`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-observed-elapsed-2026-07-03/`
- Server binary SHA256:
  `6a1e98cf21b830cee3f5d912f5c7560fbacd0108dd324ba5585450216ba402c5`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18224`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18224 \
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
| `target/proof/local-qwen05-prefill-cancel-observed-elapsed-2026-07-03/prefill-cancel-lifecycle.json` | 32 lines | `20ba2ec465e6fc9ed3fa220f0684a00d723a236e6c7eaabc5dfb7b6cdf74c49a` |
| `target/proof/local-qwen05-prefill-cancel-observed-elapsed-2026-07-03/server.log` | 2 lines | `80f8086617f79e14e4a4ad922f53a33e1a8a173a7d5542a9c24f3c4f3c474780` |
| `target/proof/local-qwen05-prefill-cancel-observed-elapsed-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.756 ms` |
| Delay after role marker before socket close | `508.818 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `3.354 ms` |
| Reconnect first generated event | `6420.264 ms` |
| Reconnect done | `6456.580 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `159488 KiB` |
| Immediately after abort close | `182704 KiB` |
| After reconnect | `441856 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6495 disconnect_observed_elapsed_ms=6495 disconnect_to_finish_ms=0
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=473 disconnect_observed_elapsed_ms=none disconnect_to_finish_ms=none
```

## Interpretation

The observed-elapsed lifecycle field is now proven in a real-model
OpenAI-compatible server run. The abandoned request again produced no generated
chunks and no generated token IDs.

The key timing result is:

```text
elapsed_ms=6495
disconnect_observed_elapsed_ms=6495
disconnect_to_finish_ms=0
```

This means Ferrite first observed the closed stream at the end of the cancelled
request lifecycle, then returned from cancellation immediately enough to round
to zero milliseconds. The reconnect delay is therefore not post-observation
cleanup. It is the time until the prompt-evaluation path next observes the
closed stream.

The next optimization target is more precise prompt-evaluation placement:
record prompt token index and transformer layer index at cancellation. If the
closure is only observed after a long layer or token step, the evidence can
justify a lower-level cancellation poll. If it is delayed before entering the
runtime callback, the next fix belongs in transport or stream-state propagation.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- Millisecond counters do not show sub-millisecond cleanup latency.
- The log still does not report prompt token index or transformer layer index
  at cancellation.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Add prompt-token and layer-index lifecycle fields for prompt-evaluation
cancellation. Those fields should identify whether the observed 6.5 second
closure-observation delay is dominated by tokenization, one prompt-token
evaluation, a specific transformer layer, or work before the runtime callback.
