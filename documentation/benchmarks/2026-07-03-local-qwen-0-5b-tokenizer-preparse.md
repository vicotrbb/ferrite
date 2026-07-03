# Benchmark: Local Qwen 0.5B Tokenizer Preparse Timing

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Measure the long-prompt tokenization timing after pre-parsing BPE token maps and
merge metadata at GGUF tokenizer load time.

The prior runtime-stage proof showed a long prompt reaching
`prompt_tokenized_elapsed_ms=8581` before any prompt evaluation work. This run
checks whether moving BPE merge parsing and token-id map construction out of the
per-request encode path materially changes that lifecycle field.

## Environment

- Ferrite commit: `0dffac1`
- Host: local macOS workspace
- Server: `127.0.0.1:18229`
- Server PID for RSS sampling: `40330`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-tokenizer-preparse-2026-07-03/`
- Server binary SHA256:
  `39a2bc40e1443ca3d1fec9b8d048b3899e245ff0e529d0969351beaf610b976a`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18229`.

## Probe

The request used:

- endpoint: `POST /v1/chat/completions`
- `stream: true`
- `max_tokens: 1`
- prompt shape: system message plus one user message
- user prompt length: `155399` characters

The client did not intentionally disconnect after the initial role event. The
probe had a bounded `180 s` timeout. It timed out after reading only the initial
assistant-role SSE event. The server was then interrupted, causing the request
to record a cancellation lifecycle line.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-tokenizer-preparse-2026-07-03/server.log` | 1 line | `8e0e5450fb7806065fa8166cdf04c6bfccd696ece8e11c29e5186c5e1bb48ba4` |
| `target/proof/local-qwen05-tokenizer-preparse-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1881 prompt_cancellation_polls=47025 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=180227 disconnect_observed_elapsed_ms=180207 disconnect_to_finish_ms=20 prompt_cancellation_token_index=1880 prompt_cancellation_layer_index=23 engine_lock_acquired_elapsed_ms=0 generation_started_elapsed_ms=0 prompt_tokenized_elapsed_ms=8323 prefix_cache_key_built_elapsed_ms=8323 session_started_elapsed_ms=8323 prefix_cache_lookup_finished_elapsed_ms=8323 prefix_cache_restored_elapsed_ms=none prompt_evaluation_started_elapsed_ms=8323 first_prompt_token_started_elapsed_ms=8323 first_prompt_cancellation_poll_elapsed_ms=8323
```

## Interpretation

The preparse slice moved BPE merge validation and token-id map construction to
tokenizer load time. Correctness is covered by focused tests, including
load-time rejection of malformed merge metadata.

This single local proof does not justify a broad tokenizer-throughput claim.
The measured long-prompt tokenization field was:

```text
prompt_tokenized_elapsed_ms=8323
```

The earlier comparable runtime-stage proof recorded:

```text
prompt_tokenized_elapsed_ms=8581
```

That is directionally lower, but it is one local run and still dominated by the
simple BPE merge-scan algorithm. More importantly, a full long-prompt request
without disconnect did not finish within the `180 s` client timeout; it was in
prompt evaluation at prompt token `1880`, layer `23`, with no generated chunks.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The run was interrupted after the client timeout; it is not a successful
  full long-prompt completion proof.
- This is one sample and should not be treated as statistically meaningful.
- Pre-parsing avoids repeated metadata work, but it does not change the
  asymptotic cost of the simple BPE merge-scan tokenizer.

## Next Step

The next tokenizer theory should target the BPE algorithm itself, not only
metadata setup. Candidate experiments:

- cache byte-seeded symbols for repeated prompt prefixes;
- use merge-rank pair queues instead of scanning all merge rules over the whole
  symbol list;
- add a dedicated tokenizer micro-benchmark so request lifecycle noise does not
  obscure tokenizer-only latency.
