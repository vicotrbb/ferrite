# Benchmark: llama-benchy Qwen 0.5B 128-Token Calibration

Date: 2026-07-02

## Purpose

Run a bounded `llama-benchy` 128-token calibration against Ferrite's
OpenAI-compatible chat server and compare it with Ferrite's own throughput
client on the same local server process.

This is a harness-calibration slice. It does not replace the long-chat gate,
because `llama-benchy` does not validate repeated generated-context turns,
client reconnect behavior, stop/EOS behavior, or Ferrite's cached-token usage
metadata.

## Environment

- Ferrite commit: `574330d`
- Host: local macOS development machine
- Server port: `127.0.0.1:18185`
- Server binary: `target/release/ferrite-server`
- Ferrite comparator: `target/release/ferrite-openai-throughput`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Raw proof logs:
  - `target/proof/ferrite-throughput-qwen-0-5b-128-calibration.log`
  - `target/proof/llama-benchy-qwen-0-5b-128-calibration.stdout.txt`
  - `target/proof/llama-benchy-qwen-0-5b-128-calibration-latency-none.stdout.txt`

The local server was stopped after the benchmark, and the listener check
returned no listener:

```sh
lsof -nP -iTCP:18185 -sTCP:LISTEN
```

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18185 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Ferrite Throughput Comparator

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18185 \
  --endpoint chat-completions \
  --api-key local-secret \
  --model Qwen2.5-0.5B-Instruct-Q4_K_M \
  --prompt "Write a concise operational note about CPU inference stability." \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 128 \
  --stream \
  --stream-usage \
  --rss-pid "$SERVER_PID"
```

Result:

| Metric | Value |
| --- | ---: |
| Exit code | 0 |
| Streaming token events | 129 |
| Time to first token | 679 ms |
| Streaming total elapsed | 6239 ms |
| Streaming tokens/sec | 20.673809 |
| Completion tokens | 128 |
| Prompt tokens | 15 |
| All content chunks had token IDs | true |
| Server RSS before | 431456256 |
| Server RSS after | 445104128 |
| Server RSS idle | 445104128 |

## llama-benchy Commands

Generation-latency mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18185/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 128 \
  --tg 128 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-128-calibration.json
```

Latency-none mode:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18185/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 128 \
  --tg 128 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-128-calibration-latency-none.json
```

Both commands exited `0`. Both printed warmup lines even though `--no-warmup`
was passed, matching earlier Qwen 0.5B `llama-benchy` observations.

## llama-benchy Results

| Mode | Prompt tokens | Generation tokens | TG tok/s | PP tok/s | TTFR | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| generation | 128 | 128 | 20.101652 | null | 0.946292 | 5709.923875 |
| none | 128 | 128 | 20.176806 | 92106.084464 | 1.411416 | 5688.253375 |

The generation-mode JSON is
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-128-calibration.json`.
The latency-none JSON is
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-128-calibration-latency-none.json`.

## Interpretation

The Ferrite throughput client and `llama-benchy` report consistent decode
throughput for this local 128-token shape:

- Ferrite throughput client: `20.673809` streamed tokens/sec;
- `llama-benchy` generation mode: `20.101652` generated tokens/sec;
- `llama-benchy` latency-none mode: `20.176806` generated tokens/sec.

The first-token timings are not directly comparable because the prompt shapes
are different. Ferrite's direct comparator used a short 15-token prompt and
reported `679` ms to first token. `llama-benchy` forced a 128-token prompt and
reported about `5.7` seconds end-to-end first-token time.

This supports using `llama-benchy` for external decode-throughput trend checks
after Ferrite's own protocol/correctness gates pass. It should not become the
primary proof gate for long-chat behavior, reconnect/error behavior, or
stop/EOS handling.
