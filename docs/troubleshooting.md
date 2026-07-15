# Troubleshooting

## The model is rejected during loading

Check the first explicit error. Common causes are:

- GGUF version is not 3.
- `general.architecture` is not `llama`, `qwen2`, or `phi3`.
- A required tensor is missing, duplicated, misaligned, or has the wrong shape.
- A tensor uses an unsupported encoding.
- Model metadata has an invalid head, RoPE, context, or feed-forward layout.
- A scale or dense value is not finite.

See [models and tensor formats](models.md) for the current contract.

## The server says `ready: false`

Start the server with `--model <path>`. Confirm that the process logged no load
error and that the path is readable by the server user. `/health` is available
without authentication and reports readiness independently from `/v1/*` auth.

## A request returns 401

When `--api-key` is configured, send:

```text
Authorization: Bearer <exact-key>
```

Do not put the key in a URL or commit it to scripts.

## A request returns 429

The inference permit was unavailable for longer than `--inference-wait-ms`.
Increase the wait if queueing is acceptable, reduce client concurrency, or
measure experimental batching. Raising concurrent inference permits can worsen
memory-bandwidth contention.

## A request reports unsupported fields

Ferrite rejects options that request behavior it does not implement. Check the
error `param` before removing a field. Sampling, ChatML function calls,
JSON-object mode, and non-streaming Responses text each have documented narrow
forms; audio, multimodal data, hosted tools, and non-neutral Responses state do
not. See [OpenAI API compatibility](openai-api.md).

## Built-in model acquisition fails

Confirm that `curl` is installed and HTTPS access to the pinned model source is
allowed. A failed transfer leaves a `.partial` file for the next invocation to
resume. Do not rename a partial file into place. Ferrite publishes the final
artifact only after its exact byte size and SHA-256 match.

If a stale acquisition lock remains after a terminated process, the error
reports its exact path. Confirm no acquisition process is active before
removing only that `.acquire.lock` file. Use `--offline` when network access is
prohibited; an absent or corrupted cache entry will then fail explicitly.

## Performance is much lower than a recorded result

Confirm all of the following:

- release binary, exact commit, and locked dependency graph;
- same model file hash, quantization, prompt, and token count;
- same execution policy and worker count;
- supported CPU feature, especially I8MM for the residual Arm path;
- stable power, thermal, and background-load conditions;
- no debug build, profiler, or concurrent eval affecting the run.

Follow [the performance golden path](performance.md) and compare repeated-run
medians.

## `--kv-backend locus` fails

Build the selected binary with the feature:

```sh
cargo build --release --locked -p ferrite-cli --features locus-kv
cargo build --release --locked -p ferrite-server --features locus-kv
```

For the CLI, verify that `--kv-max-tokens` is large enough for prompt and
generated tokens, or omit it to use workload-based sizing. The server requires
an explicit `--kv-max-tokens` value and rejects requests whose prompt plus
worst-case decode state cannot fit. The error is a configured-capacity failure,
not permission to fall back to unbounded vector storage.

## A real-model test is ignored

Ignored tests require local model artifacts. Set the environment variable named
in the test, then run that test target with `--ignored`. See
[evaluation and regression gates](evaluation.md).
