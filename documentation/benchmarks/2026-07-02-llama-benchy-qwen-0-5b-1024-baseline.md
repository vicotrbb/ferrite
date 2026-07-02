# Benchmark: llama-benchy Qwen 0.5B 1024-Token Baseline

Date: 2026-07-02

## Purpose

Run the bounded 1024-token `llama-benchy` baseline against Ferrite's
OpenAI-compatible `/v1/chat/completions` server and compare it with the nearest
Ferrite long-chat timing artifact.

This completes the first single-model `llama-benchy` 256/512/1024 baseline
matrix for local Qwen 0.5B at concurrency 1. It is not the full long-chat proof
gate because it does not cover repeated turns, RSS, reconnect/error behavior,
stop/EOS behavior, concurrency, or prefix caching.

## Environment

- Ferrite commit: `f732fcb0033dfa566b20e6ec471262776247fe55`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server binary SHA256:
  `652393f177907ba1a01e7e72f9dcd131c5701da694117b6f07477bfb9aebfa35`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>

## Model

- Name: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
```

Readiness:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Commands

Generation-latency mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 1024 \
  --tg 1024 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-1024-baseline.json
```

Latency-none mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 1024 \
  --tg 1024 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-1024-baseline-latency-none.json
```

Both successful runs printed warmup lines even though `--no-warmup` was passed.
Record the observed behavior rather than assuming the flag suppressed warmup.

## Raw Results

- Generation-latency raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-1024-baseline.json`
- Latency-none raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-1024-baseline-latency-none.json`

Both JSON files validate with `python3 -m json.tool`.

## Results

| Mode | Prompt | Response | Concurrency | Latency ms | PP tok/s | TG tok/s | TTFR | Est PPT | E2E TTFT |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| generation | 1024 | 1024 | 1 | 1.1670693347696215 | null | 8.071044619458204 | 0.9220829815603793 | 0.0 | 69507.34637497226 |
| none | 1024 | 1024 | 1 | 0.0 | 1128453.4269715385 | 8.053815994759002 | 0.9092089894693345 | 0.9092089894693345 | 68032.71204198245 |

As with the 256-token and 512-token runs, generation-latency mode is useful for
decode and first-token fields but did not emit prompt-processing throughput.
Latency-none mode emitted prompt-processing throughput and similar decode
throughput, but it intentionally skipped latency measurement.

## Comparison Artifact

Nearest Ferrite long-chat artifact:
`documentation/benchmarks/2026-07-01-openai-long-chat-qwen-0-5b-generated-context-probe-1024.md`

That run used the same local host class, same model, release build, and 1024
completion tokens, but it exercised four repeated streaming chat turns,
generated assistant context carry-forward, error probe, disconnect probe, and
RSS sampling.

| Source | Context | Completion tokens | TTFT ms | Stream tok/s | RSS |
| --- | --- | ---: | ---: | ---: | --- |
| llama-benchy generation | single request | 1024 | 69507.346 e2e TTFT | 8.071045 TG | not sampled |
| llama-benchy none | single request | 1024 | 68032.712 e2e TTFT | 8.053816 TG | not sampled |
| Ferrite long-chat turn 1 | seed | 1024 | 2304 | 12.301775 | before/after/idle present |
| Ferrite long-chat turn 2 | generated | 1024 | 86550 | 4.328482 | before/after/idle present |
| Ferrite long-chat turn 3 | generated | 1024 | 118010 | 3.348282 | before/after/idle present |
| Ferrite long-chat turn 4 | generated | 1024 | 78188 | 3.003520 | before/after/idle present |

The two tools do not measure identical request shapes. `llama-benchy` gives a
single external harness baseline. Ferrite's long-chat gate proves repeated
turns, generated-context reuse, reconnect/error behavior, and RSS sampling.

## Interpretation

Ferrite now has bounded 256-token, 512-token, and 1024-token `llama-benchy`
baselines against the same real local Tier 1 model. The external tool can
complete one 1024 prompt-token and 1024 generation-token run without
Ferrite-specific patches, and the result can be compared to Ferrite's own
long-chat artifact for the same completion length.

The 1024-token result reinforces the measurement nuance from the shorter runs:
preserve both `generation` and `none` latency modes until the benchmark protocol
settles on a single mode that emits all required fields.

## Limits

This does not prove:

- concurrency above 1;
- prefix-cache behavior;
- RSS before/after/idle behavior for `llama-benchy`;
- repeated multi-turn conversations;
- reconnect/error behavior;
- stop/EOS behavior;
- production throughput.
