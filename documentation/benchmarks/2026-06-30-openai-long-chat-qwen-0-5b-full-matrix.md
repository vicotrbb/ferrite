# OpenAI Long-Chat Qwen 0.5B Full Matrix

## Scope

This is the first full single-model long-chat gate pass for the required
256/512/1024-token streaming response lengths. It uses the OpenAI-compatible
HTTP server, repeated multi-turn chat shape, per-token streaming latency, usage
validation, finish reason capture, and server RSS sampling.

This proves the full token-length matrix for one local model only. It does not
prove the agreed multi-model gate yet.

## Environment

- Date: 2026-06-30
- Commit: `654c75d`
- Host: local macOS development machine
- Server port: `127.0.0.1:18095`
- Server PID for RSS sampling: `33958`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18095 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --addr 127.0.0.1:18095 \
  --api-key local-secret \
  --rss-pid 33958 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

Planned scenarios:

- Models: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

## Results

All twelve streaming chat scenarios completed with `finish_reason=length`.
Usage completion tokens matched the requested token length for every scenario.

| Turn | Max tokens | Completed | Finish | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 16341 | 14254 | 17.959314 | 2067 | 47 | 55 | 2162688 | 421871616 | 421527552 |
| 1 | 512 | 1 | length | 32423 | 30304 | 16.895367 | 1904 | 55 | 69 | 421527552 | 410517504 | 408502272 |
| 1 | 1024 | 1 | length | 74742 | 72561 | 14.112081 | 1942 | 68 | 95 | 408502272 | 409649152 | 409649152 |
| 2 | 256 | 1 | length | 16154 | 14071 | 18.192188 | 1903 | 47 | 55 | 409649152 | 424099840 | 423903232 |
| 2 | 512 | 1 | length | 32178 | 30059 | 17.032855 | 1971 | 55 | 68 | 423903232 | 409239552 | 409223168 |
| 2 | 1024 | 1 | length | 75190 | 73011 | 14.025240 | 1940 | 69 | 96 | 409223168 | 410681344 | 410681344 |
| 3 | 256 | 1 | length | 16520 | 14438 | 17.730965 | 1999 | 48 | 57 | 410681344 | 423968768 | 423723008 |
| 3 | 512 | 1 | length | 32183 | 30069 | 17.027112 | 1996 | 54 | 68 | 423723008 | 410648576 | 410648576 |
| 3 | 1024 | 1 | length | 75641 | 73466 | 13.938386 | 1983 | 69 | 96 | 410648576 | 409583616 | 408846336 |
| 4 | 256 | 1 | length | 16204 | 14120 | 18.129582 | 1939 | 47 | 55 | 408846336 | 432357376 | 432357376 |
| 4 | 512 | 1 | length | 32341 | 30223 | 16.940490 | 1923 | 55 | 68 | 432357376 | 411516928 | 411484160 |
| 4 | 1024 | 1 | length | 75428 | 73249 | 13.979648 | 1917 | 69 | 96 | 411484160 | 410615808 | 410615808 |

Usage was stable by token length:

- `256`: prompt tokens `47`, completion tokens `256`, total tokens `303`.
- `512`: prompt tokens `47`, completion tokens `512`, total tokens `559`.
- `1024`: prompt tokens `47`, completion tokens `1024`, total tokens `1071`.

After stopping the server, `lsof -nP -iTCP:18095 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite completed the required long-chat token-length matrix for one local
Qwen2.5 0.5B model through the OpenAI-compatible HTTP server. The run captured
the evidence required for this slice: repeated multi-turn shape, 256/512/1024
streaming completions, finish reason, usage totals, per-token latency, time to
first token, throughput, and RSS before/after/idle.

Observed throughput declined as expected with longer generations:

- 256-token scenarios: about `17.73` to `18.19` tok/s.
- 512-token scenarios: about `16.90` to `17.03` tok/s.
- 1024-token scenarios: about `13.94` to `14.11` tok/s.

Remaining proof gaps:

- Repeat the full matrix across the agreed model set.
- Combine this full matrix with explicit stop assertions.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Include disconnect/error probes in a broader long-chat proof run, not only in
  separate smoke probes.
