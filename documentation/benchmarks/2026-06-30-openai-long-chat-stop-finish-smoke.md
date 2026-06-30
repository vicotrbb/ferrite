# OpenAI Long-Chat Stop Finish Smoke

## Scope

This is a bounded local smoke test for the long-chat gate's explicit finish
reason assertion. It proves the gate can require `finish_reason=stop` and fail
closed when the observed streaming response returns a different finish reason.

This is not the full long-chat milestone. It uses one local model and a
one-token stop scenario to validate the stop gate behavior before running the
larger 256/512/1024-token matrix.

## Environment

- Date: 2026-06-30
- Commit: `96e36c7`
- Host: local macOS development machine
- Server port: `127.0.0.1:18093`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18093 \
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

## Failed-Closed Check

The first attempt used `--stop '你'` with the long-chat-shaped prompt and
required `--expect-finish-reason stop`.

```text
expected finish_reason stop, got length
```

That failure is useful evidence: the gate did not silently accept a length
terminal event as a stop proof.

## Output Sampling

Sampling the same long-chat-shaped request without a stop sequence showed that
the first generated content token was `1`, followed by `0`, `0`, `0`,
` characters`, newline, `user`, and `:` before `finish_reason=length`.

The successful stop proof therefore used `--stop '1'`, matching actual observed
model output for this prompt.

## Successful Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1 \
  --turns 4 \
  --addr 127.0.0.1:18093 \
  --api-key local-secret \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop '1' \
  --expect-finish-reason stop
```

## Result

All four planned streaming chat scenarios completed with `finish_reason=stop`.

| Turn | Completed | Finish | Total ms | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| 1 | 1 | stop | 748 | 18 | 1 | 19 |
| 2 | 1 | stop | 754 | 18 | 1 | 19 |
| 3 | 1 | stop | 761 | 18 | 1 | 19 |
| 4 | 1 | stop | 780 | 18 | 1 | 19 |

After stopping the server, `lsof -nP -iTCP:18093 -sTCP:LISTEN` returned no
listener.

## Interpretation

The long-chat gate now has an executable stop-finish assertion. A stop proof
must produce `finish_reason=stop`; a length-limited response does not pass when
the scenario requires stop.

Remaining proof gaps:

- Add an EOS-specific proof once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Run the full 256, 512, and 1024-token matrix.
- Combine stop assertions with RSS and latency sampling in a longer proof pass.
- Repeat across the agreed model set.
