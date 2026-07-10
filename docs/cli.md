# Command-line interface

The `ferrite` binary is a low-level generation, parity, profiling, and
benchmarking interface. It prints stable `key=value` records so scripts can
parse results without scraping prose.

```sh
target/release/ferrite --help
```

## Required inputs

Every inference command requires a model and exactly one prompt source:

```text
--model <path.gguf>
--prompt <text>
--prompt-token-ids <id[,id...]>
```

Use either `--prompt` or `--prompt-token-ids`, never both. Text prompts pass
through the GGUF tokenizer. Token IDs bypass encoding and are useful for exact
parity tests.

## Generation

Without a generation or benchmark mode, Ferrite evaluates the prompt and
reports the next token and logits metadata.

- `--generate-tokens <count>` generates up to the requested count and stops
  early at the model's EOS token when one is configured.
- `--stream` emits each generated token immediately. It requires
  `--generate-tokens`.
- `--top-logits <count>` prints the highest logits for the initial next token.
- `--expect-token-id <id>` fails if the initial next token differs.
- `--expect-generated-token-ids <ids>` fails if the generated trace differs.

## Threads and cache

- `--threads <count>` overrides Ferrite's automatic worker selection. Use it
  for controlled experiments, not as an assumed optimization.
- `--kv-backend vec` selects the default in-memory vector cache.
- `--kv-backend locus` selects the Locus block-pool cache and requires a build
  with `--features locus-kv`.
- `--kv-tokens-per-block <count>` controls Locus block granularity. The default
  is 16.
- `--kv-max-tokens <count>` caps Locus capacity. When omitted, Ferrite sizes it
  from prompt, generation, benchmark, and safety headroom.

Build the optional backend with:

```sh
cargo build --release --locked -p ferrite-cli --features locus-kv
```

## Benchmarks and profiling

- `--benchmark-runs <count>` measures repeated decode steps after prompt
  evaluation.
- `--benchmark-batch-streams <count>` runs the benchmark across that many
  sessions and reports aggregate throughput. It requires `--benchmark-runs`.
- `--benchmark-tokenization-runs <count>` measures encoding only and requires a
  text prompt.
- `--profile-next-token` reports the initial token's stage timings.
- `--profile-benchmark-token` profiles the first measured benchmark step and
  requires `--benchmark-runs`.
- `--sleep-after-load-ms <ms>` pauses after model loading and raw GGUF buffer
  release, allowing an external RSS sampler to measure retained memory.

## Experimental activation policies

- `--experimental-q8-k-activation-matvec` enables the parity-scoped Q8_K
  activation policy.
- `--experimental-residual-q8-activation-matvec` enables the Arm I8MM residual
  policy on supported hardware.
- `--experimental-q8-k-activation-roles <roles>` limits an experimental or
  comparison policy to `q_proj`, `k_proj`, `v_proj`, `o_proj`, `ffn_gate`,
  `ffn_up`, `ffn_down`, `output`, or the alias `all`.
- `--compare-q8-k-activation-matvec` compares candidate and exact matrix paths
  while profiling. It requires `--profile-next-token` or
  `--profile-benchmark-token`.

Experimental policies are mutually exclusive. Treat them as measured options,
not compatibility defaults. See [the performance golden path](performance.md).

## Output contract

Common records include:

```text
inference_threads
prompt_token_ids
q8_k_activation_matvec_policy
next_token_id
generated_token_ids
generated_text
benchmark_avg_ns
benchmark_batch_tokens_per_second
model_file_bytes
scalar_weight_bytes
kv_cache_bytes
```

Additional records are mode-specific. The eval harness consumes this output,
so changing a key is an API change and requires corresponding harness tests.
