# 2026-06-28 Tier 1 Qwen2.5 1.5B Q6_K Queue Memory

## Scope

This benchmark records one bounded local three-request queue probe for
Ferrite's OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q6_K, with RSS
sampled after server readiness and after the queued requests complete.

This is a single-server memory sample for one prompt shape. It does not prove
general queue fairness under load, multi-client throughput, long-context memory
growth, leak-free steady state, or full Tier 1 memory posture.

## Tree State

- Branch: `main`
- Commit before run: `c15fad3`
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
- Ferrite model id for this run: `qwen2.5-1.5b-q6_k`

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k \
  --bind 127.0.0.1:18188 \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 300000
```

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URL: `http://127.0.0.1:18188`
- Inference wait window: 300,000 ms
- Holder request:
  - route: `POST /v1/chat/completions`
  - prompt: `hello world`
  - generation limit: 4 tokens
  - streaming: true
- Queued requests:
  - route: `POST /v1/completions`
  - prompt: `hello world`
  - generation limit: 1 token
  - streaming: false
- Launch sequence:
  - start holder stream
  - wait about 50 ms
  - start `queued_one`
  - wait about 20 ms
  - start `queued_two`

One discarded wrapper attempt completed the HTTP bodies but exited while parsing
curl metadata before the post-probe RSS sample. The retained run below used
fixed parsing and produced complete request, RSS, and response evidence.

## Results

| Request | Start ms | Finish ms | Curl elapsed s | HTTP | Stream DONE markers |
| --- | ---: | ---: | ---: | ---: | ---: |
| `holder_stream` | 1,782,698,128,181 | 1,782,698,131,460 | 3.241283 | 200 | 1 |
| `queued_one` | 1,782,698,128,242 | 1,782,698,132,286 | 4.006308 | 200 | 0 |
| `queued_two` | 1,782,698,128,274 | 1,782,698,133,075 | 4.764104 | 200 | 0 |

Finish order matched start order for the queued completion requests:

```text
holder_stream -> queued_one -> queued_two
```

RSS samples:

```text
rss_after_health_kib=1463616
rss_after_health_bytes=1498742784
rss_after_probe_kib=1497248
rss_after_probe_bytes=1533181952
```

## Response Checks

The holder stream returned HTTP `200`, emitted four content chunks, emitted a
final stop chunk, then emitted `data: [DONE]`.

Content chunks:

```text
你好
，
世界
！
```

Both queued completions returned HTTP `200` with:

```text
object=text_completion
model=qwen2.5-1.5b-q6_k
text=\n
prompt_tokens=2
completion_tokens=1
total_tokens=3
```

## Interpretation

This probe shows a real Qwen2.5-1.5B Q6_K streaming chat holder and two queued
legacy completion requests all succeeding under a configured bounded wait
window while the server RSS stayed within the sampled local envelope.

This narrows the concurrent-server-memory evidence gap for Qwen2.5-1.5B Q6_K.
It is not a leak test, a long-running steady-state pass, a throughput pass, or
a complete fairness proof.
