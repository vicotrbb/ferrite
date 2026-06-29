# 2026-06-28 Tier 1 Qwen2.5 1.5B Q6_K HTTP Latency

## Scope

This benchmark records bounded local HTTP request latency for Ferrite's
OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q6_K.

This is server-path benchmark evidence only. It does not prove the Tier 1
throughput gate, concurrent successful real-model serving, or broader
model/prompt behavior.

## Tree State

- Branch: `main`
- Commit before run: `0004ed8`
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
  --bind 127.0.0.1:18185 \
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
- Base URL: `http://127.0.0.1:18185`
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

An initial benchmark script failed before measurement because a zsh function
variable named `path` clobbered command lookup and made `curl` unavailable
after the health check. That run is discarded. The retained run used
`route_path` instead and completed all measured requests.

## Results

| Case | Runs | Latencies | Average | Approx requests/s |
| --- | ---: | --- | ---: | ---: |
| Legacy completion | 3 | 818.868 ms, 824.305 ms, 820.412 ms | 821.195 ms | 1.218 |
| Chat completion | 3 | 2374.011 ms, 2392.155 ms, 2344.896 ms | 2370.354 ms | 0.422 |
| Legacy completion stream | 3 | 830.373 ms, 832.391 ms, 823.528 ms | 828.764 ms | 1.207 |
| Chat completion stream | 3 | 2321.392 ms, 2358.555 ms, 2302.478 ms | 2327.475 ms | 0.430 |

Server process RSS samples:

```text
rss_after_health_kib=1463920
rss_after_health_bytes=1499054080
rss_after_benchmark_kib=1515216
rss_after_benchmark_bytes=1551581184
```

## Response Checks

All measured requests returned HTTP `200`.

Legacy completion responses used:

```text
object=text_completion
model=qwen2.5-1.5b-q6_k
text=\n
prompt_tokens=2
completion_tokens=1
total_tokens=3
```

Chat completion responses used:

```text
object=chat.completion
model=qwen2.5-1.5b-q6_k
content=你好
prompt_tokens=8
completion_tokens=1
total_tokens=9
```

Streaming responses emitted OpenAI-compatible chunks followed by one
`data: [DONE]` marker per measured stream.

## Interpretation

The real Tier 1 HTTP path is usable for deterministic one-token local requests
on Qwen2.5-1.5B Q6_K across legacy completions, chat completions, and both SSE
streaming shapes. As expected from the CLI throughput benchmarks, Q6_K is
slower than Q8_0 on this local aarch64 host for this model: the Q6_K legacy
completion average was 821.195 ms versus the prior Q8_0 309.739 ms average, and
the Q6_K chat completion average was 2370.354 ms versus the prior Q8_0
805.396 ms average.

This result should not be treated as a throughput pass. The benchmark uses a
single local client, a single model, a single prompt, one generated token, and
three measured requests per case. Broader throughput, concurrency, and longer
generation behavior remain unproven.
