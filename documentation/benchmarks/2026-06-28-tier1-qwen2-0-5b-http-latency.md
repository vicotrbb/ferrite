# 2026-06-28 Tier 1 Qwen2.5 0.5B HTTP Latency

## Scope

This benchmark records bounded local HTTP request latency for Ferrite's
OpenAI-compatible server running Qwen2.5-0.5B-Instruct Q4_K_M.

This is server-path benchmark evidence only. It does not prove the Tier 1
throughput gate, concurrent real-model serving, or broader model/prompt
behavior.

## Tree State

- Branch: `main`
- Commit before run: `bab06c3`
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

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- File size: 379 MB from `ls -lh`
- Ferrite model id for this run: `qwen2.5-0.5b`

## Server Command

```sh
/usr/bin/time -l target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b \
  --bind 127.0.0.1:18080 \
  --default-max-tokens 1 \
  --hard-max-tokens 16
```

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-0.5b"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URL: `http://127.0.0.1:18080`
- Prompt: `hello world`
- Chat messages: one user message with `hello world`
- Generation limit: one token
- Warmup: one request per case, not included in averages
- Measured runs: three requests per case
- Cases:
  - `POST /v1/completions`
  - `POST /v1/chat/completions`
  - `POST /v1/completions` with `stream: true`
  - `POST /v1/chat/completions` with `stream: true`

## Results

| Case | Runs | Latencies | Average | Approx requests/s |
| --- | ---: | --- | ---: | ---: |
| Legacy completion | 3 | 213.032 ms, 243.251 ms, 266.548 ms | 240.944 ms | 4.15 |
| Chat completion | 3 | 627.284 ms, 511.681 ms, 582.201 ms | 573.722 ms | 1.74 |
| Legacy completion stream | 3 | 213.485 ms, 295.528 ms, 246.828 ms | 251.947 ms | 3.97 |
| Chat completion stream | 3 | 633.284 ms, 719.125 ms, 576.223 ms | 642.878 ms | 1.56 |

Server process resource summary after the bounded run:

```text
63.63 real
14.26 user
3.52 sys
771964928 maximum resident set size
828197952 peak memory footprint
```

## Response Checks

All measured requests returned HTTP `200`.

Legacy completion responses used:

```text
object=text_completion
prompt_tokens=2
completion_tokens=1
```

Chat completion responses used:

```text
object=chat.completion
content=你好
```

Streaming responses used `content-type: text/event-stream` and emitted
OpenAI-compatible chunks followed by `data: [DONE]`.

## Interpretation

The real Tier 1 HTTP path is usable for deterministic one-token local requests
on Qwen2.5-0.5B Q4_K_M. The legacy completion path is faster than the chat path
in this slice because the chat prompt renders to more prompt tokens and
therefore performs more prompt work before the single generated token.

This result should not be treated as a throughput pass. The benchmark uses a
single local client, a single model, a single prompt, one generated token, and
three measured requests per case. Broader throughput, concurrency, and longer
generation behavior remain unproven.
