# 2026-06-27 Tier 0 SmolLM2 Multi-Token Reference Comparison

## Scope

This slice compares Ferrite's first six generated token IDs for a real text
prompt against a locally built `llama.cpp` reference runtime.

## Model

- Repo: `bartowski/SmolLM2-135M-Instruct-GGUF`
- File: `SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Hugging Face repo commit observed during download: `09816acd5d99df7be770d85ea30822623dab342c`
- Local path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`

## Reference Runtime

- Runtime: `ggml-org/llama.cpp`
- Local path: `target/reference/llama.cpp`
- Commit: `0ed235ea2c17a19fc8238668653946721ed136fd`
- Built tools:
  - `target/reference/llama.cpp/build/bin/llama-tokenize`
  - `target/reference/llama.cpp/build/bin/llama-simple`

## Ferrite Result

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Output:

```text
prompt_token_ids=28120,905
next_token_id=30
next_token=.
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,198,57,5248,597
benchmark_total_ns=667996167
benchmark_avg_ns=133599233
```

The generated token IDs from the initial next-token probe plus the repeated
session benchmark are:

```text
[30, 198, 198, 57, 5248, 597]
```

## llama.cpp Result

Command:

```sh
target/reference/llama.cpp/build/bin/llama-simple -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf -n 6 'hello world'
```

Relevant stdout:

```text
hello world.

I'm also
```

Command:

```sh
printf "hello world.\n\nI'm also" | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[28120, 905, 30, 198, 198, 57, 5248, 597]
```

Command:

```sh
printf ".\n\nI'm also" | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[30, 198, 198, 57, 5248, 597]
```

## Result

Ferrite and `llama.cpp` agree on the deterministic greedy six-token
continuation for the prompt `hello world`:

- Prompt token IDs: `[28120, 905]`
- Generated token IDs: `[30, 198, 198, 57, 5248, 597]`
- Generated text: `.\n\nI'm also`

This extends the Tier 0 correctness checkpoint from a single token to a short
multi-token continuation while reusing Ferrite's scalar session cache. It does
not yet prove parity for longer generations, sampling, chat-template prompts,
or stop-token handling.
