# Benchmark: Local Qwen 0.5B Natural EOS Probe 64

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Attempt to prove natural tokenizer EOS behavior without an explicit stop
sequence. The probe required `finish_reason=stop` for a small deterministic
prompt and intentionally failed if the model continued to the token cap.

## Result

Status: Failed as expected for this prompt shape.

The gate stopped on turn 1 with:

```text
long_chat_run_error=expected finish_reason stop, got length
```

Both exit files contained `1`, and the server lifecycle log shows the request
generated the full 64 token IDs. This is negative evidence: explicit stop
behavior is proven separately, but natural EOS behavior is not proven by this
prompt.

## Environment

- Ferrite runtime code commit: `8c1cc4f`
- Host: local macOS workspace
- Server: `127.0.0.1:18236`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/`
- Server binary SHA256:
  `dec0167a646244de6392efbfe5b1549c4064dbab729de894aaa87c02c988b473`
- Gate binary SHA256:
  `7a953e710de9210b2832d61fa55dc89a8f835d5207a7e18659d9f9480ab03e97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18236`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18236 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --addr 127.0.0.1:18236 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --prompt 'Answer exactly: OK.' \
  --assistant-context 'OK' \
  --follow-up 'Answer exactly: OK.' \
  --expect-finish-reason stop \
  --token-lengths 64 \
  --require-token-lengths 64 \
  --turns 4 \
  --rss-pid <server-pid> \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/long-chat.exit
```

No explicit `--stop` sequence was configured.

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/long-chat.log` | 17 | `9ddef70d04fc74cb0c02b8cc3358558eac4255ff65b8e8364009bf0cceefc81e` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/gate.stdout` | 16 | `951f8d7b7c90c453338146bb30cfc0196be589cb72053c072395242ea256217a` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/gate.stderr` | 1 | `778034d3b0083f381e8f809b8ccb43e7eb7639f6ff0323b509309f2c9113f3b1` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/server.log` | 1 | `a08802a3235f90da22403ee9e78c0afaed1f5d48712f0291b14fcf263c6e104a` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/health.json` | 0 | `e3284eada962df1c75177574e65d3c528a2dcc0fb990143e5877c096413857b4` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/long-chat.exit` | 1 | `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865` |
| `target/proof/local-qwen05-natural-eos-probe-64-2026-07-03/gate-command.exit` | 1 | `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865` |

Both exit-code files contained `1`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Observed Failure

```text
long_chat_expected_finish_reason=stop
long_chat_required_token_lengths=64
long_chat_planned_scenarios=4
long_chat_scenario=model:qwen2.5-0.5b-q4_k_m,turn:1,max_tokens:64
long_chat_scenario=model:qwen2.5-0.5b-q4_k_m,turn:2,max_tokens:64
long_chat_scenario=model:qwen2.5-0.5b-q4_k_m,turn:3,max_tokens:64
long_chat_scenario=model:qwen2.5-0.5b-q4_k_m,turn:4,max_tokens:64
long_chat_run_error=expected finish_reason stop, got length
```

Server lifecycle for the failed request:

```text
generated_chunks=64 generated_token_ids=64
```

## Interpretation

This invalidates the idea that the simple `Answer exactly: OK.` prompt is a
reliable natural-EOS proof for local Qwen2.5-0.5B. It does not prove EOS is
broken; it proves this prompt shape does not produce EOS before the token cap.

Explicit stop behavior remains proven by:

`documentation/benchmarks/2026-07-03-local-qwen-0-5b-stop-probe-64.md`

Natural EOS behavior remains open and needs either a better prompt fixture, a
tokenizer-aware EOS harness, or a model-specific generation fixture that can
demonstrate EOS without relying on natural-language instruction following.

## Next Step

Add a deterministic EOS fixture or harness mode that can distinguish tokenizer
EOS from explicit stop strings and length termination.
