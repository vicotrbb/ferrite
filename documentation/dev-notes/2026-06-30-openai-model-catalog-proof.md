# OpenAI Model Catalog Endpoint Proof

Date: 2026-06-30

## Scope

This note records fresh fixture-server evidence for Ferrite's OpenAI-compatible
model catalog endpoints:

- `GET /v1/models`
- `GET /v1/models/{model}`

The proof covers raw HTTP requests and the standard `async-openai` client. It
does not prove every OpenAI model-management endpoint, provider metadata parity,
real-model serving throughput, or broader client ecosystem compatibility.

## Implementation Surface

- Routes: `crates/ferrite-server/src/openai/routes.rs`
- Handlers: `crates/ferrite-server/src/openai/catalog.rs`
- Raw HTTP tests: `crates/ferrite-server/tests/openai_http.rs`
- Standard client tests: `crates/ferrite-server/tests/openai_client_catalog.rs`

The catalog route returns OpenAI-shaped model objects for the loaded local model
and supports retrieving a loaded model by id. The route also handles percent
encoded slash characters in model ids, which is needed for Hugging Face style
ids such as `HuggingFaceTB/SmolLM2-135M-Instruct`.

## Verification

Command:

```sh
cargo test -p ferrite-server --test openai_client_catalog -- --nocapture
```

Result:

```text
running 2 tests
test async_openai_client_lists_ferrite_model ... ok
test async_openai_client_retrieves_ferrite_model ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

Command:

```sh
cargo test -p ferrite-server --test openai_http model -- --nocapture
```

Result:

```text
running 3 tests
test live_http_server_accepts_openai_style_model_retrieve ... ok
test live_http_server_retrieves_encoded_slash_model_id ... ok
test live_http_server_accepts_openai_style_model_list ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.01s
```

## Interpretation

Ferrite's local OpenAI-compatible server can expose the loaded model through
OpenAI-shaped catalog responses over raw HTTP and through `async-openai` model
list/retrieve calls. This strengthens the local base-URL product path expected
from an Ollama-like OpenAI-compatible service.

The result remains fixture-server compatibility evidence. It does not prove
all OpenAI API catalog semantics, dynamic multi-model catalogs, production auth
policy, or real-model endpoint behavior under load.
