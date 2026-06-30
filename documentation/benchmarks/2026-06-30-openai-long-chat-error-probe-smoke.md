# OpenAI Long-Chat Error Probe Smoke

## Scope

This is a bounded local smoke test for the long-chat gate's client
error/reconnect probe. It proves the gate can deliberately send an unauthorized
streaming chat request, observe the expected OpenAI-compatible HTTP error, open
a new authorized streaming connection, and require that replacement connection
to complete.

This does not prove mid-stream reconnect/resume semantics. It is a recovery
probe for request-level client errors followed by a clean reconnect.

## Environment

- Date: 2026-06-30
- Commit: `5ab58e5`
- Host: local macOS development machine
- Server port: `127.0.0.1:18092`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18092 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --error-probe \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --addr 127.0.0.1:18092 \
  --api-key local-secret \
  --prompt 'Write one short line about CPU inference.' \
  --assistant-context 'CPU inference runs local model weights on commodity processors.' \
  --follow-up 'Continue briefly.'
```

## Result

The command completed with:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
```

The full command also printed the planned default long-chat matrix for the
single configured model:

- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

After stopping the server, `lsof -nP -iTCP:18092 -sTCP:LISTEN` returned no
listener.

## Interpretation

The long-chat gate now includes an explicit request-level error recovery probe:
an invalid-key streaming chat request must fail with `401`, and the next
authorized streaming chat request on a new connection must complete.

Remaining proof gaps:

- Exercise mid-stream disconnect behavior.
- Decide whether Ferrite should support resume semantics or only clean retry
  semantics after a failed stream.
- Run this probe as part of the complete 256/512/1024 long-chat matrix.
- Repeat across the agreed model set.
