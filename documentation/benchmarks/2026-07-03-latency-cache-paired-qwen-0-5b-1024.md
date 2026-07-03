# Benchmark: Paired Latency Cache Qwen 0.5B 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the third bounded paired measurement from the latency/cache companion
protocol:

- Ferrite's long-chat gate provides correctness, generated-context identity,
  cache metadata, reconnect probes, and RSS samples.
- `llama-benchy` provides an external OpenAI-compatible client-side
  prefix-cache latency view.

This completes the local 256/512/1024 paired smoke ladder for Qwen 0.5B. It is
still local macOS evidence, not x86_64 evidence.

## Environment

- Ferrite commit: `afec12a`
- Host: local macOS workspace
- OS: macOS 14.5 (`23F79`)
- Architecture: `arm64`
- Memory: `17179869184` bytes
- Server bind: `127.0.0.1:18214`
- Server PID for RSS sampling: `71103`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Raw proof directory:
  `target/proof/local-paired-latency-cache-1024-2026-07-03/`

The local server was stopped after the paired run. A final bind-specific
listener check found no process listening on port `18214`.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were built from the current tree:

```sh
cargo build -p ferrite-server --release --bins
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.24s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18214 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Ferrite Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18214 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 71103 \
  --prompt-cache-key ferrite:paired:qwen05:latency-cache-1024 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-paired-latency-cache-1024-2026-07-03/ferrite-long-chat-1024.log \
  --proof-exit-code target/proof/local-paired-latency-cache-1024-2026-07-03/ferrite-long-chat-1024.exit
```

The gate exited `0` and wrote 210 log lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | 1670 | 14.902722 | 408256512 |
| 2 | 1054 | 12 | `shared_prefix_hit` | 66593 | 5.403135 | 412286976 |
| 3 | 1054 | 16 | `shared_prefix_hit` | 66664 | 5.390572 | 420855808 |
| 4 | 1054 | 1054 | `exact_hit` | 230 | 8.884296 | 415547392 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=1024
long_chat_result_streaming_token_id_chunks=1024
long_chat_result_streaming_token_ids=1024
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

Summary fields included:

```text
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_run_complete=true
```

## llama-benchy Companion

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18214/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 1024 \
  --tg 1024 \
  --depth 1024 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:qwen05:benchy-1024 \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-1024.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-1024.json`

Captured stdout:
`target/proof/local-paired-latency-cache-1024-2026-07-03/llama-benchy-1024.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 1024 | 1024 | 1024 | 1 | 8.403341 | 0.937167 | 0.0 | 65689.654583 |
| inference | 1024 | 1024 | 1024 | 1 | 5.991473 | 2.872042 | 1.296903 | 114978.518583 |

## Interpretation

The 1024-token paired run reproduces the generated-context fixed-point
mechanism inside the paired protocol:

- Ferrite's long-chat gate proved generated assistant context was carried
  across turns and that all generated-context identity links matched previous
  responses.
- Turns 2 and 3 were shallow shared-prefix hits: `12 / 1054` and `16 / 1054`
  cached prompt tokens, with TTFT around 66 seconds.
- Turn 3 generated the same response identity as its own assistant context.
- Turn 4 reused the full prompt: `1054 / 1054` cached prompt tokens,
  `lookup=exact_hit`, and TTFT collapsed to `230` ms.
- `llama-benchy` successfully exercised the different OpenAI-compatible
  system-context prefix-cache shape at depth 1024, prompt 1024, and generation
  1024.
- The external companion run produced portable JSON, but it did not expose
  Ferrite's cached-token metadata or generated-context identity fields.

This completes the local 256/512/1024 paired smoke ladder. The results support
the protocol split: Ferrite's gate explains cache behavior and correctness;
`llama-benchy` provides external latency trend data.

## Limits

This run does not prove:

- x86_64 paired behavior;
- high-concurrency behavior;
- stop/EOS behavior;
- long-running RSS stability;
- that `llama-benchy` can replace Ferrite's long-chat gate.
