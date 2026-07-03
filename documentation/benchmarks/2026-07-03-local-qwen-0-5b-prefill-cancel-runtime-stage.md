# Benchmark: Local Qwen 0.5B Prefill Cancel Runtime Stage

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding runtime
stage timing fields to `openai_stream_lifecycle`.

The previous stage-timing run proved the delay happened after generation start
and before the first prompt-token callback. This run splits that window into
tokenization, prefix-cache key construction, session start, prefix-cache lookup,
and prompt-evaluation entry.

## Environment

- Ferrite commit: `13c302c`
- Host: local macOS workspace
- Server: `127.0.0.1:18227`
- Server PID for RSS sampling: `32255`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-runtime-stage-2026-07-03/`
- Server binary SHA256:
  `661eae84fadf6520de9d050676a3e8d74a539c636605d303e63cfee3e5d71246`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18227`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18227 \
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
| `target/proof/local-qwen05-prefill-cancel-runtime-stage-2026-07-03/prefill-cancel-runtime-stage.json` | 89 lines | `df8f76412bd430dc68368715ac283168598d26808a42b06b3274d4852319c29a` |
| `target/proof/local-qwen05-prefill-cancel-runtime-stage-2026-07-03/server.log` | 2 lines | `50c3d1d402a57e508158b56c8dffb1a63dc345cb2b2e8b843809fb6b9db73b73` |
| `target/proof/local-qwen05-prefill-cancel-runtime-stage-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `1.403 ms` |
| Delay after role marker before socket close | `509.000 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `9.136 ms` |
| Reconnect first generated event | `8054.906 ms` |
| Reconnect done | `8582.772 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `135952 KiB` |
| Immediately after abort close | `28016 KiB` |
| After reconnect | `420096 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=8581 disconnect_observed_elapsed_ms=8581 disconnect_to_finish_ms=0 prompt_cancellation_token_index=0 prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=8581 prefix_cache_key_built_elapsed_ms=8581 session_started_elapsed_ms=8581 prefix_cache_lookup_finished_elapsed_ms=8581 prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=8581 first_prompt_token_started_elapsed_ms=8581 first_prompt_cancellation_poll_elapsed_ms=8581
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=527 disconnect_observed_elapsed_ms=none disconnect_to_finish_ms=none prompt_cancellation_token_index=none prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=64 prefix_cache_key_built_elapsed_ms=64 session_started_elapsed_ms=64 prefix_cache_lookup_finished_elapsed_ms=64 prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=64 first_prompt_token_started_elapsed_ms=64 first_prompt_cancellation_poll_elapsed_ms=64
```

## Interpretation

The abandoned request reported:

```text
generation_started_elapsed_ms=0
prompt_tokenized_elapsed_ms=8581
prefix_cache_key_built_elapsed_ms=8581
session_started_elapsed_ms=8581
prefix_cache_lookup_finished_elapsed_ms=8581
prompt_evaluation_started_elapsed_ms=8581
first_prompt_cancellation_poll_elapsed_ms=8581
```

This identifies tokenization as the dominant pre-prompt-evaluation delay in
this local run. The server does not poll for stream closure while tokenizing the
long prompt, so the abandoned request holds the single inference permit until
tokenization finishes and the first prompt cancellation poll observes the
closed stream.

The short reconnect again completed normally after the abandoned request
released the permit.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- Millisecond timestamps collapse several fast stages into the same value after
  tokenization finishes.
- This run does not yet split tokenizer internals or prove an optimized
  cancellation policy.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Move a stream-closure check before tokenization, and consider a chunked or
cancellable tokenizer path for very long prompts. The next proof should show
that a disconnected long prompt can release the single inference permit before
the expensive tokenization path completes.
