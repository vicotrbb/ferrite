# Benchmark: Local Qwen 0.5B Prefill Cancel Tokenization

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the local long-prompt prefill cancellation probe after adding cooperative
tokenization cancellation.

The previous runtime-stage proof identified tokenization as the dominant delay
for disconnected long-prompt streams. This run checks whether polling stream
closure during tokenization releases the single inference permit before prompt
tokenization completes.

## Environment

- Ferrite commit: `2a5bc02`
- Host: local macOS workspace
- Server: `127.0.0.1:18228`
- Server PID for RSS sampling: `37362`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-tokenization-2026-07-03/`
- Server binary SHA256:
  `aa62de9abc98bbed9719ec2687f3a4f35a131c31846909ef2b3c9d5ed754f970`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18228`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18228 \
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
| `target/proof/local-qwen05-prefill-cancel-tokenization-2026-07-03/prefill-cancel-tokenization.json` | 89 lines | `ceb34b56e2a7c4ae3403472ea058f2ddf5ec2958669c3da4fa71e69733808fbc` |
| `target/proof/local-qwen05-prefill-cancel-tokenization-2026-07-03/server.log` | 2 lines | `fa1da73f65387b7dfc1dfa374b5ff245d8696691c0c46755f1331d0dab7f1931` |
| `target/proof/local-qwen05-prefill-cancel-tokenization-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `0.765 ms` |
| Delay after role marker before socket close | `509.000 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `5.628 ms` |
| Reconnect first generated event | `0.261 ms` |
| Reconnect done | `590.204 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `1680 KiB` |
| Immediately after abort close | `27056 KiB` |
| After reconnect | `423824 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=tokenization prompt_tokens_started=0 prompt_cancellation_polls=0 prompt_cancellation_closed_polls=0 generated_chunks=0 generated_token_ids=0 elapsed_ms=517 disconnect_observed_elapsed_ms=514 disconnect_to_finish_ms=2 prompt_cancellation_token_index=none prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=none prefix_cache_key_built_elapsed_ms=none session_started_elapsed_ms=none prefix_cache_lookup_finished_elapsed_ms=none prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=none first_prompt_token_started_elapsed_ms=none first_prompt_cancellation_poll_elapsed_ms=none
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 prompt_cancellation_closed_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=589 disconnect_observed_elapsed_ms=none disconnect_to_finish_ms=none prompt_cancellation_token_index=none prompt_cancellation_layer_index=none engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=65 prefix_cache_key_built_elapsed_ms=65 session_started_elapsed_ms=65 prefix_cache_lookup_finished_elapsed_ms=65 prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=65 first_prompt_token_started_elapsed_ms=65 first_prompt_cancellation_poll_elapsed_ms=65
```

## Interpretation

The abandoned request now stops during tokenization:

```text
disconnect_point=tokenization
elapsed_ms=517
disconnect_observed_elapsed_ms=514
disconnect_to_finish_ms=2
prompt_tokenized_elapsed_ms=none
prompt_tokens_started=0
generated_chunks=0
generated_token_ids=0
```

This validates the tokenization-cancellation theory for the local Qwen2.5-0.5B
Q4_K_M run. The abandoned request no longer waits for full long-prompt
tokenization before releasing the single inference permit. The short reconnect
started about `5.628 ms` after abort close, observed generated content
immediately, and completed in `590.204 ms`.

Compared with the previous runtime-stage proof, the abandoned request dropped
from about `8581 ms` elapsed to `517 ms` elapsed for the same prompt shape and
close timing.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The BPE tokenizer still uses a simple merge-scan algorithm; this improves
  cancellation latency, not raw tokenization throughput.
- The first RSS sample is low because the model is memory-mapped and pages are
  faulted lazily as inference touches them.
- This run does not cover Kubernetes port-forward buffering or high
  concurrency.

## Next Step

Run the same cancellation gate on the x86 proof model and add a tokenizer
throughput theory: cache parsed BPE merges and token maps, then compare raw
tokenization latency before attempting a larger BPE algorithm rewrite.
