# Benchmark: Paired Latency Cache Qwen 0.5B 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the first bounded paired measurement from the latency/cache companion
protocol:

- Ferrite's long-chat gate provides correctness, generated-context identity,
  cache metadata, reconnect probes, and RSS samples.
- `llama-benchy` provides an external OpenAI-compatible client-side
  prefix-cache latency view.

This is a 256-token local smoke of the paired protocol, not a full 256/512/1024
matrix.

## Environment

- Ferrite commit: `af5d619`
- Host: local macOS workspace
- OS: macOS 14.5 (`23F79`)
- Architecture: `arm64`
- Memory: `17179869184` bytes
- Server bind: `127.0.0.1:18212`
- Server PID for RSS sampling: `66855`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Raw proof directory:
  `target/proof/local-paired-latency-cache-256-2026-07-03/`

The local server was stopped after the paired run. A final bind-specific
listener check found no process listening on port `18212`.

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
Finished `release` profile [optimized] target(s) in 0.28s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18212 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
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
  --addr 127.0.0.1:18212 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256 \
  --turns 4 \
  --probe-max-tokens 256 \
  --rss-pid 66855 \
  --prompt-cache-key ferrite:paired:qwen05:latency-cache-256 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-paired-latency-cache-256-2026-07-03/ferrite-long-chat-256.log \
  --proof-exit-code target/proof/local-paired-latency-cache-256-2026-07-03/ferrite-long-chat-256.exit
```

The gate exited `0` and wrote 210 log lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | 1728 | 19.214389 | 428441600 |
| 2 | 286 | 12 | `shared_prefix_hit` | 12144 | 9.504617 | 424312832 |
| 3 | 286 | 14 | `shared_prefix_hit` | 12002 | 9.612208 | 433225728 |
| 4 | 286 | 14 | `shared_prefix_hit` | 12130 | 9.463943 | 427048960 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_id_chunks=256
long_chat_result_streaming_token_ids=256
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
  --base-url http://127.0.0.1:18212/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 256 \
  --tg 256 \
  --depth 256 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:qwen05:benchy-256 \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-256.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-256.json`

Captured stdout:
`target/proof/local-paired-latency-cache-256-2026-07-03/llama-benchy-256.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 256 | 256 | 256 | 1 | 17.854485 | 0.945333 | 0.0 | 11551.662375 |
| inference | 256 | 256 | 256 | 1 | 14.800289 | 1.043833 | 0.0 | 14724.025291 |

## Interpretation

The paired run confirms that the two tools are complementary:

- Ferrite's long-chat gate proved generated assistant context was carried
  across turns and that all generated-context identity links matched previous
  responses.
- The 256-token generated-context lane did not converge to an exact prompt
  fixed point. Follow-up turns reused only 12 to 14 prompt tokens and remained
  `shared_prefix_hit`.
- TTFT for the generated follow-up turns stayed around 12 seconds, matching the
  shallow-cache interpretation.
- `llama-benchy` successfully exercised a different OpenAI-compatible
  prefix-cache shape: a system-message context-load phase followed by an
  inference phase at depth 256 and prompt 256.
- The external companion run produced portable JSON, but it did not expose
  Ferrite's cached-token metadata or generated-context identity fields.

This supports the protocol decision: use Ferrite's gate for correctness and
cache explanation, then use `llama-benchy` for repeatable external latency
trend data.

## Limits

This run does not prove:

- 512 or 1024-token paired behavior;
- x86_64 paired behavior;
- high-concurrency behavior;
- stop/EOS behavior;
- long-running RSS stability;
- generated-context exact-hit behavior at 256 tokens.
