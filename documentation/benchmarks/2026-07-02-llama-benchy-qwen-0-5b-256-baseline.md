# Benchmark: llama-benchy Qwen 0.5B 256-Token Baseline

Date: 2026-07-02

## Purpose

Run the first bounded 256-token `llama-benchy` baseline against Ferrite's
OpenAI-compatible `/v1/chat/completions` server and compare it with the nearest
Ferrite long-chat timing artifact.

This is one model, one token length, one concurrency level, and one local host.
It is not the full 256/512/1024-token protocol.

## Environment

- Ferrite commit: `26cc2d6231db6900061e882f5c76d6b02bedb723`
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
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Readiness:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

An initial attempt against a server started with `--api-key local-secret`
failed during `llama-benchy` warmup with HTTP 401. No JSON result was written
for that failed attempt. The successful baseline used the unauthenticated local
server shape used by the earlier `llama-benchy` compatibility smokes.

## Commands

Generation-latency mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 256 \
  --tg 256 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-256-baseline.json
```

Latency-none mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 256 \
  --tg 256 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-256-baseline-latency-none.json
```

Both successful runs printed warmup lines even though `--no-warmup` was passed.
Record the observed behavior rather than assuming the flag suppressed warmup.

## Raw Results

- Generation-latency raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-256-baseline.json`
- Latency-none raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-256-baseline-latency-none.json`

Both JSON files validate with `python3 -m json.tool`.

## Results

| Mode | Prompt | Response | Concurrency | Latency ms | PP tok/s | TG tok/s | TTFR | Est PPT | E2E TTFT |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| generation | 256 | 256 | 1 | 1.283166668144986 | null | 17.108893401529688 | 0.8937079983297735 | 0.0 | 11560.583166981814 |
| none | 256 | 256 | 1 | 0.0 | 294450.6291659168 | 16.579741995461383 | 0.8762080105952919 | 0.8762080105952919 | 11649.098833004246 |

The generation-latency run is useful for decode and first-token measurements
but did not emit prompt-processing throughput. The latency-none run emitted
prompt-processing throughput and similar decode throughput, but it intentionally
skipped latency measurement.

## Comparison Artifact

Nearest Ferrite long-chat artifact:
`documentation/benchmarks/2026-07-01-openai-long-chat-qwen-0-5b-generated-context-probe-256.md`

That run used the same local host class, same model, release build, and 256
completion tokens, but it exercised four repeated streaming chat turns,
generated assistant context carry-forward, error probe, disconnect probe, and
RSS sampling.

| Source | Context | Completion tokens | TTFT ms | Stream tok/s | RSS |
| --- | --- | ---: | ---: | ---: | --- |
| llama-benchy generation | single request | 256 | 11560.583 e2e TTFT | 17.108893 TG | not sampled |
| llama-benchy none | single request | 256 | 11649.099 e2e TTFT | 16.579742 TG | not sampled |
| Ferrite long-chat turn 1 | seed | 256 | 1948 | 16.771144 | before/after/idle present |
| Ferrite long-chat turn 2 | generated | 256 | 14790 | 7.854043 | before/after/idle present |
| Ferrite long-chat turn 3 | generated | 256 | 14938 | 7.344736 | before/after/idle present |
| Ferrite long-chat turn 4 | generated | 256 | 17953 | 7.110644 | before/after/idle present |

The two tools do not measure identical request shapes. `llama-benchy` gives a
single external harness baseline. Ferrite's long-chat gate proves repeated
turns, generated-context reuse, reconnect/error behavior, and RSS sampling.

## Interpretation

Ferrite now has a bounded 256-token `llama-benchy` baseline against a real local
Tier 1 model. The external tool can complete one 256 prompt-token and 256
generation-token run without Ferrite-specific patches, and the results can be
compared to Ferrite's own long-chat artifacts.

The result also exposes a measurement nuance: `llama-benchy --latency-mode
generation` produced `pp_throughput: null`, while `--latency-mode none`
produced prompt-processing throughput. Future protocol runs should preserve
both modes or confirm a better single mode before treating the harness as a
standard gate.

## Limits

This does not prove:

- the 512-token or 1024-token `llama-benchy` cases;
- concurrency above 1;
- prefix-cache behavior;
- RSS before/after/idle behavior for `llama-benchy`;
- repeated multi-turn conversations;
- reconnect/error behavior;
- stop/EOS behavior;
- production throughput.
