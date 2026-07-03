# Benchmark: Local Qwen 0.5B Tokenizer CLI Isolation

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Measure long-prompt tokenizer latency outside the OpenAI server lifecycle and
outside generation. This uses the dedicated `ferrite` CLI tokenizer benchmark
mode so prompt tokenization can be measured without loading scalar model
weights into retained runtime memory.

The earlier server lifecycle proof reported
`prompt_tokenized_elapsed_ms=8323` after BPE metadata preparse. This run checks
the same prompt shape through a smaller tokenizer-only path.

## Environment

- Ferrite commit: `1fe2337`
- Host: local macOS workspace
- Binary: `target/release/ferrite`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Proof directory:
  `target/proof/local-qwen05-tokenizer-cli-benchmark-2026-07-03/`
- Ferrite binary SHA256:
  `2db9fbbc4ad185d64978d5ee6c7438dd7961718f9a292b4c6b51811f2f65c801`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Probe

The command shape was:

```text
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt <155399-char prompt> --benchmark-tokenization-runs 1
```

The prompt contained `155399` characters. The CLI reported `19428` tokens.

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-tokenizer-cli-benchmark-2026-07-03/tokenizer-cli.json` | 15 | `db7d3cc8f63ce1e4abf5109731c78bece687a04aad4c70edefd7b304d378934e` |
| `target/proof/local-qwen05-tokenizer-cli-benchmark-2026-07-03/tokenizer-cli.stdout` | 7 | `47b85b13dc129d24d4812d428f78d33ef71fb866d4343ba43eb2262c6ec02ccd` |
| `target/proof/local-qwen05-tokenizer-cli-benchmark-2026-07-03/tokenizer-cli.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Result

```text
tokenization_benchmark_runs=1
tokenization_benchmark_prompt_bytes=155399
tokenization_benchmark_token_count=19428
tokenization_benchmark_total_ns=8188082917
tokenization_benchmark_avg_ns=8188082917
model_file_bytes=397808192
model_file_retained_bytes=0
```

The harness wrapper recorded:

```text
wall_ms=8732.88875000435
returncode=0
```

## Interpretation

The isolated CLI path agrees with the server lifecycle result: this prompt's
tokenization path remains roughly eight seconds on the local machine after BPE
metadata preparse. The CLI mode removes HTTP streaming, OpenAI request
lifecycle, and generation from the measurement, so the remaining work is
tokenizer load plus prompt encode.

This does not prove a broad tokenizer throughput claim. It is one local sample
for one Qwen2.5-0.5B Q4_K_M prompt. It does, however, give a cleaner baseline
for tokenizer algorithm experiments than the streaming server lifecycle line.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The benchmark used one run, so it is a baseline probe rather than a stable
  distribution.
- The command still parses GGUF metadata and loads tokenizer metadata; it does
  not isolate only the inner BPE encode loop.
- The result should not be compared directly to full request latency or
  generation throughput.

## Next Step

Use this CLI benchmark mode for repeated tokenizer theory tests. The next
highest-value experiments are:

- measure repeated runs in one process to separate tokenizer metadata load from
  encode latency;
- test a pair-queue or adjacent-pair rank algorithm for BPE merges;
- test prompt-prefix symbol caching for repeated long-chat prefixes.
