# 2026-06-27 CLI Top-Logits Diagnostic

## Scope

This slice adds a CLI diagnostic flag:

```sh
--top-logits <count>
```

The flag prints the top next-token logits for the current prompt state, sorted
by descending logit and then ascending token ID for deterministic ties.

## Why

The SmolLM2-360M Tier 0 probe exposed a backend-sensitive reference split:
Ferrite and default local `llama.cpp` generation picked token `284` after
`[28120, 905, 18]`, while CPU-only `llama.cpp` picked token `198`. A top-logits
view is the smallest useful diagnostic for checking whether such divergences
are near ties or large correctness failures.

## Evidence

Red:

```sh
cargo test -p ferrite-cli --test next_token_cli cli_prints_top_next_token_logits
```

Failed because `--top-logits` was unknown.

Green:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
rg -n "TODO|TBD|expect\(|unwrap\(|panic!|unsafe" Cargo.toml crates
```

The hygiene scan only reported the existing workspace lint setting:

```text
Cargo.toml:16:unsafe_code = "forbid"
```

Real 360M divergence-point probe:

```sh
target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt-token-ids 28120,905,18 --top-logits 8 --expect-token-id 284
```

Output:

```text
prompt_token_ids=28120,905,18
next_token_id=284
next_token=Ġand
top_logits=284:18.689020,198:18.645466,314:18.396881,288:18.296913,281:18.225044,347:17.635653,355:17.402699,2489:17.103884
model_file_bytes=270590880
model_file_retained_bytes=0
scalar_weight_bytes=268803840
kv_cache_bytes=245760
expected_token_id=284
match=true
```

## Boundaries

This is a diagnostic output only. It does not compare logits against
`llama.cpp`, does not emit logits for every generated step, and does not change
sampling or generation behavior.
