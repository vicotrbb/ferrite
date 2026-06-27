# 2026-06-27 Tier 0 SmolLM2 Reference Comparison

## Scope

This slice compares Ferrite's first real Tier 0 next-token result against a
locally built `llama.cpp` reference runtime.

## Model

- Repo: `bartowski/SmolLM2-135M-Instruct-GGUF`
- File: `SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Hugging Face repo commit observed during download: `09816acd5d99df7be770d85ea30822623dab342c`
- Local path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Local size: 101 MB

## Reference Runtime

- Runtime: `ggml-org/llama.cpp`
- Local path: `target/reference/llama.cpp`
- Commit: `0ed235ea2c17a19fc8238668653946721ed136fd`
- Built tools:
  - `target/reference/llama.cpp/build/bin/llama-tokenize`
  - `target/reference/llama.cpp/build/bin/llama-simple`

## Prompt Alignment

The comparison prompt is the special token text `<|im_start|>`.

- `llama-tokenize --log-disable --model
  target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --ids --no-bos --prompt
  '<|im_start|>'` returned `[1]`.
- `llama-tokenize --log-disable --model
  target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --ids --no-bos --prompt
  ','` returned `[28]`.

## Ferrite Result

Command:

```sh
cargo run -p ferrite-cli -- --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt-token-ids 1
```

Output:

```text
prompt_token_ids=1
next_token_id=28
next_token=,
```

## llama.cpp Result

Command:

```sh
target/reference/llama.cpp/build/bin/llama-simple -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf -n 1 '<|im_start|>'
```

Relevant output:

```text
<|im_start|>,
```

## Result

Ferrite and `llama.cpp` agree on the next token for the aligned prompt:

- Prompt token IDs: `[1]`
- Next token ID: `28`
- Next token text: `,`

This satisfies the first Tier 0 correctness checkpoint for one deterministic
single-token SmolLM2 Q4_K_M probe. It does not yet prove tokenizer parity for
general text prompts, multi-token generation parity, or benchmark readiness.
