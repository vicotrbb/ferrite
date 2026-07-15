# Getting started

This guide builds Ferrite, acquires a verified model, runs local generation,
and starts the OpenAI-compatible server.

## Requirements

- A 64-bit Windows, Linux, or macOS host. Release archives currently cover
  macOS arm64 and Linux x86_64. Other maintained CI platforms build from
  source.
- An aarch64 CPU with NEON, or an x86_64 CPU with AVX2 for the optimized paths.
- Rust 1.96.1. The repository's `rust-toolchain.toml` selects it automatically
  when Rust is installed through rustup.
- Python 3 for the optional eval harness.
- `curl` for built-in resumable model acquisition on Windows, Linux, or macOS.
- Enough disk and memory for the selected GGUF model. The reference Qwen2.5
  0.5B Q4_K_M artifact is about 469 MiB on disk and used about 557 MiB peak RSS
  in the accepted 2026-07-14 Apple M5 Pro eval. Other models can require much
  more.

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

## One-command verified first run

The built-in registry contains the official Microsoft Phi-3 Mini 4K Instruct
Q4 GGUF at one immutable revision. This command acquires and verifies the model
when absent, then generates locally:

```sh
target/release/ferrite \
  --model-id phi3-mini-4k-instruct-q4 \
  --prompt '<|user|>
Write one sentence about a lighthouse.<|end|>
<|assistant|>' \
  --generate-tokens 32 \
  --stream
```

The artifact is 2,393,231,072 bytes. Ferrite prints its source, revision,
license, filename, expected size, and SHA-256 before loading it. Acquisition is
resumable, restricted to HTTPS, and completed only after the pinned size and
hash match. The final model and `artifact.json` manifest are read-only.

The default cache root is:

| Platform | Cache root |
| --- | --- |
| macOS | `~/Library/Caches/ferrite/models` |
| Linux | `$XDG_CACHE_HOME/ferrite/models`, or `~/.cache/ferrite/models` |
| Windows | `%LOCALAPPDATA%\Ferrite\models` |

Set `FERRITE_MODEL_CACHE` or pass `--model-cache` to override the root. Pass
`--offline` to require a verified cache hit and prohibit network acquisition.

The model remains local after acquisition. Ferrite has no built-in telemetry
or remote inference fallback.

## Get the evaluation reference model

The eval harness can download the pinned reference artifact:

```sh
scripts/eval.sh --download --skip-cli --skip-server
```

That command still builds the binaries, but does not run inference phases. The
model is stored as:

```text
target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

The download is pinned to Qwen revision
`df5bf01389a39c743ab467d734bf501681e041c5`. The harness verifies the exact
491,400,032-byte size and SHA-256 before atomically publishing the cache file.
An incomplete or mismatched partial download is removed. Eval JSON for this
artifact records its Qwen source, revision, Apache-2.0 license, filename, size,
SHA-256, license URL, and download URL.

This smaller Qwen2.5 artifact is the performance harness reference, not the
built-in first-run model. Models are intentionally ignored by Git. Review each
model's license and source before using it outside local evaluation.

## Generate with an explicit model path

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
