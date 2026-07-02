# Benchmark: llama-benchy Qwen 0.5B Token IDs Smoke

Date: 2026-07-02

## Purpose

Verify that `llama-benchy 0.3.8` can consume Ferrite chat streaming chunks with
generated `token_ids`, avoiding the local-tokenization fallback observed in the
previous compatibility smoke.

This is still a tiny interoperability smoke. It is not the full 256, 512, and
1024-token protocol.

## Environment

- Ferrite commit: `2bae10f526f5237557b387fe8f24534381ca3c7f`
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
CARGO_TARGET_DIR=target/codex-chat-token-ids \
  cargo run -p ferrite-server --bin ferrite-server -- \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 8 \
  --hard-max-tokens 32
```

Readiness check:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 16 \
  --tg 8 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json
```

Observed output:

```text
Running test: pp=16, tg=8, depth=0, concurrency=1
  Run 1/1 (batch size 1)...
Saving results to documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json in JSON format...
```

The earlier fallback line was not observed:

```text
No token_ids in response, using local tokenization
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json`

## Results

- Prompt size: 16
- Response size: 8
- Concurrency: 1
- Prefix caching: false
- `pp_throughput.mean`: `5806.4823346706535`
- `tg_throughput.mean`: `0.4396310243715505`
- `ttfr.mean`: `2.755540976068005`
- `est_ppt.mean`: `2.755540976068005`
- `e2e_ttft.mean`: `52699.25154099474`

## Interpretation

Ferrite now returns generated token IDs on no-stop chat streaming content
chunks in the shape expected by `llama-benchy` for this minimal run.

This proves better external harness interoperability than the previous smoke.
It does not prove production throughput, memory behavior, prefix-cache benefit,
long-chat behavior, reconnect/error behavior, stop/EOS correctness, or the full
256/512/1024-token protocol.
