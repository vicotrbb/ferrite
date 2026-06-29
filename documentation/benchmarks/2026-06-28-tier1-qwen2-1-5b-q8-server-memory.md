# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 Server Memory

## Scope

This benchmark records bounded local RSS samples for Ferrite's
OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q8_0.

This is server memory evidence for one local model, one prompt, one-token
requests, and sequential request cycles. It does not prove concurrent server
memory behavior, longer-running steady-state RSS, long-context KV-cache growth,
or broader Tier 1 memory posture.

## Tree State

- Branch: `main`
- Commit before run: `75d2435`
- Working tree before run: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- Physical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

Commands:

```sh
sysctl -n machdep.cpu.brand_string hw.ncpu hw.physicalcpu hw.logicalcpu hw.memsize
uname -a
```

## Model

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q8_0.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- File size: 1.8 GB from `ls -lh`
- SHA-256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Ferrite model id for this run: `qwen2.5-1.5b-q8_0`

## Server Command

```sh
cargo build --release -p ferrite-server

target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0 \
  --bind 127.0.0.1:18182 \
  --default-max-tokens 1 \
  --hard-max-tokens 16
```

The retained current-RSS samples run `target/release/ferrite-server` directly.
An attempted sampler wrapped the server in `/usr/bin/time -l`, which made `$!`
refer to the time wrapper and produced invalid `1120 KiB` RSS samples. Those
wrapper samples are rejected.

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URLs:
  - one-shot pass: `http://127.0.0.1:18182`
  - repeated pass: `http://127.0.0.1:18183`
- Prompt: `hello world`
- Chat messages: one user message with `hello world`
- Generation limit: one token
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- Request cases:
  - `POST /v1/completions`
  - `POST /v1/chat/completions`
  - `POST /v1/completions` with `stream: true`
  - `POST /v1/chat/completions` with `stream: true`

## One-Shot RSS Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health, sample 1 | 1,882,944 | 1,928,134,656 |
| After health, sample 2 | 1,882,944 | 1,928,134,656 |
| After legacy completion | 1,896,336 | 1,941,848,064 |
| After chat completion | 1,898,864 | 1,944,436,736 |
| After legacy completion stream | 1,903,536 | 1,949,220,864 |
| After chat completion stream | 1,903,824 | 1,949,515,776 |
| After one-second idle | 1,903,824 | 1,949,515,776 |

All four request cases returned HTTP `200`. Both streaming responses emitted
one `data: [DONE]` marker.

Representative response checks:

```text
completion object=text_completion
completion text=\n
completion usage.prompt_tokens=2
completion usage.completion_tokens=1
completion usage.total_tokens=3
chat object=chat.completion
chat content=你好
chat usage.prompt_tokens=8
chat usage.completion_tokens=1
chat usage.total_tokens=9
```

## Repeated Sequential RSS Results

The repeated pass ran three sequential cycles. Each cycle issued all four
request cases above and then sampled the direct server process RSS.

| Point | HTTP status set | Stream DONE markers | Chat stream DONE markers | RSS KiB | RSS bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| After health | n/a | n/a | n/a | 1,882,848 | 1,928,036,352 |
| After cycle 1 | 200, 200, 200, 200 | 1 | 1 | 1,901,664 | 1,947,303,936 |
| After cycle 2 | 200, 200, 200, 200 | 1 | 1 | 1,910,192 | 1,956,036,608 |
| After cycle 3 | 200, 200, 200, 200 | 1 | 1 | 1,912,944 | 1,958,854,656 |
| After two-second idle | n/a | n/a | n/a | 1,912,944 | 1,958,854,656 |

## Interpretation

The local Qwen2.5-1.5B Q8_0 server path loads at about 1.88 GiB current RSS and
stays below 1.92 GiB current RSS across this bounded one-token sequential HTTP
probe. The repeated pass increased by about 29.4 MiB from post-load to the
post-idle sample after three full endpoint cycles.

This narrows the Tier 1 server-memory gap for the local Qwen2.5-1.5B Q8_0
OpenAI-compatible server path. It is not a leak test, a concurrency memory
test, or a long-context memory test.
