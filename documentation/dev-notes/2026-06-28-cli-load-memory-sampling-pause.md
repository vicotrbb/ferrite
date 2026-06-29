# 2026-06-28 CLI Load Memory Sampling Pause

## Scope

This slice adds a small CLI probe hook for steady-state memory measurement after
model load. It does not change inference behavior unless the operator passes
the new flag.

## Change

- Added `--sleep-after-load-ms <ms>` to `ferrite`.
- The CLI now pauses after:
  - reading and parsing the GGUF;
  - building the tokenizer;
  - building the scalar model; and
  - dropping the raw GGUF byte buffer.
- The CLI prints `sleep_after_load_ms=<ms>` and flushes stdout before sleeping
  so external samplers can synchronize on the load boundary.

## TDD Evidence

Red test:

```sh
cargo test -p ferrite-cli cli_can_pause_after_model_load_for_memory_sampling -- --nocapture
```

Initial result:

```text
unknown argument --sleep-after-load-ms
test cli_can_pause_after_model_load_for_memory_sampling ... FAILED
```

Green test after implementation:

```text
test cli_can_pause_after_model_load_for_memory_sampling ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out
```

Package verification:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-cli -- --nocapture
cargo clippy -p ferrite-cli --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p ferrite-cli -- --nocapture`: 22 passed.
- `cargo clippy -p ferrite-cli --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Measurement Note

The first attempted shell sampler wrapped the command in `/usr/bin/time -l`,
which meant `$!` referred to the `time` wrapper rather than the Ferrite process.
The retained benchmark protocol starts `target/release/ferrite` directly,
waits for `sleep_after_load_ms=5000`, samples `ps -o rss= -p "$pid"`, and
keeps `/usr/bin/time -l` peak RSS as a separate measurement.
