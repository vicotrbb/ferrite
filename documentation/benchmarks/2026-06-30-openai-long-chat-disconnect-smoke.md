# OpenAI Long-Chat Disconnect Smoke

## Scope

This is a bounded local smoke test for client disconnect behavior in the
long-chat gate. It proves the gate can open a streaming chat request, observe at
least one generated SSE event, drop the socket before `[DONE]`, and then
complete a fresh streaming chat request against the same server.

This does not prove resumable streams. The supported behavior demonstrated here
is clean retry after a client-side stream abort.

## Environment

- Date: 2026-06-30
- Commits:
  - `6411393` adds the disconnect probe.
  - `11ae480` retries transient reconnect `429` responses.
- Host: local macOS development machine
- Server port: `127.0.0.1:18094`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18094 \
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

## Initial Finding

The first live run aborted the stream after generated content and immediately
reconnected. The reconnect returned:

```text
expected reconnect probe status 200, got 429
```

That showed the server can still have its single inference slot occupied after a
client disconnect. The probe was updated to retry bounded `429` responses before
declaring reconnect failure.

## Successful Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --disconnect-probe \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --addr 127.0.0.1:18094 \
  --api-key local-secret \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world'
```

## Result

The command completed with:

```text
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
```

The command also printed the planned default long-chat matrix for the single
configured model:

- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

After stopping the server, `lsof -nP -iTCP:18094 -sTCP:LISTEN` returned no
listener.

## Interpretation

The long-chat gate now includes an executable client disconnect probe. It
distinguishes clean retry from stream resumption: the client can abandon a
stream after generated output, tolerate transient queue pressure, and complete a
new streaming request.

Remaining proof gaps:

- Decide whether Ferrite should expose resumable stream semantics or explicitly
  document clean retry only.
- Run the full 256, 512, and 1024-token matrix.
- Combine disconnect behavior with RSS and latency sampling in a longer proof
  pass.
- Repeat across the agreed model set.
