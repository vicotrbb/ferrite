# Tier 1 Sixth Prompt Reference Probe

Date: 2026-06-28

## Scope

This slice expands local Tier 1 prompt breadth beyond the first five fixed
profiles. It adds the prompt:

```text
The recipe calls for
```

The check covers the currently local Tier 1 artifacts:

- `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`;
- `qwen2.5-1.5b-instruct-q4_k_m.gguf`;
- `Qwen2.5-0.5B-Instruct-Q8_0.gguf`;
- `Qwen2.5-0.5B-Instruct-Q6_K.gguf`.

This is local aarch64 prompt-breadth evidence. It does not add x86_64 coverage
for the fifth or sixth prompts and does not complete Tier 1.

## Reference Generation

References used local `llama.cpp` with deterministic decode:

```sh
target/reference/llama.cpp/build/bin/llama-completion \
  -m "$model" \
  -p 'The recipe calls for' \
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

The Qwen2.5-0.5B continuation tokenization uses the exact generated text
without the display newline added by the terminal.

## Reference Results

| Model | Prompt token IDs | Reference continuation | Reference token IDs |
| --- | --- | --- | --- |
| SmolLM2-1.7B Q4_K_M | `[504,11594,6100,327]` | ` 2 cups of flour.` | `[216,34,12382,282,7367,30]` |
| Qwen2.5-0.5B Q4_K_M | `[785,11116,6738,369]` | ` 3 cups of flour.` | `[220,18,25374,315,19828,13]` |
| Qwen2.5-1.5B Q4_K_M | `[785,11116,6738,369]` | ` 2 cups of flour and` | `[220,17,25374,315,19828,323]` |
| Qwen2.5-0.5B Q8_0 | `[785,11116,6738,369]` | ` 3 cups of flour.` | `[220,18,25374,315,19828,13]` |
| Qwen2.5-0.5B Q6_K | `[785,11116,6738,369]` | ` 3 cups of flour.` | `[220,18,25374,315,19828,13]` |

## Ferrite Checks

Ferrite matched all five deterministic six-token continuations:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt 'The recipe calls for' \
  --generate-tokens 6 \
  --expect-token-id 216 \
  --expect-generated-token-ids 216,34,12382,282,7367,30

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --prompt 'The recipe calls for' \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13

target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt 'The recipe calls for' \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,17,25374,315,19828,323

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt 'The recipe calls for' \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'The recipe calls for' \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13
```

Every run reported `generated_match=true` and `match=true`.

## Operational Note

An attempted x86_64 AVX2 fifth-prompt run was started on the `staging` context
in pod `ferrite-avx2-fifth-prompt`, but the Kubernetes API connection reset
during the large model copy and then refused TCP connections on
`192.168.50.132:6443`. The pod was deleted after the API returned, and a final
`kubectl get pod ferrite-avx2-fifth-prompt --ignore-not-found` returned no
output. No x86_64 model-output result is claimed by this note.

## Conclusion

Ferrite now has local aarch64 sixth-prompt parity for the current Tier 1 local
model/quantization set. The follow-up
`documentation/dev-notes/2026-06-28-tier1-avx2-prompt-closure.md` adds the
matching x86_64 AVX2 sixth-prompt evidence. Broader prompt coverage and 1.5B
additional quantizations remain open.
