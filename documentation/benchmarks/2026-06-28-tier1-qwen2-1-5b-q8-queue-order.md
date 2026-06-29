# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 Queue Order

## Scope

This benchmark records one bounded local queue-order probe for Ferrite's
OpenAI-compatible server running Qwen2.5-1.5B-Instruct Q8_0.

This is a single-server, three-request concurrency probe. It does not prove
general queue fairness under load, multi-client throughput, cancellation
behavior, or long-stream overlap across broad prompts.

## Tree State

- Branch: `main`
- Commit before run: `f6b92b3`
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
  --bind 127.0.0.1:18186 \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 180000
```

Health check:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0"}
```

## Protocol

- Host: local macOS aarch64
- Server: `target/release/ferrite-server`
- Base URL: `http://127.0.0.1:18186`
- Inference wait window: 180,000 ms
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

The retained probe used a `bash` wrapper for explicit request PID waiting. Two
discarded harness attempts failed before producing retained evidence: one zsh
cleanup glob had no matches, and one zsh PID-list wait treated the whole list as
one job. Both were harness issues, not server responses.

## Results

| Request | Start ms | Finish ms | Elapsed ms | HTTP | Stream DONE markers |
| --- | ---: | ---: | ---: | ---: | ---: |
| `holder_stream` | 1,782,695,415,599 | 1,782,695,416,733 | 1,092.417 | 200 | 1 |
| `queued_one` | 1,782,695,415,660 | 1,782,695,417,034 | 1,336.813 | 200 | 0 |
| `queued_two` | 1,782,695,415,685 | 1,782,695,417,327 | 1,602.915 | 200 | 0 |

Finish order matched start order for the queued completion requests:

```text
holder_stream -> queued_one -> queued_two
```

RSS samples:

```text
rss_after_health_kib=1882560
rss_after_health_bytes=1927741440
rss_after_probe_kib=1909872
rss_after_probe_bytes=1955708928
```

## Response Checks

The holder stream returned HTTP `200`, emitted four content chunks, then emitted
`data: [DONE]`.

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
model=qwen2.5-1.5b-q8_0
text=\n
prompt_tokens=2
completion_tokens=1
total_tokens=3
```

## Interpretation

This probe shows a real Qwen2.5-1.5B Q8_0 holder stream and two queued legacy
completion requests all succeeding under a configured bounded wait window. The
two queued completions finished in request-start order in this run.

This narrows the Tier 1 successful-concurrent-serving evidence gap beyond the
previous two-request overlap proof. It is not a throughput pass or a complete
fairness proof: it covers one local model, one holder stream, two queued
requests, one prompt, and one run.
