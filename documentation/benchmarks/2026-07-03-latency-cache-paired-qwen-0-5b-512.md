# Benchmark: Paired Latency Cache Qwen 0.5B 512

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the second bounded paired measurement from the latency/cache companion
protocol:

- Ferrite's long-chat gate provides correctness, generated-context identity,
  cache metadata, reconnect probes, and RSS samples.
- `llama-benchy` provides an external OpenAI-compatible client-side
  prefix-cache latency view.

This is a 512-token local smoke of the paired protocol, extending the 256-token
paired run. It is not a full 256/512/1024 matrix.

## Environment

- Ferrite commit: `844154c`
- Host: local macOS workspace
- OS: macOS 14.5 (`23F79`)
- Architecture: `arm64`
- Memory: `17179869184` bytes
- Server bind: `127.0.0.1:18213`
- Server PID for RSS sampling: `68623`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Raw proof directory:
  `target/proof/local-paired-latency-cache-512-2026-07-03/`

The local server was stopped after the paired run. A final bind-specific
listener check found no process listening on port `18213`.

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
Finished `release` profile [optimized] target(s) in 0.23s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18213 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024 \
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
  --addr 127.0.0.1:18213 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 512 \
  --turns 4 \
  --probe-max-tokens 512 \
  --rss-pid 68623 \
  --prompt-cache-key ferrite:paired:qwen05:latency-cache-512 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-paired-latency-cache-512-2026-07-03/ferrite-long-chat-512.log \
  --proof-exit-code target/proof/local-paired-latency-cache-512-2026-07-03/ferrite-long-chat-512.exit
```

The gate exited `0` and wrote 210 log lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=512
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | 1687 | 18.068418 | 421871616 |
| 2 | 542 | 12 | `shared_prefix_hit` | 27349 | 7.606032 | 414564352 |
| 3 | 542 | 306 | `shared_prefix_hit` | 13743 | 9.703722 | 417660928 |
| 4 | 542 | 20 | `shared_prefix_hit` | 26745 | 7.699689 | 420167680 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=512
long_chat_result_streaming_token_id_chunks=512
long_chat_result_streaming_token_ids=512
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
  --base-url http://127.0.0.1:18213/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 512 \
  --tg 512 \
  --depth 512 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:qwen05:benchy-512 \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-512.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-512.json`

Captured stdout:
`target/proof/local-paired-latency-cache-512-2026-07-03/llama-benchy-512.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 512 | 512 | 512 | 1 | 12.861305 | 0.937958 | 0.0 | 26284.783375 |
| inference | 512 | 512 | 512 | 1 | 9.919268 | 2.804417 | 1.164084 | 38107.459042 |

## Interpretation

The 512-token paired run adds a stronger cache-depth signal than the 256-token
smoke:

- Ferrite's long-chat gate again proved generated assistant context was carried
  across turns and that all generated-context identity links matched previous
  responses.
- The generated-context lane did not converge to an exact prompt fixed point.
  All follow-up turns remained `shared_prefix_hit`.
- Turn 3 reused a much deeper shared prefix (`306 / 542`) than turns 2 and 4,
  and its TTFT dropped to `13743` ms. Turns 2 and 4 reused only `12 / 542` and
  `20 / 542`, with TTFT around 27 seconds.
- `llama-benchy` successfully exercised the different OpenAI-compatible
  system-context prefix-cache shape at depth 512, prompt 512, and generation
  512.
- The external companion run produced portable JSON, but it did not expose
  Ferrite's cached-token metadata or generated-context identity fields.

This reinforces the protocol decision: cache interpretation still comes from
Ferrite's gate, while `llama-benchy` supplies external latency trend data.

## Limits

This run does not prove:

- 1024-token paired behavior;
- x86_64 paired behavior;
- high-concurrency behavior;
- stop/EOS behavior;
- long-running RSS stability;
- generated-context exact-hit behavior at 512 tokens.
