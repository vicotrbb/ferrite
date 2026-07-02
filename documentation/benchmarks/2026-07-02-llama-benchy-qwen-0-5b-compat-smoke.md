# Benchmark: llama-benchy Qwen 0.5B Compatibility Smoke

Date: 2026-07-02

## Purpose

Test whether `llama-benchy` can drive Ferrite's OpenAI-compatible
`/v1/chat/completions` streaming endpoint against a real local Tier 1 model and
write a machine-readable result artifact.

This is a compatibility smoke. It is not the full 256, 512, and 1024-token
protocol from `documentation/benchmarks/2026-07-02-llama-benchy-openai-protocol.md`.

## Environment

- Pre-change Ferrite commit: `562041a`
- Post-change Ferrite commit: `32d6425`
- Host: local macOS development machine
- Execution target: local Ferrite server on `127.0.0.1:18080`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>

## Model

- Name: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`

## Server Command

```sh
cargo run -p ferrite-server --bin ferrite-server -- \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 16 \
  --hard-max-tokens 64
```

Readiness check:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Before Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 32 \
  --tg 16 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke.json
```

Result: command exited `0`, but the benchmark request failed with:

```text
HTTP 400: {"error":{"message":"unsupported chat completion field(s): return_token_ids","type":"invalid_request_error","param":"return_token_ids","code":null}}
```

Raw result: `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke.json`

The raw result contains null throughput and latency metrics because no
successful benchmark request completed.

## Compatibility Change

Ferrite now accepts `return_token_ids` as a typed OpenAI-compatible extension
when the value is boolean or null. Ferrite does not yet return token ids in chat
completion chunks; `llama-benchy` observed that and fell back to local
tokenization.

## After Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 32 \
  --tg 16 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke-after-return-token-ids.json
```

Observed output:

```text
Running test: pp=32, tg=16, depth=0, concurrency=1
  Run 1/1 (batch size 1)...
  No token_ids in response, using local tokenization
Saving results to documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke-after-return-token-ids.json in JSON format...
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke-after-return-token-ids.json`

## Results

- Prompt size: 32
- Response size: 16
- Concurrency: 1
- Prefix caching: false
- `pp_throughput.mean`: `8981.40740715751`
- `tg_throughput.mean`: `0.4081084677215171`
- `ttfr.mean`: `3.5629159829113632`
- `est_ppt.mean`: `3.5629159829113632`
- `e2e_ttft.mean`: `92971.50650000549`

## Interpretation

This proves that `llama-benchy 0.3.8` can complete a minimal streaming chat
benchmark request against Ferrite after accepting the `return_token_ids`
extension.

It does not prove production throughput, memory behavior, prefix-cache benefit,
long-chat behavior, reconnect/error behavior, stop/EOS correctness, or the full
256/512/1024-token protocol. The next useful benchmark slice is a bounded
single-model protocol run at 256 tokens before expanding to the full matrix.
