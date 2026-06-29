# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 HTTP Latency

## Scope

This benchmark records bounded local HTTP request latency for Ferrite's
OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q8_0.

This is server-path benchmark evidence only. It does not prove the Tier 1
throughput gate, concurrent successful real-model serving, or broader
model/prompt behavior.

## Tree State

- Branch: `main`
- Commit before run: `f919aea`
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

/usr/bin/time -l target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0 \
  --bind 127.0.0.1:18181 \
  --default-max-tokens 1 \
  --hard-max-tokens 16
```

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URL: `http://127.0.0.1:18181`
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
| Legacy completion | 3 | 313.404 ms, 298.828 ms, 316.986 ms | 309.739 ms | 3.23 |
| Chat completion | 3 | 816.222 ms, 815.293 ms, 784.674 ms | 805.396 ms | 1.24 |
| Legacy completion stream | 3 | 317.888 ms, 315.916 ms, 316.136 ms | 316.647 ms | 3.16 |
| Chat completion stream | 3 | 811.311 ms, 817.676 ms, 828.215 ms | 819.067 ms | 1.22 |

Server process resource summary after the bounded run and response checks:

```text
72.33 real
21.19 user
3.77 sys
3822059520 maximum resident set size
3823182208 peak memory footprint
```

## Response Checks

All measured requests returned HTTP `200`.

Legacy completion responses used:

```text
object=text_completion
model=qwen2.5-1.5b-q8_0
text=\n
prompt_tokens=2
completion_tokens=1
total_tokens=3
```

Chat completion responses used:

```text
object=chat.completion
model=qwen2.5-1.5b-q8_0
content=你好
prompt_tokens=8
completion_tokens=1
total_tokens=9
```

Streaming responses used `content-type: text/event-stream` and emitted
OpenAI-compatible chunks followed by `data: [DONE]`.

During an accidental concurrent header probe, the second simultaneous stream
returned the expected OpenAI-shaped `429 Too Many Requests` response from the
single-inference permit. The successful stream header checks above were rerun
sequentially and returned `HTTP/1.1 200 OK`.

## Interpretation

The real Tier 1 HTTP path is usable for deterministic one-token local requests
on Qwen2.5-1.5B Q8_0. The legacy completion path remains faster than the chat
path in this slice because the chat prompt renders to more prompt tokens and
therefore performs more prompt work before the single generated token.

This result should not be treated as a throughput pass. The benchmark uses a
single local client, a single model, a single prompt, one generated token, and
three measured requests per case. Broader throughput, concurrency, and longer
generation behavior remain unproven.
