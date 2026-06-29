# 2026-06-28 Tier 1 Qwen2.5 1.5B Q6_K Server Memory

## Scope

This benchmark records bounded local RSS samples for Ferrite's
OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q6_K.

This is server memory evidence for one local model, one prompt, one-token
requests, and sequential request cycles. It does not prove concurrent server
memory behavior, longer-running steady-state RSS, long-context KV-cache growth,
or broader Tier 1 memory posture.

## Tree State

- Branch: `main`
- Commit before run: `4ddde1e`
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
- File: `qwen2.5-1.5b-instruct-q6_k.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- File size: 1.4 GB from `ls -lh`
- SHA-256:
  `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7`
- Ferrite model id for this run: `qwen2.5-1.5b-q6_k`

## Server Command

```sh
cargo build --release -p ferrite-server

target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k \
  --bind 127.0.0.1:18184 \
  --default-max-tokens 1 \
  --hard-max-tokens 16
```

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URL: `http://127.0.0.1:18184`
- Prompt: `hello world`
- Chat messages: one user message with `hello world`
- Generation limit: one token
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- Request cases:
  - `POST /v1/completions`
  - `POST /v1/chat/completions`
  - `POST /v1/completions` with `stream: true`
  - `POST /v1/chat/completions` with `stream: true`

## Repeated Sequential RSS Results

The pass ran three sequential cycles. Each cycle issued all four request cases
above and then sampled the direct server process RSS.

| Point | HTTP status set | Stream DONE markers | Chat stream DONE markers | RSS KiB | RSS bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| After health | n/a | n/a | n/a | 1,463,824 | 1,498,955,776 |
| After cycle 1 | 200, 200, 200, 200 | 1 | 1 | 1,497,328 | 1,533,263,872 |
| After cycle 2 | 200, 200, 200, 200 | 1 | 1 | 1,504,688 | 1,540,800,512 |
| After cycle 3 | 200, 200, 200, 200 | 1 | 1 | 1,507,904 | 1,544,093,696 |
| After two-second idle | n/a | n/a | n/a | 1,507,904 | 1,544,093,696 |

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

## Interpretation

The local Qwen2.5-1.5B Q6_K server path loads at about 1.46 GiB current RSS and
stays below 1.51 million KiB, about 1.54 GB, across this bounded one-token
sequential HTTP probe. The repeated pass increased by about 43.0 MiB from
post-load to the post-idle sample after three full endpoint cycles.

This complements the Q8_0 server memory probe with a lower-retained-memory
Qwen2.5-1.5B quantization. It is not a leak test, a concurrency memory test, or
a long-context memory test.
