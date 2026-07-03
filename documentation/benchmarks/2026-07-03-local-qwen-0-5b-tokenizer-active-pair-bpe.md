# Benchmark: Local Qwen 0.5B Active-Pair BPE Encoder

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Test the BPE tokenizer throughput theory by replacing the full merge-rule scan
with an active adjacent-pair rank loop. The benchmark checks both correctness
parity and tokenizer-only latency on the deterministic same-size prompt sample.

## Environment

- Ferrite commit: `6781e7f`
- Host: local macOS workspace
- Binary: `target/release/ferrite`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Proof directory:
  `target/proof/local-qwen05-tokenizer-active-pair-bpe-2026-07-03/`
- Ferrite binary SHA256:
  `e64fc170fac68f731b80126ed37666e42b64ac0a18b3c0c959425f9acb876c97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Probe

The command shape was:

```text
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt <155399-byte generated prompt> --benchmark-tokenization-runs 3
```

The acceptance target from the prior fingerprint proof was:

```text
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-tokenizer-active-pair-bpe-2026-07-03/tokenizer-cli.json` | 24 | `b42daefd74bac57b15159d1112611fedc76029e1c73ee9acdbd2785f914f8b30` |
| `target/proof/local-qwen05-tokenizer-active-pair-bpe-2026-07-03/tokenizer-cli.stdout` | 12 | `50fa6e8e8d1a367715ccfbd9749222f221255a6881d1a1bc7bff509d6f7e2684` |
| `target/proof/local-qwen05-tokenizer-active-pair-bpe-2026-07-03/tokenizer-cli.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Result

```text
tokenization_benchmark_runs=3
tokenization_benchmark_prompt_bytes=155399
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
tokenization_benchmark_gguf_parse_ns=13781792
tokenization_benchmark_tokenizer_load_ns=139168916
tokenization_benchmark_encode_total_ns=12186135500
tokenization_benchmark_encode_avg_ns=4062045166
tokenization_benchmark_total_ns=12186135500
tokenization_benchmark_avg_ns=4062045166
model_file_bytes=397808192
model_file_retained_bytes=0
```

The harness wrapper recorded:

```text
wall_ms=12793.965958000626
returncode=0
```

## Comparison

Prior fingerprint baseline:

```text
tokenization_benchmark_encode_avg_ns=6894344416
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
```

Active-pair encoder:

```text
tokenization_benchmark_encode_avg_ns=4062045166
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
```

The local sample improved by about `41.08%`:

```text
(6894344416 - 4062045166) / 6894344416 = 0.4108
```

## Interpretation

The theory passed this local gate. Token count and token-id fingerprint were
unchanged, while tokenizer-only average encode time fell from about `6.89 s`
to about `4.06 s`.

This supports keeping the active adjacent-pair rank loop. It also shows that
there is still substantial tokenizer work remaining, so a priority queue or
linked active-pair structure may be worth testing next.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- This is one three-run local sample, not a distribution.
- The prompt is the deterministic generated prompt from the previous
  fingerprint proof, not the original unsaved server lifecycle prompt.
- The active-pair implementation still scans the symbol list each merge
  iteration; it does not yet use a priority queue.
- This proves tokenizer-only latency movement, not end-to-end OpenAI request
  latency.

## Next Step

Rerun the OpenAI streaming cancellation proof with this encoder to confirm that
`prompt_tokenized_elapsed_ms` drops in the server lifecycle path. Separately,
test a priority-queue active-pair structure only if parity remains easy to
prove with the benchmark fingerprint.
