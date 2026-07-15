# HTTP server

`ferrite-server` loads one model and exposes a local OpenAI-compatible API.
Treat the configured GGUF artifact as immutable for the complete server
lifetime; replacing or truncating a live mapped model file is unsupported.
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
| `--kernel-provider <auto\|portable>` | `auto` | Runtime-gated optimized kernels or the portable oracle |
| `--max-concurrent-inferences <n>` | 1 | Admission permits for non-batched generation |
| `--experimental-prefix-cache` | off | Enables explicit cross-request prefix reuse |
| `--prefix-cache-max-entries <n>` | 8 | Maximum retained immutable prefix snapshots |
| `--prefix-cache-max-mib <n>` | 64 | Maximum logical KV MiB retained by prefix snapshots |
| `--experimental-residual-q8-activation-matvec` | off | Enables the supported Arm residual fast path |
| `--experimental-batched-decode` | off | Enables greedy continuous batching |
| `--max-batch-streams <n>` | none | Required batch admission limit |
| `--max-batch-queue <n>` | batch streams | Maximum jobs waiting behind active batch streams |
| `--kv-backend <vec\|locus>` | `vec` | Session KV storage policy |
| `--kv-tokens-per-block <n>` | 16 | Locus block granularity |
| `--kv-max-tokens <n>` | none | Required per-session Locus token capacity |

`--default-max-tokens` must not exceed `--hard-max-tokens`. Batch mode requires
`--max-batch-streams`, and the residual activation policy cannot be combined
with batch mode. Prefix-cache limit flags require the prefix-cache opt-in.
Locus sizing flags require `--kv-backend locus`, and Locus always requires an
explicit token capacity.

The automatic server thread count reserves one recommended CPU slot for HTTP
work when the normal kernel policy would otherwise use the complete topology
recommendation. Explicit `--threads`, `FERRITE_THREADS`, and
`RAYON_NUM_THREADS` values remain exact.

## Local API surface

Ferrite serves models, legacy completions, chat completions, and a bounded
non-streaming Responses text endpoint. Chat JSON-object generation is
grammar-constrained. Compatible Qwen ChatML models can return parsed function
calls, but Ferrite never authorizes or executes a tool. See
[OpenAI API compatibility](openai-api.md) for exact fields, limits, and
rejections.

The server has no built-in telemetry, hosted model fallback, stored response
service, or outbound prompt path. Model loading and inference remain local.

## Readiness and authentication

`GET /health` is always unauthenticated. It returns `ready: false` until a
model is loaded.

When `--api-key` is set, every `/v1/*` route requires:

```text
Authorization: Bearer <api-key>
```

Ferrite validates authentication before parsing protected request bodies. This
avoids leaking request-shape information to unauthenticated clients.
Authenticated JSON bodies have an explicit 2 MiB limit before deserialization.
This preserves bounded memory use for malformed or hostile requests.

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
template, execution policy, and exact token prefix. Reused prompt tokens are
reported as `usage.prompt_tokens_details.cached_tokens` for chat and legacy
completions, and as `usage.input_tokens_details.cached_tokens` for Responses.

Without the server flag, `prompt_cache_key` is accepted as compatibility
metadata but does not enable reuse. Cache entries are immutable snapshot leases
with reference-counted ownership. Eviction removes the cache's owner while any
request already restoring the snapshot keeps a valid lease. Entry and logical
KV-byte limits are enforced together.

## Continuous batching

Enable batching for simultaneous streaming requests:

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --experimental-prefix-cache \
  --prefix-cache-max-entries 8 \
  --prefix-cache-max-mib 64 \
  --experimental-batched-decode \
  --max-batch-streams 4 \
  --max-batch-queue 4
```

Eligible fused-greedy chat, completion, and non-streaming Responses requests
use one scheduler. This includes prefix-cache and cache-trace requests. Sampled
or logit-modified requests use the normal inference-permit path because they
need full logits.

For each scheduled request, Ferrite finds the longest compatible cached token
prefix, restores its immutable snapshot, batches only the uncached suffix, and
then advances each ready stream by one decode token per scheduler cycle. Equal
prompts are evaluated once only when their complete cache options, including
namespace, match. A slow client pauses only its own stream through a bounded
event channel. Closed receivers are removed before admission, during prefill,
or at the next decode boundary. Active streams and queued jobs have separate
limits, and exceeding admission returns an OpenAI-shaped 429 response.

## Bounded block KV storage

The default `vec` backend preserves the established storage path. To opt into
fixed-size mapped KV blocks, build and run the server with the Locus feature:

```sh
cargo build --release --locked -p ferrite-server --features locus-kv

target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --kv-backend locus \
  --kv-tokens-per-block 16 \
  --kv-max-tokens 8192
```

`--kv-max-tokens` is a per-session capacity for the complete prompt plus decode
state. Ferrite rejects a request whose worst-case KV need exceeds the cap before
prompt evaluation begins. The process-level upper bound is the per-session
capacity multiplied by admitted normal and batched sessions, plus the separate
prefix-snapshot budget and model memory. Locus pool exhaustion is an explicit
error, never an unbounded allocation fallback.

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

## Operational validation

Use the built-in throughput client for request-rate, token-latency, usage, and
RSS measurements. Use the long-chat gate for cancellation, queue recovery,
cache, finish-source, and multi-turn proofs. Both are documented in
[operational tools](benchmark-tools.md).
