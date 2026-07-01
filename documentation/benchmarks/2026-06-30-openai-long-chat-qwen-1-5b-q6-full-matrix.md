# OpenAI Long-Chat Qwen 1.5B Q6 Full Matrix

## Scope

This is a full single-model long-chat gate pass for Qwen2.5 1.5B Q6_K using
the required 256/512/1024-token streaming response lengths. It uses the
OpenAI-compatible HTTP server, repeated multi-turn chat shape, per-token
streaming latency, usage validation, finish reason capture, and server RSS
sampling.

This is the second configured-model full matrix after Qwen2.5 0.5B Q4_K_M. It
does not complete the agreed multi-model gate yet.

## Environment

- Date: 2026-06-30
- Commit: `1337c8b`
- Host: local macOS development machine
- Server port: `127.0.0.1:18096`
- Server PID for RSS sampling: `38946`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18096 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models Qwen2.5-1.5B-Instruct-Q6_K \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --addr 127.0.0.1:18096 \
  --api-key local-secret \
  --rss-pid 38946 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

Planned scenarios:

- Models: `Qwen2.5-1.5B-Instruct-Q6_K`
- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

## Results

All twelve streaming chat scenarios completed with `finish_reason=length`.
Usage completion tokens matched the requested token length for every scenario.

| Turn | Max tokens | Completed | Finish | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 89974 | 87624 | 2.921542 | 13218 | 289 | 318 | 2260992 | 1504821248 | 1504821248 |
| 1 | 512 | 1 | length | 174748 | 172364 | 2.970453 | 13722 | 308 | 350 | 1504821248 | 1523826688 | 1523826688 |
| 1 | 1024 | 1 | length | 368875 | 366381 | 2.794898 | 12740 | 346 | 412 | 1523826688 | 1551073280 | 1551073280 |
| 2 | 256 | 1 | length | 91104 | 88775 | 2.883683 | 13686 | 291 | 325 | 1551073280 | 1500561408 | 1500561408 |
| 2 | 512 | 1 | length | 175737 | 173352 | 2.953512 | 12894 | 311 | 354 | 1500561408 | 1519665152 | 1519665152 |
| 2 | 1024 | 1 | length | 371690 | 369193 | 2.773613 | 13217 | 346 | 422 | 1519665152 | 1549402112 | 1549402112 |
| 3 | 256 | 1 | length | 91360 | 88999 | 2.876420 | 13197 | 293 | 320 | 1549402112 | 1503985664 | 1503985664 |
| 3 | 512 | 1 | length | 178390 | 175918 | 2.910446 | 14339 | 313 | 351 | 1503985664 | 1517469696 | 1517469696 |
| 3 | 1024 | 1 | length | 372909 | 370430 | 2.764351 | 13910 | 347 | 419 | 1517469696 | 1552121856 | 1552121856 |
| 4 | 256 | 1 | length | 90457 | 88129 | 2.904805 | 12851 | 290 | 333 | 1552121856 | 1502986240 | 1502986240 |
| 4 | 512 | 1 | length | 176743 | 174336 | 2.936848 | 13045 | 309 | 366 | 1502986240 | 1517502464 | 1517502464 |
| 4 | 1024 | 1 | length | 371627 | 369140 | 2.774011 | 13227 | 346 | 411 | 1517502464 | 1545551872 | 1545551872 |

Usage was stable by token length:

- `256`: prompt tokens `47`, completion tokens `256`, total tokens `303`.
- `512`: prompt tokens `47`, completion tokens `512`, total tokens `559`.
- `1024`: prompt tokens `47`, completion tokens `1024`, total tokens `1071`.

After stopping the server, `lsof -nP -iTCP:18096 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite completed the required long-chat token-length matrix for the local
Qwen2.5 1.5B Q6_K model through the OpenAI-compatible HTTP server. The
incremental result output added before this run worked as intended: each
scenario result was printed and flushed as it completed.

Observed throughput:

- 256-token scenarios: about `2.88` to `2.92` tok/s.
- 512-token scenarios: about `2.91` to `2.97` tok/s.
- 1024-token scenarios: about `2.76` to `2.79` tok/s.

RSS after load and during the matrix stayed around `1.50` to `1.55` GB.

Remaining proof gaps:

- Repeat the full matrix for Qwen2.5 1.5B Q8_0.
- Repeat the full matrix for SmolLM2 1.7B Q4_K_M.
- Combine full matrix runs with explicit stop assertions.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Include disconnect/error probes in a broader long-chat proof run, not only in
  separate smoke probes.
