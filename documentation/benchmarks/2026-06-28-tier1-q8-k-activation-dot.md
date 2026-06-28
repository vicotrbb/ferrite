# Tier 1 Q8_K Activation Dot Evidence

Date: 2026-06-28

## Scope

This note records the first real Tier 1 model-output gate for the opt-in
Q4_K/Q6_K x Q8_K activation matvec path.

The route was enabled with:

```sh
--experimental-q8-k-activation-matvec
```

Default Q4_K/Q6_K dispatch remains unchanged.

## Build And Repository Gates

The release binary was rebuilt before model probes:

```sh
cargo build --release -p ferrite-cli
```

Repository gates passed before the model checks:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

## Qwen2.5-1.5B Opt-In Parity

Command:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11 --experimental-q8-k-activation-matvec
```

Result:

```text
experimental_q8_k_activation_matvec=true
generated_token_ids=198,9707,11
generated_match=true
```

Command:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'The capital of France is' --generate-tokens 3 --expect-generated-token-ids 12095,13,576 --experimental-q8-k-activation-matvec
```

Result:

```text
experimental_q8_k_activation_matvec=true
generated_token_ids=12095,13,576
generated_match=true
```

## SmolLM2-1.7B Opt-In Parity Failure

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-generated-token-ids 18,198,3725,198,198,788 --experimental-q8-k-activation-matvec
```

Result:

```text
experimental_q8_k_activation_matvec=true
generated_token_ids=18,198,198,3272,24,2334
generated_match=false
```

The default path for the same prompt still matched:

```text
experimental_q8_k_activation_matvec=false
generated_token_ids=18,198,3725,198,198,788
generated_match=true
```

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 6 --expect-generated-token-ids 7042,30,2 --experimental-q8-k-activation-matvec
```

Result:

```text
experimental_q8_k_activation_matvec=true
generated_token_ids=7042,30,198,504,3575,282
generated_match=false
```

The default path for the same prompt still matched:

```text
experimental_q8_k_activation_matvec=false
generated_token_ids=7042,30,2
generated_match=true
```

## Post Q6_K Argmax Option Refresh

After `documentation/dev-notes/2026-06-28-q8-k-q6-argmax-options.md`, the
release binary was rebuilt from commit `29cd21d`:

```sh
cargo build --release -p ferrite-cli
```

The Qwen2.5-1.5B opt-in Q8_K checks still matched:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11 --experimental-q8-k-activation-matvec
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'The capital of France is' --generate-tokens 3 --expect-generated-token-ids 12095,13,576 --experimental-q8-k-activation-matvec
```

Results:

```text
generated_token_ids=198,9707,11
generated_match=true
generated_token_ids=12095,13,576
generated_match=true
```

The SmolLM2-1.7B opt-in Q8_K checks still diverged:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --experimental-q8-k-activation-matvec
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 6 --experimental-q8-k-activation-matvec
```

Results:

```text
generated_token_ids=18,198,198,3272,24,2334
generated_token_ids=7042,30,198,504,3575,282
```

The same current binary still matched both default SmolLM2 references:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-generated-token-ids 18,198,3725,198,198,788
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 6 --expect-generated-token-ids 7042,30,2
```

Results:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
generated_token_ids=7042,30,2
generated_match=true
generated_stopped_on_eos=true
```

## Verdict

The opt-in Q8_K activation matvec path is not eligible for default Q4_K/Q6_K
dispatch.

The Qwen2.5-1.5B checks are encouraging but insufficient. SmolLM2-1.7B
multi-token parity fails on both documented prompts, while the default path
continues to match. The post-Q6_K-argmax-option refresh confirms that this
verdict still holds after experimental token-id-only decoding honors Q8_K
execution options.

A follow-up Qwen2.5-1.5B benchmark in
`documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-k-opt-in-benchmark.md`
shows that the opt-in route has real throughput value for that model, improving
the local default-pool benchmark average from 261,316,083 ns to 226,673,736 ns.
That does not change the default-dispatch verdict because SmolLM2-1.7B still
fails real model-output parity.

Follow-up boundary probes in
`documentation/dev-notes/2026-06-28-q8-k-smollm-boundary-probes.md` did not find
a localized output-projection or Q6_K-only formula hole. The current evidence
supports expected activation-quantization drift crossing narrow SmolLM2 output
margins, so next work should focus on a deliberate role/model parity policy or
a tighter activation quantization strategy.
