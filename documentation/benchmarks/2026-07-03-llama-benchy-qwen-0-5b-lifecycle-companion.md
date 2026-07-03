# Benchmark: llama-benchy Qwen 0.5B Lifecycle Companion

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run an external `llama-benchy` companion against the same local Qwen 0.5B
OpenAI-compatible server shape used by the lifecycle-instrumented long-chat
proof.

This benchmark does not replace Ferrite's long-chat gate. It gives portable
client-side latency JSON for `256`, `512`, and `1024` prompt/depth/generation
points while the Ferrite server records lifecycle summaries for every stream.

## Environment

- Ferrite commit: `0703a3e`
- Host: local macOS workspace
- Server: `127.0.0.1:18205`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>
- Server binary SHA256:
  `9e6458f6ca175e830b253ef77e3d8205195f5597c3d6543ddc7c3e82f9061198`
- Long-chat gate binary SHA256:
  `92273a007b95a2f71d89cc69cf88dc66f11728e90279f112e89072aebd98de70`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Proof directory:
  `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/`

The local server was stopped after the run. A final bind-specific process check
returned no listener on `127.0.0.1:18205`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18205 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Benchmark Commands

Each token point was run as a single-point command to avoid the expensive
`pp/tg/depth` cross-product:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18205/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name qwen2.5-0.5b-q4_k_m \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp <256|512|1024> \
  --tg <256|512|1024> \
  --depth <256|512|1024> \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:lifecycle:benchy:<tokens> \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-<tokens>.json
```

The `256` command saved valid JSON, then the shell wrapper failed after the
benchmark because it assigned to zsh's read-only `status` variable. The wrapper
was corrected for `512` and `1024`. The proof directory records normalized
exit files of `0` for all three saved benchmark JSON files, and the `256`
stdout still preserves the wrapper mistake.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/llama-benchy-256.stdout` | 21 lines | `95d526420840c23e4c8cc7cc1b9a09146a683d72248ded6ab465fac7887b6193` |
| `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/llama-benchy-512.stdout` | 21 lines | `1096304f03ed3ea2ce00bfda221f13baddedfeb9365ce265d388c57193438abf` |
| `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/llama-benchy-1024.stdout` | 22 lines | `5c29182fe79d2caffcc2e2c2881323948eef742e0ca6b119897314b02ed2f743` |
| `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/server.log` | 15 lines | `57cf4e177f739554a60b68859c3bd84588b3694d0df982514745364ddd0344f8` |
| `target/proof/local-qwen05-lifecycle-llama-benchy-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-256.json` | 3015 bytes | `741496d58210a24de1b59dbd384bd2b9db0253516540ad4654c700440e2aab02` |
| `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-512.json` | 3009 bytes | `3f9706aea96a88a5649870481168aab7b61a303655c4fdc6a4252f5e3f81ff2d` |
| `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-1024.json` | 3017 bytes | `746e9f3092b0a9390e8ddf70d79fc2e5cd6e1fab6060217cf982b7cc4bb35267` |

All three exit files contain `0` and have SHA256
`9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.

## llama-benchy Results

| Phase | Depth | Prompt | Generated | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 256 | 256 | 256 | 17.355062 | 0.930083 | 0.0 | 11682.690542 |
| inference | 256 | 256 | 256 | 14.184121 | 2.877000 | 1.326097 | 14931.817000 |
| context | 512 | 512 | 512 | 12.855879 | 1.087500 | 0.0 | 26427.955375 |
| inference | 512 | 512 | 512 | 9.661417 | 2.227375 | 0.365472 | 39347.656125 |
| context | 1024 | 1024 | 1024 | 8.281224 | 1.184041 | 0.0 | 66191.389000 |
| inference | 1024 | 1024 | 1024 | 5.867250 | 2.904084 | 1.483223 | 117547.157792 |

## Ferrite Lifecycle Results

The server emitted 15 lifecycle lines:

| Stream | Generated tokens | Prompt tokens started | Elapsed ms |
| ---: | ---: | ---: | ---: |
| 3 | 256 | 267 | 26442 |
| 4 | 256 | 261 | 33064 |
| 8 | 512 | 522 | 66273 |
| 9 | 512 | 517 | 92368 |
| 13 | 1024 | 1033 | 189889 |
| 14 | 1024 | 1027 | 292133 |

The remaining lifecycle lines were `llama-benchy` one-token latency probes. No
stream reported a client disconnect; all lifecycle summaries had
`finish_reason=completed` and `disconnect_point=none`.

## Interpretation

The external client trend matches the expected scale curve: larger
prompt/depth/generation points lower generation throughput and increase
end-to-end first-token time. The Ferrite lifecycle log adds server-side
evidence that the long `llama-benchy` requests completed rather than being
silently dropped or cancelled.

This strengthens the companion-protocol theory: `llama-benchy` is useful for
portable latency trend data, while Ferrite's own long-chat gate remains the
source of truth for generated-context identity, cached-token metadata,
reconnect/error probes, RSS sampling, and stop/EOS behavior.

## Limits

This run does not prove:

- x86_64 behavior for the lifecycle-instrumented server;
- concurrency behavior;
- stop/EOS behavior;
- client reconnect or error behavior;
- generated-context correctness;
- RSS stability beyond the server process snapshot taken during execution.
