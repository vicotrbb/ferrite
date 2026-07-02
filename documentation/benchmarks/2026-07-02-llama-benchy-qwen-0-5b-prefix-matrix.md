# Benchmark: llama-benchy Qwen 0.5B Prefix-Cache Matrix

Date: 2026-07-02

## Purpose

Run a bounded 256/512/1024 prefix-cache matrix against Ferrite's
OpenAI-compatible `/v1/chat/completions` server.

This extends the earlier 128-token prefix-cache smoke. It keeps generation
fixed at 32 tokens while varying prompt size and context depth.

## Environment

- Ferrite commit: `7e0e670c0b0dae788d7224f61ff1a9f5f8cff492`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server binary SHA256:
  `652393f177907ba1a01e7e72f9dcd131c5701da694117b6f07477bfb9aebfa35`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>

## Model

- Name: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Server

Command:

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --experimental-prefix-cache
```

## llama-benchy Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 256 512 1024 \
  --tg 32 \
  --depth 256 512 1024 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:benchy:prefix-matrix \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-matrix.json
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-matrix.json`

Captured stdout:
`target/proof/llama-benchy-qwen-0-5b-prefix-matrix.stdout.txt`

The command produced a 3x3 cross-product, not a diagonal-only matrix:

- prompt sizes: 256, 512, 1024;
- context depths: 256, 512, 1024;
- generation length: 32;
- phases per combination: context-load and inference.

## llama-benchy Results

| Depth | Prompt | Phase | TG tok/s | TTFR | E2E TTFT | Peak |
| ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 256 | 256 | context | 18.066042757242027 | 0.8973749936558306 | 12988.40979198576 | 19.0 |
| 256 | 256 | inference | 14.561603833875226 | 1.1492920166347176 | 28643.461125000613 | 15.0 |
| 256 | 512 | context | 18.123649335236333 | 2.3497919901274145 | 12560.56220899336 | 19.0 |
| 256 | 512 | inference | 12.182257774591958 | 1.0885000228881836 | 47225.098292023176 | 13.0 |
| 256 | 1024 | context | 18.3099520531956 | 2.3099579848349094 | 12364.619375002803 | 19.0 |
| 256 | 1024 | inference | 9.209281098667313 | 1.3435829896479845 | 95545.70587500348 | 10.0 |
| 512 | 256 | context | 14.623361178755554 | 2.8104170050937682 | 28342.90874999715 | 15.0 |
| 512 | 256 | inference | 12.218256489387398 | 1.1290419788565487 | 48056.71804200392 | 13.0 |
| 512 | 512 | context | 15.07705591712494 | 2.634417003719136 | 28509.484542009886 | 16.0 |
| 512 | 512 | inference | 10.499453349811796 | 1.818082993850112 | 68978.64845799631 | 11.0 |
| 512 | 1024 | context | 15.108515386408039 | 1.2569590180646628 | 27755.142417008756 | 16.0 |
| 512 | 1024 | inference | 8.136255241949678 | 2.5602090172469616 | 126363.71175001841 | 9.0 |
| 1024 | 256 | context | 10.672904233169225 | 3.107042022747919 | 69341.59850000287 | 11.0 |
| 1024 | 256 | inference | 9.123177424304494 | 2.41499999538064 | 95549.419707997 | 10.0 |
| 1024 | 512 | context | 10.61047783669317 | 2.8780840220861137 | 69378.98858400877 | 11.0 |
| 1024 | 512 | inference | 8.080062998468819 | 3.6987499915994704 | 125699.5397080027 | 9.0 |
| 1024 | 1024 | context | 10.615776840556968 | 2.591124997707084 | 69209.2213750002 | 11.0 |
| 1024 | 1024 | inference | 6.667126615243654 | 2.7822499978356063 | 196352.9143340129 | 7.0 |

The external result shows larger prompt/depth combinations materially increase
end-to-end first-token time. The largest inference row, depth 1024 and prompt
1024, reported `196352.9143340129` ms E2E TTFT.

As in the 128-token smoke, `llama-benchy` does not expose Ferrite's
`usage.prompt_tokens_details.cached_tokens` field in saved JSON.

## Direct Ferrite Cache-Hit Probe

To verify cached-token metadata at larger prompt sizes, three direct
OpenAI-compatible repeated-prompt probes were sent against the same live server.
Each scenario sends two identical non-streaming requests with the same
`prompt_cache_key`.

Raw result:
`documentation/benchmarks/2026-07-02-ferrite-qwen-0-5b-prefix-cache-direct-matrix.json`

| Scenario | Prompt tokens | Request 1 cached | Request 1 ms | Request 2 cached | Request 2 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| target-256 | 342 | 0 | 17226.462082995567 | 342 | 575.041833013529 |
| target-512 | 678 | 0 | 39789.818249992095 | 678 | 752.9950000171084 |
| target-1024 | 1350 | 0 | 103150.62875000876 | 1350 | 1117.001792008523 |

The direct probe confirms full exact-prompt cache hits at all three larger
sizes. The second request in every scenario reports all prompt tokens cached and
drops to sub-1.2-second elapsed time for an 8-token completion.

## RSS Sampling

RSS was sampled with `ps -o rss= -p <pid>` once per second while the server
process was alive.

Raw RSS sample:
`target/proof/llama-benchy-qwen-0-5b-prefix-matrix-rss.tsv`

- Samples: 1337
- First sample bytes: 430915584
- Last sample bytes: 485572608
- Peak bytes: 543719424
- Minimum bytes: 174489600

The matrix stayed bounded on this local run, but peak RSS was higher than
the earlier 128-token prefix smoke. The low minimum appears during
startup/loading and is not treated as a steady loaded-model RSS floor.

## Interpretation

Ferrite now has external `llama-benchy` prefix-cache evidence at 256, 512, and
1024 prompt/context sizes, plus direct Ferrite metadata proving repeated exact
prompts hit the experimental prefix cache.

The list-form `llama-benchy` command is a cross-product. It is useful for
research evidence but too heavy for a quick default gate on a local Mac. A
future repeatable gate should either run a diagonal-only wrapper or keep this
cross-product as an explicit long benchmark.

## Limits

This does not prove:

- 256/512/1024 generated-token prefix-cache behavior;
- generated-context long-chat prefix reuse;
- partial-prefix cache reuse;
- cache eviction policy under pressure;
- high-concurrency prefix-cache behavior;
- reconnect/error behavior under cache pressure;
- stop/EOS behavior under cached prompts;
- long-running RSS stability.
