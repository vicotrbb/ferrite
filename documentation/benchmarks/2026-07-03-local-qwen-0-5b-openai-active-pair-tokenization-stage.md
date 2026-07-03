# Benchmark: Local Qwen 0.5B OpenAI Active-Pair Tokenization Stage

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Verify that the active-pair BPE tokenizer improvement appears in the
OpenAI-compatible server lifecycle path, not only in the tokenizer-only CLI
benchmark.

The probe intentionally waited after the initial assistant-role SSE event
before closing the client socket. That gives tokenization time to complete, so
the lifecycle line can report `prompt_tokenized_elapsed_ms`.

## Environment

- Ferrite commit: `6781e7f`
- Host: local macOS workspace
- Server: `127.0.0.1:18230`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-openai-active-pair-tokenization-stage-2026-07-03/`
- Server binary SHA256:
  `ba5b35063ca1412f7a6af505a0d3a8695fb4e471c74ce12653aa0070366865d4`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18230`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18230 \
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
- delay after initial assistant-role SSE event before socket close: `6500 ms`

The follow-up reconnect request used:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-openai-active-pair-tokenization-stage-2026-07-03/openai-active-pair-tokenization-stage.json` | 17 | `6647e5f2ab3916b54fbd2ee4c5c4277a94c2017534d2629f8625e7145fb2ae83` |
| `target/proof/local-qwen05-openai-active-pair-tokenization-stage-2026-07-03/server.log` | 2 | `ed6bae942179178a9ecd1209499c5e1629e0029c49d983b4e22af44f9f97a0b5` |
| `target/proof/local-qwen05-openai-active-pair-tokenization-stage-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-openai-active-pair-tokenization-stage-2026-07-03/health.json` | 1 | `2b68d51958114f7e29bc03cfa4d5ad1e18f511877011a629786ebee4448f06cb` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.447 ms` |
| Delay after role marker before socket close | `6500 ms` |
| Generated content before close | false |
| Reconnect elapsed | `353.532 ms` |
| Total probe wall estimate | `6854.197 ms` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=53 prompt_cancellation_polls=1321 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6506 disconnect_observed_elapsed_ms=6505 disconnect_to_finish_ms=0 prompt_cancellation_token_index=52 prompt_cancellation_layer_index=19 engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=4331 prefix_cache_key_built_elapsed_ms=4331 session_started_elapsed_ms=4331 prefix_cache_lookup_finished_elapsed_ms=4331 prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=4331 first_prompt_token_started_elapsed_ms=4331 first_prompt_cancellation_poll_elapsed_ms=4331
```

The reconnect request completed:

```text
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=352 ...
```

## Comparison

Earlier local server lifecycle baseline after BPE metadata preparse:

```text
prompt_tokenized_elapsed_ms=8323
```

Active-pair BPE server lifecycle result:

```text
prompt_tokenized_elapsed_ms=4331
```

The local server-stage tokenization field improved by about `47.96%`:

```text
(8323 - 4331) / 8323 = 0.4796
```

## Interpretation

The active-pair BPE encoder improvement carries into the OpenAI-compatible
server path for this local same-size prompt proof. The prompt-tokenized stage
fell from about `8.32 s` to about `4.33 s`, and reconnect completed in about
`354 ms` after the delayed close.

This is now stronger than tokenizer-only evidence. It still does not prove
full long-chat throughput, because the request was intentionally cancelled
during prompt evaluation after tokenization completed.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The prompt is a deterministic generated same-size sample, not the unsaved
  prompt bytes from the earliest lifecycle probes.
- This is a cancelled streaming request, not a complete long-prompt response.
- Prompt evaluation still dominated after tokenization: cancellation happened
  at prompt token `52`, layer `19`.

## Next Step

Run the dedicated long-chat gate with 256, 512, and 1024-token streaming
responses to measure whether the tokenizer-stage improvement changes
end-to-end OpenAI-compatible behavior under repeated multi-turn conversations.
