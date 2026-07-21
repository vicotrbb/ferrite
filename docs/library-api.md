# Library API

Ferrite publishes three crates. `ferrite-model` and `ferrite-inference` are the
two production library boundaries. `ferrite-fixtures` is a source-generated
test-support API published so downstream package verification can build the
inference crate's tests. The CLI, HTTP server, and operational clients are
repository tools, not stable library contracts.

## `ferrite-fixtures`

`ferrite-fixtures` builds deterministic GGUF byte vectors entirely from
source-controlled values. It contains no model binaries and performs no network
access. Its API is intended for parser, tokenizer, loader, and inference tests,
not application inference.

## `ferrite-model`

`ferrite-model` owns safe GGUF v3 parsing, architecture-aware configuration,
tensor descriptors, metadata values, and tokenization.

The normal flow is:

```rust
use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::GgufTokenizer;

fn inspect(bytes: &[u8]) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let file = parse_gguf(bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;
    Ok(tokenizer.encode("hello")?)
}
```

Parsing validates file structure and tensor byte ranges before callers can
borrow tensor data. Recognizing a GGML type identifier does not imply that the
inference crate can execute that type.

## `ferrite-inference`

`ferrite-inference` owns model loading, execution policies, scalar sessions,
KV storage, matrix dispatch, SIMD kernels, batching, and prefix-cache identity.

The high-level sequence is:

1. Parse bytes with `ferrite-model`.
2. Load a `ScalarLlamaModel` from the validated file.
3. Start a session with default or explicit execution options.
4. Accept prompt tokens, then advance generation one token at a time.
5. Keep experimental activation and KV policies explicit.

The default policy is the compatibility baseline. Experimental policies can
change numerical behavior at the matrix level and must pass the documented
token-parity gate for each supported model and machine.

## Generate and test rustdoc

```sh
RUSTDOCFLAGS="-D warnings" \
  cargo doc -p ferrite-fixtures -p ferrite-model -p ferrite-inference \
    --all-features --no-deps --locked

cargo test -p ferrite-fixtures -p ferrite-model -p ferrite-inference \
  --all-features --doc --locked
```

All three publishable crates deny missing public documentation, malformed
rustdoc, undocumented result failures selected by policy, and builder methods
that silently discard a returned replacement value.

## Stability

Ferrite is pre-1.0 alpha software. Public types follow semantic versioning, but
minor `0.x` releases can still revise APIs. Review the
[changelog](../CHANGELOG.md) and run `cargo semver-checks` against the previous
release before publishing.
