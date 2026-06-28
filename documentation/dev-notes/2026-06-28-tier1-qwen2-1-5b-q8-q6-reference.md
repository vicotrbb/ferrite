# Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K Reference Probe

Date: 2026-06-28

## Scope

This slice expands Qwen2.5-1.5B Tier 1 model-output evidence beyond Q4_K_M by
checking the Q8_0 and Q6_K GGUF artifacts from `Qwen/Qwen2.5-1.5B-Instruct-GGUF`.

This is local aarch64 evidence only. It does not prove x86_64 AVX2 parity or
throughput for these two additional 1.5B quantizations.

## Artifacts

```text
target/models/qwen2.5-1.5b-instruct-q6_k.gguf
size: 1.4G
sha256: e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7

target/models/qwen2.5-1.5b-instruct-q8_0.gguf
size: 1.8G
sha256: d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8
```

Downloaded with:

```sh
huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct-GGUF \
  qwen2.5-1.5b-instruct-q6_k.gguf \
  qwen2.5-1.5b-instruct-q8_0.gguf \
  --local-dir target/models
```

## Reference Method

Reference continuations were generated with local `llama.cpp`:

```sh
target/reference/llama.cpp/build/bin/llama-completion \
  -m "$model" \
  -p "$prompt" \
  -n 6 \
  --temp 0 \
  --top-k 1 \
  --top-p 1 \
  --repeat-last-n 0 \
  --no-conversation \
  --no-jinja \
  --no-display-prompt \
  --verbosity 1
```

Each continuation was tokenized with `llama-tokenize --stdin --ids --no-bos
--log-disable`, preserving leading whitespace and newlines. Ferrite then ran
with `--generate-tokens 6 --expect-generated-token-ids "$ids"`.

## Reference Continuations

Both Q6_K and Q8_0 produced the same six deterministic continuations:

| Prompt | Prompt token IDs | Reference continuation | Reference token IDs |
| --- | --- | --- | --- |
| `hello world` | `[14990,1879]` | `\nHello, world! How` | `[198,9707,11,1879,0,2585]` |
| `The capital of France is` | `[785,6722,315,9625,374]` | ` Paris. The capital of France` | `[12095,13,576,6722,315,9625]` |
| `Once upon a time` | `[12522,5193,264,882]` | `, there was a young girl` | `[11,1052,572,264,3908,3743]` |
| `Rust is a systems programming language` | `[49,590,374,264,5942,15473,4128]` | ` that is designed to be safe` | `[429,374,6188,311,387,6092]` |
| `Machine learning models can` | `[21605,6832,4119,646]` | ` be used to predict the future` | `[387,1483,311,7023,279,3853]` |
| `The recipe calls for` | `[785,11116,6738,369]` | ` 2 cups of flour and` | `[220,17,25374,315,19828,323]` |

## Ferrite Checks

All twelve Ferrite runs reported `generated_match=true` with default execution:

```text
q6_k:
hello world -> 198,9707,11,1879,0,2585
The capital of France is -> 12095,13,576,6722,315,9625
Once upon a time -> 11,1052,572,264,3908,3743
Rust is a systems programming language -> 429,374,6188,311,387,6092
Machine learning models can -> 387,1483,311,7023,279,3853
The recipe calls for -> 220,17,25374,315,19828,323

q8_0:
hello world -> 198,9707,11,1879,0,2585
The capital of France is -> 12095,13,576,6722,315,9625
Once upon a time -> 11,1052,572,264,3908,3743
Rust is a systems programming language -> 429,374,6188,311,387,6092
Machine learning models can -> 387,1483,311,7023,279,3853
The recipe calls for -> 220,17,25374,315,19828,323
```

The runs also reported:

```text
q6_k model_file_bytes=1464178720
q6_k scalar_weight_bytes=1458228224
q8_0 model_file_bytes=1894532128
q8_0 scalar_weight_bytes=1888581632
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
```

## Conclusion

Qwen2.5-1.5B now has local six-prompt deterministic model-output parity for
Q4_K_M, Q6_K, and Q8_0. The new Q6_K and Q8_0 checks are local aarch64
correctness evidence only; x86_64 parity and throughput remain separate work.
