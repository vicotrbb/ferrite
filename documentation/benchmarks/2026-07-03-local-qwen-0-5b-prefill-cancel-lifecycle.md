# Benchmark: Local Qwen 0.5B Prefill Cancel Lifecycle

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the long-prompt prefill cancellation probe after adding
`openai_stream_lifecycle` server logs. The previous real-model cancellation
smoke proved that reconnect did not wait for a full abandoned prompt, but it did
not show the server-side finish reason, disconnect point, prompt counters, or
generated chunk counters.

This run targets Ferrite's OpenAI-compatible streaming chat endpoint directly
on localhost, without Kubernetes port-forward.

## Environment

- Ferrite commit: `9b0b87e`
- Host: local macOS workspace
- Server: `127.0.0.1:18222`
- Server PID for RSS sampling: `11958`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-prefill-cancel-lifecycle-2026-07-03/`
- Server binary SHA256:
  `50c221c62302c644f0278c5c52ead73e68cc5247e7fe154ff3bf4702d3d6cb59`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18222`.

Focused lifecycle validation before the real-model probe:

```text
cargo test -p ferrite-server stream_lifecycle -- --nocapture
test openai::stream_lifecycle::tests::lifecycle_summary_records_prompt_generation_and_disconnect_state ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out
```

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18222 \
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
`510 ms`, then closed the socket before any generated content arrived.

Immediately after closing the first socket, the script sent a short streaming
reconnect request:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-2026-07-03/prefill-cancel-lifecycle.json` | 32 lines | `a6b19625d326af7a326180ad591a831d6d4eb06ef3b16580aa6cdb1ab790b1a2` |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-2026-07-03/server.log` | 2 lines | `43184144d3b2dd442d1cf9afd65ab58be84ec09a6c8c11ceb1ffbe14f1b4e065` |
| `target/proof/local-qwen05-prefill-cancel-lifecycle-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Client Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `1.481 ms` |
| Delay after role marker before socket close | `510.044 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `3.903 ms` |
| Reconnect first generated event | `6387.344 ms` |
| Reconnect done | `6421.889 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |
| Reconnect done event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `161600 KiB` |
| Immediately after abort close | `167696 KiB` |
| After reconnect | `427184 KiB` |

## Server Lifecycle Results

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6419
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 generated_chunks=1 generated_token_ids=1 elapsed_ms=516
```

## Interpretation

The lifecycle log proves the abandoned long-prompt request did not generate any
content after the client closed the socket. It finished as
`finish_reason=cancelled` at `disconnect_point=prompt_evaluation`, with
`generated_chunks=0` and `generated_token_ids=0`.

The run also shows that cancellation was not instantaneous from the client's
point of view. The client started reconnect about `3.9 ms` after closing the
first socket, but the reconnect did not receive generated content until
`6387.344 ms`. The server lifecycle line for the abandoned request reports
`elapsed_ms=6419`, while the short reconnect request itself reports only
`elapsed_ms=516`. That means most of the client-observed reconnect latency was
spent waiting for the cancelled prompt-evaluation path to release the single
inference permit.

This weakens the severe version of the cancellation theory: the server observed
the disconnect, cancelled in prompt evaluation, and did not stream generated
chunks for the abandoned request. It strengthens the narrower latency theory:
for long prompts, a disconnected client may still occupy the single inference
permit for several seconds before cooperative cancellation reaches a checked
boundary.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The lifecycle counters do not yet identify layer index, tokenization time, or
  work completed after the client closed the socket.
- RSS grew after reconnect because the model was still warming into its loaded
  working set; this is not a memory-retention proof.
- This run does not cover Kubernetes port-forward buffering, high concurrency,
  or cancellation inside a single matvec/kernel.

## Next Step

Add finer prompt-evaluation lifecycle counters before considering lower-level
cancellation policy changes. The next useful counters are:

- prompt token index at cancellation;
- cancellation polls before and after the stream is observed closed;
- current transformer layer when cancellation is observed;
- elapsed time from first observed stream closure to cancellation return.
