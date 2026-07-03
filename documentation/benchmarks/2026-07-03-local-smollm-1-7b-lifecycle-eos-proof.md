# Benchmark: Local SmolLM2 1.7B Lifecycle EOS Proof

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Refresh natural tokenizer-EOS proof on the current lifecycle-instrumented
Ferrite OpenAI-compatible server.

This repeats the known EOS-sensitive SmolLM2 prompt,
`The capital of France is`, and verifies that tokenizer EOS:

- maps to OpenAI `finish_reason=stop`;
- emits exactly one `[DONE]` for streaming responses;
- preserves usage accounting;
- suppresses the EOS control marker from assistant-visible text;
- emits server lifecycle summaries for streaming requests.

## Environment

- Ferrite commit: `15df02c`
- Host: local macOS workspace
- Server: `127.0.0.1:18208`
- Server PID during proof: `33981`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Proof directory:
  `target/proof/local-smollm17-lifecycle-eos-2026-07-03/`
- Server binary SHA256:
  `9e6458f6ca175e830b253ef77e3d8205195f5597c3d6543ddc7c3e82f9061198`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

The local server was stopped after the run. A final bind-specific process check
returned no listener on `127.0.0.1:18208`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18208 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Probe

The accepted proof artifact is:

`target/proof/local-smollm17-lifecycle-eos-2026-07-03/eos-probe.log`

The probe sent:

- streaming legacy completion to `POST /v1/completions`;
- non-streaming legacy completion to `POST /v1/completions`;
- streaming chat completion to `POST /v1/chat/completions`.

All requests used:

```text
model=SmolLM2-1.7B-Instruct-Q4_K_M
prompt="The capital of France is"
```

The streaming completion used `max_tokens=6` and
`stream_options.include_usage=true`. The non-streaming completion used
`max_tokens=6`. The streaming chat completion used `max_tokens=16` and
`stream_options.include_usage=true`.

Two earlier local prechecks failed because of probe-script assertion and shell
quoting mistakes. They are not used as proof evidence. The accepted probe
exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-smollm17-lifecycle-eos-2026-07-03/eos-probe.log` | 35 lines | `b7054de4151561152584b7dc7df2af7a832bbadcc8d7d703fbe34278bc4dfa9c` |
| `target/proof/local-smollm17-lifecycle-eos-2026-07-03/eos-probe.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-smollm17-lifecycle-eos-2026-07-03/server.log` | 5 lines | `9e5951679471f73de3f1e78f6e3c1edec360d9a71b5b8a299e1edf2ee44c5d4c` |
| `target/proof/local-smollm17-lifecycle-eos-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Results

Streaming legacy completion:

```text
http_status path=/v1/completions status=200 content_type=text/event-stream
completion_stream_eos_finish_reason=stop
completion_stream_done_events=1
completion_stream_visible_text=' Paris.'
completion_stream_eos_usage_prompt_tokens=5
completion_stream_eos_usage_completion_tokens=3
completion_stream_eos_usage_total_tokens=8
completion_stream_finish_is_stop=true
completion_stream_done_once=true
completion_stream_visible_eos_marker_absent=true
```

Non-streaming legacy completion:

```text
http_status path=/v1/completions status=200 content_type=application/json
completion_eos_finish_reason=stop
completion_eos_visible_text=' Paris.'
completion_eos_usage_prompt_tokens=5
completion_eos_usage_completion_tokens=3
completion_eos_usage_total_tokens=8
completion_finish_is_stop=true
completion_visible_eos_marker_absent=true
```

Streaming chat completion:

```text
http_status path=/v1/chat/completions status=200 content_type=text/event-stream
chat_stream_eos_finish_reason=stop
chat_stream_done_events=1
chat_stream_visible_text='\nThe capital of France is Paris.'
chat_stream_eos_usage_prompt_tokens=12
chat_stream_eos_usage_completion_tokens=9
chat_stream_eos_usage_total_tokens=21
chat_stream_finish_is_stop=true
chat_stream_done_once=true
chat_stream_visible_eos_marker_absent=true
```

RSS sampled by the probe process:

```text
eos_probe_before_rss_bytes=2015232
eos_probe_after_rss_bytes=1087782912
eos_probe_rss_delta_bytes=1085767680
```

The low initial RSS sample reflects a cold local server before this accepted
probe faulted model pages into resident memory.

## Server Lifecycle

The accepted probe added two streaming lifecycle lines:

```text
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=5 prompt_cancellation_polls=125 generated_chunks=2 generated_token_ids=2 elapsed_ms=1705
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=12 prompt_cancellation_polls=300 generated_chunks=8 generated_token_ids=8 elapsed_ms=3354
```

The server log also contains three earlier completed streaming lines from
discarded probe-script prechecks. None of the five lifecycle lines reported a
disconnect.

## Interpretation

Ferrite's current local OpenAI-compatible server preserves the expected natural
EOS behavior for SmolLM2 1.7B Q4_K_M:

- tokenizer EOS maps to OpenAI `finish_reason=stop`;
- streaming responses terminate with one `[DONE]`;
- completion usage counts EOS-side termination without exposing the control
  marker as visible assistant text;
- lifecycle instrumentation records the streaming EOS requests as completed.

This proof is current for the lifecycle-instrumented server, unlike the older
June 30 local EOS note.

## Limits

This run does not prove:

- Qwen-specific natural EOS behavior;
- x86_64 current-commit lifecycle EOS behavior;
- repeated multi-turn long-chat EOS behavior;
- high-concurrency EOS behavior;
- long-running RSS stability.
