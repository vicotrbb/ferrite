# Getting started

This guide builds Ferrite, downloads the small reference model, runs local
generation, and starts the OpenAI-compatible server.

## Requirements

- A 64-bit Linux or macOS host.
- An aarch64 CPU with NEON, or an x86_64 CPU with AVX2 for the optimized paths.
- Rust 1.96.1. The repository's `rust-toolchain.toml` selects it automatically
  when Rust is installed through rustup.
- Python 3 for the optional eval harness.
- Enough disk and memory for the selected GGUF model. The reference Qwen2.5
  0.5B Q4_K_M artifact is about 469 MiB on disk and used about 1.0 GiB peak RSS
  in the recorded Apple M5 Pro eval. Other models can require much more.

For a versioned binary install, see [installation and verification](install.md)
first. Building from source remains the right path for development and for
experimental feature combinations.

## Build

```sh
git clone https://github.com/vicotrbb/ferrite.git
cd ferrite
cargo build --release --locked -p ferrite-cli -p ferrite-server
```

Always use a release build for inference measurements. Debug builds include
checks and omit the optimizer, so their speed is not representative.

## Get the reference model

The eval harness can download the current reference artifact:

```sh
scripts/eval.sh --download --skip-cli --skip-server
```

That command still builds the binaries, but does not run inference phases. The
model is stored as:

```text
target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

Models are intentionally ignored by Git. Review the model's license and source
before using it outside local evaluation.

## Generate text

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write one sentence about a lighthouse.' \
  --generate-tokens 32 \
  --stream
```

Ferrite prints machine-readable `key=value` lines. With `--stream`, each token
is reported immediately as `stream_token_id` and `stream_text`.

## Start the server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:8080 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Check readiness:

```sh
curl http://127.0.0.1:8080/health
```

Send a streaming chat request:

```sh
curl -N http://127.0.0.1:8080/v1/chat/completions \
  -H 'authorization: Bearer local-secret' \
  -H 'content-type: application/json' \
  -d '{"model":"qwen2.5-0.5b-q4_k_m","messages":[{"role":"user","content":"Hello"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}'
```

Use `http://127.0.0.1:8080/v1` as the base URL for OpenAI-compatible clients.

## Next steps

- Follow [the performance golden path](performance.md) before tuning threads or
  enabling experimental kernels.
- Read [models and tensor formats](models.md) before selecting a larger model.
- Read [the server guide](server.md) before exposing Ferrite beyond localhost.
