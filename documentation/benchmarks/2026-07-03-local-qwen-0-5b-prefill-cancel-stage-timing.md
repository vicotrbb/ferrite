# Benchmark: Local Qwen 0.5B Prefill Cancel Stage Timing

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding
pre-prompt-evaluation stage timing fields to `openai_stream_lifecycle`.

The previous location run proved cancellation was first observed at prompt
token `0` before transformer layer execution. This run separates engine lock
wait, generation entry, first prompt token callback, and first prompt
cancellation poll.

## Environment

- Ferrite commit: `4297db6`
- Host: local macOS workspace
- Server: `127.0.0.1:18226`
- Server PID for RSS sampling: `28053`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-stage-timing-2026-07-03/`
- Server binary SHA256:
  `821b6dced4796807ad1c8256dc22c6ec4921db5a4d36ae6e60efc52826a6649e`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18226`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18226 \
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
then closed the socket before generated content was produced by the abandoned
request.

Immediately after closing the first socket, the script sent a short streaming
reconnect request:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-prefill-cancel-stage-timing-2026-07-03/prefill-cancel-stage-timing.json` | 77 lines | `e922b301142d2db510df3468976c7a03a9125a803308af0d57354412b092202c` |
| `target/proof/local-qwen05-prefill-cancel-stage-timing-2026-07-03/server.log` | 2 lines | `882222703ef021e206aa1c43741752b0f9d3f47b711dddc76ee787a6ca7d8367` |
| `target/proof/local-qwen05-prefill-cancel-stage-timing-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.537 ms` |
| Delay after role marker before socket close | `509.000 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `4.195 ms` |
| Reconnect first generated event | `7609.333 ms` |
| Reconnect done | `8117.260 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `139264 KiB` |
| Immediately after abort close | `164224 KiB` |
| After reconnect | `422928 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=8132 disconnect_observed_elapsed_ms=8132 disconnect_to_finish_ms=0 prompt_cancellation_token_index=0 prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 first_prompt_token_started_elapsed_ms=8132 first_prompt_cancellation_poll_elapsed_ms=8132
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=507 disconnect_observed_elapsed_ms=none disconnect_to_finish_ms=none prompt_cancellation_token_index=none prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 first_prompt_token_started_elapsed_ms=64 first_prompt_cancellation_poll_elapsed_ms=64
```

## Interpretation

The abandoned request reported:

```text
engine_lock_acquired_elapsed_ms=0
generation_started_elapsed_ms=0
first_prompt_token_started_elapsed_ms=8132
first_prompt_cancellation_poll_elapsed_ms=8132
prompt_cancellation_token_index=0
prompt_cancellation_layer_index=none
```

This rejects engine-lock wait and a single transformer layer as the dominant
delay source for this local run. The delay happens after generation starts but
before the first prompt-token callback and first prompt-cancellation poll.

The strongest current theory is that long-prompt request time is dominated by
pre-prompt-evaluation work inside `InferenceEngine::generate...`, such as
tokenization, prefix-cache lookup, session setup, prompt vector allocation, or
model page faults triggered before prompt token `0` is evaluated.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The timings do not yet split tokenization, prefix-cache lookup, session setup,
  and first-touch model paging.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Add runtime-level stage callbacks around tokenization, prefix-cache lookup, and
session setup. The next proof should identify which pre-prompt-evaluation stage
owns the roughly eight-second delay.
