# Evaluation and regression gates

Ferrite separates fast deterministic tests from real-model and performance
gates. A code change is complete only after the checks proportional to its risk
have passed.

## Required repository checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
python3 scripts/check_docs.py
python3 scripts/eval_test.py
cargo audit --deny warnings
cargo deny --all-features --locked check
cargo machete
cargo tree --duplicates --locked
```

`cargo-audit`, `cargo-deny`, and `cargo-machete` are additional development
tools. Install each with `cargo install --locked <tool>` if it is not already
available.

## Test layers

1. Unit tests validate parsing, metadata, kernels, math, schemas, scheduler
   components, and edge conditions.
2. Integration tests validate crate boundaries, generated GGUF fixtures, CLI
   behavior, HTTP behavior, cancellation, caching, and batching.
3. Ignored real-model tests validate known GGUF artifacts and third-party client
   compatibility.
4. The eval harness records performance, memory, CPU, TTFT, latency, and token
   traces on a named machine.

## Real-model test variables

Ignored tests can use these paths:

```text
FERRITE_REAL_MODEL
FERRITE_REAL_TIER1_MODEL
FERRITE_QWEN_1_5B_Q6_MODEL
FERRITE_QWEN_1_5B_Q8_MODEL
FERRITE_SMOLLM_1_7B_Q4_MODEL
```

Run one ignored target explicitly after setting the matching variable. Avoid
`cargo test -- --ignored` across the entire workspace unless every required
artifact is present and the long-running cost is intentional.

## Eval harness

```sh
scripts/eval.sh --help
```

The harness builds locked release binaries, runs CLI generation and precise
decode, optionally runs fixed engine batches, starts the HTTP server, drives a
streaming throughput client, samples RSS and CPU through `ps`, and writes JSON
plus Markdown to `scripts/evals/`.

A minimal local run is:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --generate-tokens 64 \
  --benchmark-runs 64
```

For optimization work, preserve both before and after artifacts. Compare the
same machine, model hash, prompt, generated-token count, benchmark length,
policy, worker count, build flags, and thermal conditions.

## Regression policy

- Correctness and API changes require deterministic tests.
- Unsafe changes require direct kernel parity tests and cross-architecture CI.
- Hot-path changes require a comparable eval and token trace.
- Cache or scheduler changes require cancellation, queueing, disconnect, and
  memory-bound tests.
- Documentation claims require a command, test, ADR, benchmark, or source.

If throughput improves while token parity, TTFT, memory, or tail latency
regresses, record the tradeoff and do not call the change unconditionally
better.
