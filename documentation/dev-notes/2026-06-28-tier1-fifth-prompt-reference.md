# Tier 1 Fifth Prompt Reference Probe

Date: 2026-06-28

## Scope

This slice expands local Tier 1 prompt breadth beyond the first four fixed
profiles. It adds the prompt:

```text
Machine learning models can
```

The check covers the currently local Tier 1 artifacts:

- `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`;
- `qwen2.5-1.5b-instruct-q4_k_m.gguf`;
- `Qwen2.5-0.5B-Instruct-Q8_0.gguf`;
- `Qwen2.5-0.5B-Instruct-Q6_K.gguf`.

This is local aarch64 prompt-breadth evidence. It does not add x86_64 coverage
for the fifth prompt and does not complete Tier 1.

## Reference Generation

References used local `llama.cpp` with deterministic decode:

```sh
target/reference/llama.cpp/build/bin/llama-completion \
  -m "$model" \
  -p 'Machine learning models can' \
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

## Reference Results

| Model | Prompt token IDs | Reference continuation | Reference token IDs |
| --- | --- | --- | --- |
| SmolLM2-1.7B Q4_K_M | `[29968,1380,2859,416]` | ` also be used to analyze the` | `[597,325,804,288,6524,260]` |
| Qwen2.5-0.5B Q4_K_M | `[21605,6832,4119,646]` | ` be used to predict the likelihood` | `[387,1483,311,7023,279,28636]` |
| Qwen2.5-1.5B Q4_K_M | `[21605,6832,4119,646]` | ` be used to predict the likelihood` | `[387,1483,311,7023,279,28636]` |
| Qwen2.5-0.5B Q8_0 | `[21605,6832,4119,646]` | ` be used to predict the likelihood` | `[387,1483,311,7023,279,28636]` |
| Qwen2.5-0.5B Q6_K | `[21605,6832,4119,646]` | ` be used to predict the likelihood` | `[387,1483,311,7023,279,28636]` |

## Ferrite Checks

Ferrite matched all five deterministic six-token continuations:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt 'Machine learning models can' \
  --generate-tokens 6 \
  --expect-token-id 597 \
  --expect-generated-token-ids 597,325,804,288,6524,260

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --prompt 'Machine learning models can' \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636

target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt 'Machine learning models can' \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt 'Machine learning models can' \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'Machine learning models can' \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636
```

Every run reported `generated_match=true` and `match=true`.

## Conclusion

Ferrite now has local aarch64 fifth-prompt parity for the current Tier 1 local
model/quantization set. Broader prompt coverage, x86_64 fifth-prompt coverage,
and 1.5B additional quantizations remain open.
