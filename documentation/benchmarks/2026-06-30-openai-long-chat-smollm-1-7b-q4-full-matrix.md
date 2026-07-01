# OpenAI Long-Chat SmolLM2 1.7B Q4 Full Matrix

## Scope

This is a full single-model long-chat gate pass for SmolLM2 1.7B Q4_K_M using
the required 256/512/1024-token streaming response lengths. It uses the
OpenAI-compatible HTTP server, repeated multi-turn chat shape, per-token
streaming latency, usage validation, finish reason capture, and server RSS
sampling.

This completes the initially configured four-model long-chat matrix alongside
Qwen2.5 0.5B Q4_K_M, Qwen2.5 1.5B Q6_K, and Qwen2.5 1.5B Q8_0.

## Environment

- Date: 2026-06-30
- Commit: `1623795`
- Host: local macOS development machine
- Server port: `127.0.0.1:18098`
- Server PID for RSS sampling: `54184`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18098 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --addr 127.0.0.1:18098 \
  --api-key local-secret \
  --rss-pid 54184 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

Planned scenarios:

- Models: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

## Results

All twelve streaming chat scenarios completed with `finish_reason=length`.
Usage completion tokens matched the requested token length for every scenario,
and streaming token event counts matched completion token counts.

| Turn | Max tokens | Completed | Finish | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 62618 | 60353 | 4.241650 | 9525 | 197 | 226 | 772636672 | 1145962496 | 1145962496 |
| 1 | 512 | 1 | length | 127451 | 125063 | 4.093925 | 9177 | 220 | 289 | 1145962496 | 1188478976 | 1188478976 |
| 1 | 1024 | 1 | length | 327033 | 324444 | 3.156161 | 9483 | 300 | 448 | 1188478976 | 1234747392 | 1234747392 |
| 2 | 256 | 1 | length | 62549 | 60286 | 4.246402 | 9263 | 198 | 223 | 1234747392 | 1190248448 | 1190248448 |
| 2 | 512 | 1 | length | 141438 | 139077 | 3.681388 | 9566 | 245 | 345 | 1190248448 | 1278410752 | 1278410752 |
| 2 | 1024 | 1 | length | 365234 | 362613 | 2.823943 | 10159 | 325 | 535 | 1278410752 | 1344503808 | 1344503808 |
| 3 | 256 | 1 | length | 101416 | 98960 | 2.586886 | 11399 | 304 | 589 | 1344503808 | 1186873344 | 340049920 |
| 3 | 512 | 1 | length | 162871 | 160473 | 3.190562 | 15609 | 262 | 412 | 318603264 | 1290862592 | 1290862592 |
| 3 | 1024 | 1 | length | 359066 | 356450 | 2.872766 | 10811 | 320 | 535 | 1290862592 | 1424703488 | 1424703488 |
| 4 | 256 | 1 | length | 73099 | 70845 | 3.613515 | 12486 | 216 | 294 | 1424703488 | 1172357120 | 1172357120 |
| 4 | 512 | 1 | length | 141338 | 138972 | 3.684182 | 10314 | 239 | 336 | 1172357120 | 1289633792 | 1289601024 |
| 4 | 1024 | 1 | length | 348815 | 346213 | 2.957708 | 10383 | 326 | 460 | 1289601024 | 1459617792 | 1459617792 |

Usage was stable by token length:

- `256`: prompt tokens `53`, completion tokens `256`, total tokens `309`.
- `512`: prompt tokens `53`, completion tokens `512`, total tokens `565`.
- `1024`: prompt tokens `53`, completion tokens `1024`, total tokens `1077`.

After stopping the server, `lsof -nP -iTCP:18098 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite completed the required long-chat token-length matrix for the local
SmolLM2 1.7B Q4_K_M model through the OpenAI-compatible HTTP server. SmolLM2
was materially slower than the Qwen2.5 1.5B Q8_0 run on this workload, but all
scenarios reached a valid final streaming finish event.

Observed throughput:

- 256-token scenarios: about `2.59` to `4.25` tok/s.
- 512-token scenarios: about `3.19` to `4.09` tok/s.
- 1024-token scenarios: about `2.82` to `3.16` tok/s.

Most RSS samples after load stayed around `1.15` to `1.46` GB. The turn 3
boundary included a sampler discontinuity where idle RSS dropped to about
`340 MB` before the next request reloaded to the expected range. This is
recorded as observed behavior and should be rechecked if SmolLM2 memory
stability becomes a release claim.

Remaining proof gaps:

- Combine full matrix runs with explicit stop assertions.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Include disconnect/error probes in a broader long-chat proof run, not only in
  separate smoke probes.
