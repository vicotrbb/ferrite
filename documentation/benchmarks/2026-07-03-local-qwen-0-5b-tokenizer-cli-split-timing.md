# Benchmark: Local Qwen 0.5B Tokenizer CLI Split Timing

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Measure tokenizer benchmark setup and encode timing separately. This run uses
the dedicated tokenizer-only CLI mode after adding fields for GGUF parse time,
tokenizer load time, and repeated prompt encode time.

The goal is to determine whether the next tokenizer optimization should target
metadata setup or the BPE encode loop itself.

## Environment

- Ferrite commit: `59a833b`
- Host: local macOS workspace
- Binary: `target/release/ferrite`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Proof directory:
  `target/proof/local-qwen05-tokenizer-cli-split-timing-2026-07-03/`
- Ferrite binary SHA256:
  `b2fa6cd9746ab1202b026063e882ff4195792dd5870f8d8b95bc0041aac6b172`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Probe

The command shape was:

```text
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt <155399-byte generated prompt> --benchmark-tokenization-runs 3
```

The prompt was a deterministic same-size local sample. The original
tokenizer-only baseline did not save prompt bytes, so this result should be
treated as a new comparable-size sample, not as a byte-identical rerun.

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-tokenizer-cli-split-timing-2026-07-03/tokenizer-cli.json` | 21 | `56db55780bc9df75183cfda7cf79188fff0dc1aac8b2534b1ff43686616f44fb` |
| `target/proof/local-qwen05-tokenizer-cli-split-timing-2026-07-03/tokenizer-cli.stdout` | 11 | `04b4a1f13ef8d96dd12aa9eb4acddc02df1d6ff686ae32e6f9b9413b972b4a47` |
| `target/proof/local-qwen05-tokenizer-cli-split-timing-2026-07-03/tokenizer-cli.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Result

```text
tokenization_benchmark_runs=3
tokenization_benchmark_prompt_bytes=155399
tokenization_benchmark_token_count=29527
tokenization_benchmark_gguf_parse_ns=13452375
tokenization_benchmark_tokenizer_load_ns=86463500
tokenization_benchmark_encode_total_ns=20377829375
tokenization_benchmark_encode_avg_ns=6792609791
tokenization_benchmark_total_ns=20377829375
tokenization_benchmark_avg_ns=6792609791
model_file_bytes=397808192
model_file_retained_bytes=0
```

The harness wrapper recorded:

```text
wall_ms=20965.1027090149
returncode=0
```

## Interpretation

The setup split is decisive for this sample:

- GGUF parse: about `13.45 ms`;
- tokenizer load: about `86.46 ms`;
- average prompt encode: about `6792.61 ms`.

The encode loop is orders of magnitude larger than GGUF parse or tokenizer
load. That supports the BPE tokenizer throughput theory: the next meaningful
optimization target is the encode algorithm, not additional metadata setup.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The prompt is a generated same-size sample, not the exact prompt from the
  prior server lifecycle probes.
- The CLI still includes the overhead of invoking the process and reading the
  model file before the printed timing fields.
- This proves where time sits in this local tokenizer-only path; it does not
  prove request-level throughput or generation behavior.

## Next Step

Prototype an alternate BPE encode path behind parity tests. Use the split
timing fields to compare:

- `tokenization_benchmark_token_count`;
- `tokenization_benchmark_encode_avg_ns`;
- token ID parity against the current implementation.
