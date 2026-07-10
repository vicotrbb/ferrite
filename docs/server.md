# HTTP server

`ferrite-server` loads one model and exposes a local OpenAI-compatible API.
Build and run release binaries for realistic behavior:

```sh
cargo build --release --locked -p ferrite-server

target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:8080 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

## Configuration

| Option | Default | Meaning |
| --- | --- | --- |
| `--bind <address>` | `127.0.0.1:8080` | TCP listen address |
| `--model <path>` | none | GGUF model path |
| `--model-id <id>` | `ferrite-local` | Exact ID required in API requests |
| `--api-key <key>` | none | Bearer token required for `/v1/*` |
| `--default-max-tokens <n>` | 16 | Generation limit when omitted by a request |
| `--hard-max-tokens <n>` | 256 | Maximum accepted request limit |
| `--inference-wait-ms <ms>` | 0 | Queue wait before a 429 response |
| `--threads <n>` | automatic | Inference worker override |
| `--max-concurrent-inferences <n>` | 1 | Admission permits for non-batched generation |
| `--experimental-prefix-cache` | off | Enables explicit cross-request prefix reuse |
| `--experimental-residual-q8-activation-matvec` | off | Enables the supported Arm residual fast path |
| `--experimental-batched-decode` | off | Enables streaming continuous batching |
| `--max-batch-streams <n>` | none | Required batch admission limit |

`--default-max-tokens` must not exceed `--hard-max-tokens`. Batch mode requires
`--max-batch-streams`, and the residual activation policy cannot be combined
with batch mode.

## Readiness and authentication

`GET /health` is always unauthenticated. It returns `ready: false` until a
model is loaded.

When `--api-key` is set, every `/v1/*` route requires:

```text
Authorization: Bearer <api-key>
```

Ferrite validates authentication before parsing protected request bodies. This
avoids leaking request-shape information to unauthenticated clients.

## Backpressure

Without continuous batching, generation acquires an inference permit. If no
permit becomes available within `--inference-wait-ms`, Ferrite returns an
OpenAI-shaped `429 rate_limit_error`. The default wait is zero, which applies
immediate backpressure.

Increasing `--max-concurrent-inferences` permits more independent model
sessions, but can contend for memory bandwidth and increase latency. Measure it
on the deployment machine.

## Prefix cache

`--experimental-prefix-cache` enables cross-request reuse only when callers
supply the same non-empty `prompt_cache_key` and compatible model, tokenizer,
template, policy, and stop fingerprints. Reused prompt tokens are reported as
`usage.prompt_tokens_details.cached_tokens`.

Without the server flag, `prompt_cache_key` is accepted as compatibility
metadata but does not enable reuse. Prefix-cache requests bypass the continuous
batch scheduler because the two experimental paths do not share a contract yet.

## Continuous batching

Enable batching for simultaneous streaming requests:

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --experimental-batched-decode \
  --max-batch-streams 4
```

Only eligible streaming chat and completion requests enter the batch
scheduler. Non-streaming, prefix-cache, and trace-enabled requests use the
normal inference-permit path.

## Network exposure

The default bind address is localhost. Before binding to a non-loopback
address:

1. Set a strong API key.
2. Put TLS, request-size limits, access logs, and network policy in a trusted
   reverse proxy.
3. Treat prompts and generated text as sensitive data.
4. Size token limits and queue waits to bound CPU and memory use.
5. Monitor process RSS, latency, errors, and restarts.

Ferrite is not currently a complete internet-facing security boundary.
