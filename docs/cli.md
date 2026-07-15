# Command-line interface

The `ferrite` binary is a low-level generation, parity, profiling, and
benchmarking interface. It prints stable `key=value` records so scripts can
parse results without scraping prose.

Treat the selected GGUF artifact as immutable until the Ferrite process exits;
replacing or truncating a live mapped model file is unsupported.

```sh
target/release/ferrite --help
target/release/ferrite --version
```

## Required inputs

Every inference command requires one model source and exactly one prompt
source:

```text
--model <path.gguf>
--model-id <built-in-id>
--prompt <text>
--prompt-token-ids <id[,id...]>
```

Use either `--model` or `--model-id`, never both. Use either `--prompt` or
`--prompt-token-ids`, never both. Text prompts pass through the GGUF tokenizer.
Token IDs bypass encoding and are useful for exact parity tests.

## Verified built-in model acquisition

`--model-id phi3-mini-4k-instruct-q4` selects the pinned official Microsoft
Phi-3 Mini 4K Instruct Q4 artifact. When it is missing, Ferrite uses `curl` for
an HTTPS-only resumable download into a partial file, verifies the exact size
and SHA-256, renames it atomically, writes `artifact.json`, and marks both final
files read-only. Existing cache entries are rehashed before use.

- `--model-cache <directory>` overrides the platform cache root.
- `FERRITE_MODEL_CACHE` provides the same override when the CLI flag is absent.
- `--offline` requires an already cached and verified artifact.

Acquisition is opt-in through `--model-id`. An explicit `--model` path never
causes network access. Ferrite does not send prompts, generated text, or usage
telemetry to the model source.

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

## Sampling and token stops

Generation is exact greedy by default. The fused argmax path remains active
when no option needs complete logits.

- `--temperature <0..2>` enables stochastic sampling above zero.
- `--top-k <count>` keeps at most that many ranked candidates. Zero disables
  the filter.
- `--top-p <0..1>` applies nucleus filtering.
- `--min-p <0..1>` removes candidates below that fraction of the maximum
  probability.
- `--repetition-penalty <positive>` applies a multiplicative history penalty.
- `--frequency-penalty <-2..2>` scales a penalty by prior token count.
- `--presence-penalty <-2..2>` applies a penalty when a token has appeared.
- `--logit-bias <id:bias[,id:bias...]>` adds biases from -100 through 100 to
  selected token IDs.
- `--seed <i64>` selects deterministic per-generation random state.
- `--stop-token-ids <id[,id...]>` ends generation when one listed token is
  selected.

`generated_token_ids` and `stream_token_id` retain a selected terminal token
for exact trace inspection. Ferrite suppresses EOS, EOT, EOM, model-native turn
terminators, and configured stop-token IDs from `generated_text` and
`stream_text`.

Sampling and stop-token options require `--generate-tokens`. Probability
filters take effect only when temperature is positive. Penalties and logit bias
also affect deterministic selection at temperature zero.

## Threads and cache

- `--threads <count>` overrides Ferrite's automatic worker selection. Use it
  for controlled experiments, not as an assumed optimization.
- `--kernel-provider auto` uses runtime-gated proven kernels. `portable` forces
  the architecture-neutral correctness path for parity and diagnosis.
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
- `--sleep-after-load-ms <ms>` pauses after model loading, allowing an external
  RSS sampler to measure retained heap storage plus the shared read-only GGUF
  mapping.

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
model_registry_id
model_source
model_revision
model_license
model_filename
model_expected_bytes
model_sha256
inference_threads
kernel_provider
cpu_features
prompt_token_ids
q8_k_activation_matvec_policy
next_token_id
generated_token_ids
generated_text
sampling_temperature
sampling_fused_greedy_path
sampling_effective_seed
benchmark_avg_ns
benchmark_batch_tokens_per_second
model_file_bytes
scalar_weight_bytes
kv_cache_bytes
```

Additional records are mode-specific. The eval harness consumes this output,
so changing a key is an API change and requires corresponding harness tests.

The server and both operational clients expose the same help and version
conventions. See [operational tools](benchmark-tools.md) for their contracts.
