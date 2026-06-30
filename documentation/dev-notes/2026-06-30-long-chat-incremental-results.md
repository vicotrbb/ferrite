# Long-Chat Incremental Results

## Context

The long-chat gate can now run large OpenAI-compatible streaming matrices. The
first full Qwen 0.5B matrix showed that even a single-model 256/512/1024 run can
take several minutes. The previous CLI collected every scenario result before
printing any result block, which made long multi-model proof runs harder to
monitor and riskier to interrupt.

## Change

- Added `LongChatGateConfig::run_with_observer`.
- Added `LongChatGateConfig::run_with_executor_and_observer` for deterministic
  test coverage.
- Updated `ferrite-openai-long-chat-gate --execute` to print and flush each
  completed scenario result immediately.

The existing `run` and `run_with_executor` APIs remain available for callers
that want collected results.

## Validation

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

The focused long-chat integration target passed 20 tests, including
`observes_long_chat_results_as_each_scenario_finishes`.

## Limits

This is an observability improvement for proof runs. It does not expand the
model matrix itself or change inference behavior.
