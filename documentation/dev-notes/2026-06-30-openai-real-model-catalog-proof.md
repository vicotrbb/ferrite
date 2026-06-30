# OpenAI Real-Model Catalog Endpoint Proof

Date: 2026-06-30

## Scope

This note records a bounded local proof that Ferrite's OpenAI-compatible model
catalog endpoints work after loading a real Tier 1 GGUF model.

Covered routes:

- `GET /health`
- `GET /v1/models`
- `GET /v1/models/{model}`

This complements the fixture-server raw HTTP and `async-openai` catalog proof.
It includes repeatable ignored raw HTTP and `async-openai` integration tests
against a real loaded Tier 1 model. It also covers provider-style model IDs
containing `/` through percent-encoded raw HTTP paths and standard-client
literal slash paths. It does not prove dynamic multi-model catalogs, provider
metadata parity, all OpenAI model-management semantics, or catalog behavior
under long-running load.

## Environment

- Commit before documentation: `54e906a`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.24s
```

## Model

- Model: Qwen2.5-0.5B-Instruct Q4_K_M GGUF
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Server model ID: `qwen2.5-0.5b-q4_k_m-catalog-proof`

## Server

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m-catalog-proof \
  --bind 127.0.0.1:18104 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

## Verification

Repeatable ignored test:

```sh
cargo test -p ferrite-server --test openai_client_real_tier1_catalog -- --ignored --nocapture
```

Result:

```text
running 2 tests
test async_openai_client_retrieves_real_tier1_provider_style_model_id ... ok
test async_openai_client_lists_and_retrieves_real_tier1_model ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s
```

Repeatable ignored raw HTTP test:

```sh
cargo test -p ferrite-server --test openai_real_tier1_catalog -- --ignored --nocapture
```

Result:

```text
running 2 tests
test live_http_server_lists_and_retrieves_real_tier1_model ... ok
test live_http_server_retrieves_real_tier1_provider_style_model_id ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
```

Repeatable ignored raw HTTP provider-style id test:

```sh
cargo test -p ferrite-server --test openai_real_tier1_catalog live_http_server_retrieves_real_tier1_provider_style_model_id -- --ignored --nocapture
```

Result:

```text
running 1 test
test live_http_server_retrieves_real_tier1_provider_style_model_id ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.91s
```

The probe used Python `urllib` with `Authorization: Bearer local-secret`.

Result:

```json
{
  "health": {
    "model": "qwen2.5-0.5b-q4_k_m-catalog-proof",
    "ready": true,
    "status": "ok"
  },
  "health_status": 200,
  "models": {
    "data": [
      {
        "created": 0,
        "id": "qwen2.5-0.5b-q4_k_m-catalog-proof",
        "object": "model",
        "owned_by": "ferrite"
      }
    ],
    "object": "list"
  },
  "models_status": 200,
  "retrieve": {
    "created": 0,
    "id": "qwen2.5-0.5b-q4_k_m-catalog-proof",
    "object": "model",
    "owned_by": "ferrite"
  },
  "retrieve_status": 200
}
```

After the probe, `lsof -nP -iTCP:18104 -sTCP:LISTEN` returned no listener.

## Interpretation

Ferrite's OpenAI-compatible catalog route exposes a loaded real Qwen2.5-0.5B
Q4_K_M model through `GET /v1/models` and `GET /v1/models/{model}` with
OpenAI-shaped model objects. The route is now covered by a one-off raw HTTP
real-model probe, a repeatable raw HTTP real-model integration test, and a
repeatable `async-openai` real-model integration test. A separate raw HTTP
real-model check verifies retrieval when the loaded model id contains a
provider-style slash and the request path percent-encodes it. The
`async-openai` real-model catalog test also verifies provider-style IDs when
the client sends literal slash path segments. This strengthens the local
base-URL service path for users wiring OpenAI-compatible clients to a Ferrite
server.

This remains a single-model, local aarch64 proof. It does not prove dynamic
multi-model serving, catalog pagination, provider metadata parity, x86_64
server behavior, or long-running catalog behavior under load.
