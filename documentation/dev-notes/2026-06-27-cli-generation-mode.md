# 2026-06-27 CLI Generation Mode

## Scope

This slice adds an explicit CLI generation mode for Tier 0 probes:

```sh
--generate-tokens <count>
```

The mode emits the first predicted token after the prompt plus `count - 1`
incremental continuations from the same scalar session. It is separate from
`--benchmark-runs`, which remains a timing surface.

`--stream` can be combined with `--generate-tokens` to print each generated
token chunk as it is produced before the final generation summary.

## Evidence

Code verification:

```sh
cargo test -p ferrite-cli --test next_token_cli
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

Release build:

```sh
cargo build --release -p ferrite-cli
```

Real Tier 0 generation probe:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 30
```

Output:

```text
prompt_token_ids=28120,905
next_token_id=30
next_token=.
generated_cached_tokens=8
generated_token_ids=30,198,198,57,5248,597
generated_text=.

I'm also
model_file_bytes=105454432
model_file_retained_bytes=0
scalar_weight_bytes=103668480
kv_cache_bytes=368640
expected_token_id=30
match=true
```

The generated token IDs match the existing `llama.cpp` reference comparison
for the prompt `hello world`: `[30, 198, 198, 57, 5248, 597]`.

Streaming probe:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --stream --expect-token-id 30
```

Output:

```text
prompt_token_ids=28120,905
next_token_id=30
next_token=.
stream_token_id=30
stream_text=.
stream_token_id=198
stream_text=

stream_token_id=198
stream_text=

stream_token_id=57
stream_text=I
stream_token_id=5248
stream_text='m
stream_token_id=597
stream_text= also
generated_cached_tokens=8
generated_token_ids=30,198,198,57,5248,597
generated_text=.

I'm also
model_file_bytes=105454432
model_file_retained_bytes=0
scalar_weight_bytes=103668480
kv_cache_bytes=368640
expected_token_id=30
match=true
```

## Boundaries

This mode does not add sampling, stop-token handling, or chat-template
rendering. BPE token display now applies the inverse GPT-2 byte alphabet, which
is enough for this SmolLM2 prompt but is still not a full chat/runtime text
policy.
