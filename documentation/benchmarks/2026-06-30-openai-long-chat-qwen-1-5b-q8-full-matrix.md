# OpenAI Long-Chat Qwen 1.5B Q8 Full Matrix

## Scope

This is a full single-model long-chat gate pass for Qwen2.5 1.5B Q8_0 using
the required 256/512/1024-token streaming response lengths. It uses the
OpenAI-compatible HTTP server, repeated multi-turn chat shape, per-token
streaming latency, usage validation, finish reason capture, and server RSS
sampling.

This is the third configured-model full matrix after Qwen2.5 0.5B Q4_K_M and
Qwen2.5 1.5B Q6_K. It does not complete the agreed multi-model gate yet.

## Environment

- Date: 2026-06-30
- Commit: `628506f`
- Host: local macOS development machine
- Server port: `127.0.0.1:18097`
- Server PID for RSS sampling: `48850`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18097 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --addr 127.0.0.1:18097 \
  --api-key local-secret \
  --rss-pid 48850 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

Planned scenarios:

- Models: `Qwen2.5-1.5B-Instruct-Q8_0`
- Token lengths: `256,512,1024`
- Turns: `4`
- Planned scenarios: `12`

## Results

All twelve streaming chat scenarios completed with `finish_reason=length`.
Usage completion tokens matched the requested token length for every scenario,
and streaming token event counts matched completion token counts.

| Turn | Max tokens | Completed | Finish | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 32975 | 30828 | 8.304122 | 4204 | 104 | 118 | 1697464320 | 1677099008 | 1675952128 |
| 1 | 512 | 1 | length | 67763 | 65564 | 7.809083 | 4139 | 119 | 149 | 1675952128 | 1683963904 | 1682898944 |
| 1 | 1024 | 1 | length | 162445 | 160141 | 6.394345 | 4350 | 150 | 209 | 1682898944 | 1714356224 | 1714356224 |
| 2 | 256 | 1 | length | 33421 | 31279 | 8.184355 | 4146 | 105 | 121 | 1714356224 | 1693040640 | 1691648000 |
| 2 | 512 | 1 | length | 67782 | 65575 | 7.807744 | 4166 | 119 | 148 | 1691648000 | 1684226048 | 1683619840 |
| 2 | 1024 | 1 | length | 161917 | 159616 | 6.415369 | 4131 | 151 | 209 | 1683619840 | 1714225152 | 1713848320 |
| 3 | 256 | 1 | length | 33039 | 30890 | 8.287432 | 4147 | 103 | 118 | 1713848320 | 1687650304 | 1672986624 |
| 3 | 512 | 1 | length | 67858 | 65668 | 7.796716 | 4169 | 119 | 148 | 1672986624 | 1684013056 | 1680359424 |
| 3 | 1024 | 1 | length | 161735 | 159430 | 6.422879 | 4202 | 151 | 209 | 1680359424 | 1714585600 | 1714585600 |
| 4 | 256 | 1 | length | 32866 | 30718 | 8.333830 | 4098 | 104 | 118 | 1714585600 | 1698955264 | 1694433280 |
| 4 | 512 | 1 | length | 68141 | 65944 | 7.764096 | 4434 | 120 | 149 | 1694433280 | 1683718144 | 1683603456 |
| 4 | 1024 | 1 | length | 161944 | 159642 | 6.414346 | 4377 | 150 | 209 | 1683603456 | 1714634752 | 1714634752 |

Usage was stable by token length:

- `256`: prompt tokens `47`, completion tokens `256`, total tokens `303`.
- `512`: prompt tokens `47`, completion tokens `512`, total tokens `559`.
- `1024`: prompt tokens `47`, completion tokens `1024`, total tokens `1071`.

After stopping the server, `lsof -nP -iTCP:18097 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite completed the required long-chat token-length matrix for the local
Qwen2.5 1.5B Q8_0 model through the OpenAI-compatible HTTP server. The
incremental result output worked as intended: each scenario result was written
to the proof log as it completed, which avoided losing evidence during a long
CPU-bound run.

Observed throughput:

- 256-token scenarios: about `8.18` to `8.33` tok/s.
- 512-token scenarios: about `7.76` to `7.81` tok/s.
- 1024-token scenarios: about `6.39` to `6.42` tok/s.

RSS after load and during the matrix stayed around `1.68` to `1.71` GB.

Remaining proof gaps:

- Repeat the full matrix for SmolLM2 1.7B Q4_K_M.
- Combine full matrix runs with explicit stop assertions.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Include disconnect/error probes in a broader long-chat proof run, not only in
  separate smoke probes.
