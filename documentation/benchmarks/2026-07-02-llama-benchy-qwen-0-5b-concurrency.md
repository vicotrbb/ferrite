# Benchmark: llama-benchy Qwen 0.5B Concurrency Step

Date: 2026-07-02

## Purpose

Run the first small `llama-benchy` concurrency step against Ferrite's
OpenAI-compatible `/v1/chat/completions` server.

This follows the protocol after the 256/512/1024 single-request baselines. It
uses one local model, one prompt/generation length, and concurrency levels 1, 2,
and 4. It is not a production serving benchmark.

## Environment

- Ferrite commit: `19bc8142cec7d1f7521be8552d350422bcd33559`
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

## Zero-Wait Server

Initial server command:

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

Command:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 512 \
  --tg 256 \
  --runs 1 \
  --concurrency 1 2 4 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-concurrency.json
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-concurrency.json`

Observed client output included queue-full responses:

```text
HTTP 429: {"error":{"message":"inference request queue is full; retry later","type":"rate_limit_error","param":null,"code":null}}
```

This proves the default zero-wait policy rejects excess concurrent requests
with OpenAI-shaped rate-limit errors. It is not clean queued-throughput
evidence.

## Queued Server

Queued server command:

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 300000
```

Readiness:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Command:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 512 \
  --tg 256 \
  --runs 1 \
  --concurrency 1 2 4 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-concurrency-queued.json
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-concurrency-queued.json`

The queued run exited `0` without the earlier 429 messages.

## Results

Zero-wait run:

| Concurrency | PP tok/s | TG tok/s | TG req tok/s | TTFR | E2E TTFT | Peak |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | null | 13.845706460678903 | 13.845706460678903 | 0.9334160131402314 | 27133.968958019977 | 16.0 |
| 2 | 18.927790648987838 | 13.708942744578492 | 13.708942744578492 | 1.6581250238232315 | 27155.83712499938 | 16.0 |
| 4 | 18.86386881727333 | 13.745350205756473 | 13.745350205756473 | 2.7662500215228647 | 27247.85700000939 | 16.0 |

Queued run:

| Concurrency | PP tok/s | TG tok/s | TG req tok/s | TTFR | E2E TTFT | Peak |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | null | 13.520923091874657 | 13.520923091874657 | 0.9005000174511224 | 27633.75150001957 | 15.0 |
| 2 | 13.735845805401992 | 7.803494064960598 | 13.613904487505575 | 23446.793436989537 | 51357.015937494 | 16.0 |
| 4 | 12.333808695731271 | 6.460806574464915 | 13.615371967698668 | 69325.27608349483 | 96832.88235425425 | 16.0 |

The queued run distinguishes total decode throughput from per-request decode
throughput at concurrency 2 and 4. That matches Ferrite's current one-permit
serving model: requests wait, per-request decode remains roughly stable, and
end-to-end first-token time grows with queue depth.

## RSS Sampling

RSS was sampled with `ps -o rss= -p <pid>` once per second while each server
process was alive.

Zero-wait raw RSS sample:
`target/proof/llama-benchy-qwen-0-5b-concurrency-rss.tsv`

- Samples: 173
- First sample bytes: 162873344
- Last sample bytes: 410533888
- Peak bytes: 452411392
- Minimum bytes: 2850816

The low minimum appears during startup/loading and is not treated as a steady
loaded-model RSS floor.

Queued raw RSS sample:
`target/proof/llama-benchy-qwen-0-5b-concurrency-queued-rss.tsv`

- Samples: 340
- First sample bytes: 430817280
- Last sample bytes: 426459136
- Peak bytes: 458915840
- Minimum bytes: 169803776

The queued run stayed bounded around the same loaded-model RSS range and did
not show unbounded growth during this short test.

## Interpretation

Ferrite now has external `llama-benchy` evidence for the small concurrency
step. With the default zero-wait server, excess concurrent requests return
OpenAI-shaped HTTP 429 errors. With `--inference-wait-ms 300000`, the same
external benchmark completes at concurrency 1, 2, and 4 and shows queued
single-permit behavior.

This is evidence of bounded queue behavior, not parallel inference scaling.
Ferrite still has a single inference permit.

## Limits

This does not prove:

- high-concurrency serving;
- fairness across many clients;
- long-running RSS stability;
- prefix-cache behavior;
- reconnect/error behavior under concurrent load;
- stop/EOS behavior under concurrent load;
- production throughput.
