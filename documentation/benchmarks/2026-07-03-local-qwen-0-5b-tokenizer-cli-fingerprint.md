# Benchmark: Local Qwen 0.5B Tokenizer CLI Fingerprint

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Record a tokenizer-only benchmark run that includes a compact token-id
fingerprint. This gives future BPE algorithm experiments a stable parity signal
without printing tens of thousands of token IDs.

The fingerprint is deterministic FNV-1a 64-bit over the token ID sequence. It
is not a cryptographic hash.

## Environment

- Ferrite commit: `2e13801`
- Host: local macOS workspace
- Binary: `target/release/ferrite`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Proof directory:
  `target/proof/local-qwen05-tokenizer-cli-fingerprint-2026-07-03/`
- Ferrite binary SHA256:
  `eae3f90db24a8abad5e3f8c816d236456cc8b7eba0940a6d50847417a99492ed`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Probe

The command shape was:

```text
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt <155399-byte generated prompt> --benchmark-tokenization-runs 3
```

The prompt generator matched the split-timing sample. The result retained the
same token count and added a token-id fingerprint.

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-tokenizer-cli-fingerprint-2026-07-03/tokenizer-cli.json` | 22 | `44da7b882d45b69178962d5ccb4c276ce6aa17d83a9585758d99bfaf66f00532` |
| `target/proof/local-qwen05-tokenizer-cli-fingerprint-2026-07-03/tokenizer-cli.stdout` | 12 | `2004b40084356db3f9d518f37d2267f4ab76b98fba480355145ceca0ae9df7bf` |
| `target/proof/local-qwen05-tokenizer-cli-fingerprint-2026-07-03/tokenizer-cli.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Result

```text
tokenization_benchmark_runs=3
tokenization_benchmark_prompt_bytes=155399
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
tokenization_benchmark_gguf_parse_ns=23735000
tokenization_benchmark_tokenizer_load_ns=82196875
tokenization_benchmark_encode_total_ns=20683033250
tokenization_benchmark_encode_avg_ns=6894344416
tokenization_benchmark_total_ns=20683033250
tokenization_benchmark_avg_ns=6894344416
model_file_bytes=397808192
model_file_retained_bytes=0
```

The harness wrapper recorded:

```text
wall_ms=21278.751000005286
returncode=0
```

## Interpretation

The token count matched the prior split-timing sample:

```text
tokenization_benchmark_token_count=29527
```

The new parity fingerprint is:

```text
fnv1a64:468c718e7fb1e5a0
```

Future tokenizer algorithm experiments should preserve both the token count and
this fingerprint for the same prompt generator before comparing timing fields.

## Limits

- FNV-1a 64-bit is a compact benchmark fingerprint, not a cryptographic
  integrity hash.
- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The prompt is generated locally and should be reproduced exactly for parity
  comparisons.
- Timing movement between this run and the prior split-timing run should be
  treated as local run variance unless repeated samples are collected.

## Next Step

Use `tokenization_benchmark_token_ids_fingerprint` as the acceptance signal for
the first alternate BPE encode prototype. Timing comparisons are meaningful
only after parity holds.
