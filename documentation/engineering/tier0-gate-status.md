# Tier 0 Gate Status

Date: 2026-06-27

## Scope

This document summarizes Ferrite's current Tier 0 evidence against
`research/11-testing-model-registry.md`.

Tier 0 covers:

- `SmolLM2-135M-Instruct`
- `SmolLM2-360M-Instruct`

The registry criteria are:

- GGUF parser loads model successfully.
- Forward pass produces output.
- Token output matches `llama.cpp` reference output.
- Streaming mode works.

## Current Verdict

Tier 0 is close, but not complete.

Ferrite has strong local evidence for loading, scalar forward execution,
generation, streaming, memory accounting, and benchmark behavior for both Tier
0 models. The remaining gap is the SmolLM2-360M CPU-only `llama.cpp` reference
split: Ferrite matches default local `llama.cpp`, while CPU-only `llama.cpp`
produces a different continuation after the first generated token. The split is
a near-tie in Ferrite logits and appears backend-sensitive, but it is not yet
resolved.

## Evidence Matrix

| Criterion | 135M Status | 360M Status | Evidence |
| --- | --- | --- | --- |
| GGUF parser loads model | Proven | Proven | `documentation/dev-notes/2026-06-27-q6-k-loader-slice.md`, `documentation/dev-notes/2026-06-27-tier0-smollm2-360m-probe.md` |
| Forward pass produces output | Proven | Proven | `documentation/dev-notes/2026-06-27-tier0-smollm2-reference-comparison.md`, `documentation/dev-notes/2026-06-27-tier0-smollm2-360m-probe.md` |
| Token output matches `llama.cpp` | Proven across checked local modes | Partial | 135M matches default, CPU-only, and CPU no-repack `llama-completion`; 360M matches default `llama.cpp`, but diverges from CPU-only paths |
| Streaming mode works | Proven | Proven | `documentation/dev-notes/2026-06-27-cli-generation-mode.md`, `documentation/dev-notes/2026-06-27-tier0-smollm2-360m-probe.md` |
| Memory and latency documented | Proven | Proven | `documentation/benchmarks/2026-06-27-tier0-smollm2-q6k-direct-block-accumulation.md`, `documentation/benchmarks/2026-06-27-tier0-smollm2-360m-scalar-probe.md` |

## Fresh 135M Reference Check

Commands:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf -p 'hello world' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf -p 'hello world' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1 --device none
target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf -p 'hello world' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1 --device none --no-repack
```

All three produced:

```text
.

I'm also
```

Tokenization:

```sh
printf '.\n\nI\x27m also' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[30, 198, 198, 57, 5248, 597]
```

Ferrite gate:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 30 --expect-generated-token-ids 30,198,198,57,5248,597
```

Relevant output:

```text
generated_token_ids=30,198,198,57,5248,597
expected_generated_token_ids=30,198,198,57,5248,597
generated_match=true
match=true
```

## Fresh 360M Reference Check

Ferrite gate:

```sh
target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,284,476,28120,905,18
```

Relevant output:

```text
generated_token_ids=18,284,476,28120,905,18
expected_generated_token_ids=18,284,476,28120,905,18
generated_match=true
match=true
```

Default local `llama.cpp` reference produced the same continuation after
tokenization:

```text
[18, 284, 476, 28120, 905, 18]
```

CPU-only `llama.cpp` paths diverged after the first generated quote token:

- CPU-only output: `"\nprint(word)`
- CPU-only token IDs: `[18, 198, 3272, 24, 3002, 25]`
- CPU-only no-repack output: `"\nprint(convert_`

Ferrite diagnostic at the divergence point:

```sh
target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt-token-ids 28120,905,18 --top-logits 8 --expect-token-id 284
```

Relevant output:

```text
top_logits=284:18.689020,198:18.645466,314:18.396881,288:18.296913,281:18.225044,347:17.635653,355:17.402699,2489:17.103884
match=true
```

The first divergent candidate margin is about `0.043554` logits.

## Remaining Work Before Tier 1

- Decide the reference policy for backend-sensitive quantized ties:
  - either define Ferrite's scalar path as compared against a fixed `llama.cpp`
    backend mode,
  - or require CPU-only `llama.cpp` parity and investigate the 360M mismatch
    down to the tensor/kernel level.
- Record that policy in an ADR or update an existing ADR.
- If CPU-only parity is required, add a deeper diagnostic path that can compare
  intermediate logits or layer outputs against `llama.cpp` for the reduced
  prompt `[28120, 905, 18]`.
- Only then mark Tier 0 complete and start Tier 1 SIMD/GQA validation work.
