# Benchmark: Local Qwen 0.5B Prefill Cancel Location

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding prompt
token and transformer layer location fields to `openai_stream_lifecycle`.

This run checks whether the remaining cancellation delay is observed before a
prompt token enters a transformer layer, inside a specific layer, or after the
server already knows the client has disconnected.

## Environment

- Ferrite commit: `0bd222a`
- Host: local macOS workspace
- Server: `127.0.0.1:18225`
- Server PID for RSS sampling: `23999`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-location-2026-07-03/`
- Server binary SHA256:
  `fc1b9b0b7e56882ffec728854188910f6e7b0184089cf86405bcda06429152b1`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18225`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18225 \
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

The client read until the initial assistant-role SSE event, waited `509 ms`,
then closed the socket before any generated content was produced by the
abandoned request.

Immediately after closing the first socket, the script sent a short streaming
reconnect request:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-prefill-cancel-location-2026-07-03/prefill-cancel-lifecycle.json` | 63 lines | `0fdda1f46592ec8b7b3a8da9b2ce4733c6fc3e3d0ef72360b33483f23c08c5a4` |
| `target/proof/local-qwen05-prefill-cancel-location-2026-07-03/server.log` | 2 lines | `94eb22ad2c9dce92f5de1bb5e220d5f6fb465eeb86e4db1224d7a2d854bc35f5` |
| `target/proof/local-qwen05-prefill-cancel-location-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.802 ms` |
| Delay after role marker before socket close | `509.000 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `4.868 ms` |
| Reconnect first generated event | `7688.862 ms` |
| Reconnect done | `8210.075 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `1872 KiB` |
| Immediately after abort close | `31024 KiB` |
| After reconnect | `417168 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=8203 disconnect_observed_elapsed_ms=8203 disconnect_to_finish_ms=0 prompt_cancellation_token_index=0 prompt_cancellation_layer_index=none
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=521 disconnect_observed_elapsed_ms=none disconnect_to_finish_ms=none prompt_cancellation_token_index=none prompt_cancellation_layer_index=none
```

## Interpretation

The location fields are now proven in a real-model OpenAI-compatible server
run. The abandoned request reported:

```text
prompt_cancellation_token_index=0
prompt_cancellation_layer_index=none
disconnect_observed_elapsed_ms=8203
disconnect_to_finish_ms=0
```

That means Ferrite first observed the closed stream at the cancellation poll
before prompt token `0` entered transformer layer execution. The remaining
delay is therefore not explained by one long transformer layer. It is dominated
by work before the runtime prompt-cancellation callback is first reached, most
likely request parsing, chat-template/tokenization work, model paging/warmup,
or other prompt setup before scalar prompt evaluation starts polling.

The short reconnect still completed normally after the abandoned request
released the single inference permit.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The first RSS sample is low because the model is memory-mapped and pages are
  faulted lazily as inference touches them.
- This run does not separate tokenization, chat-template formatting, prompt
  vector allocation, model paging, or permit wait time before the first prompt
  cancellation poll.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Move the cancellation poll earlier in the request path and measure again. The
next useful theory is a pre-tokenization/pre-template stream-state check, then a
post-tokenization but pre-inference check with elapsed counters for each stage.
