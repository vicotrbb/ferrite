# Troubleshooting

## The model is rejected during loading

Check the first explicit error. Common causes are:

- GGUF version is not 3.
- `general.architecture` is not `llama` or `qwen2`.
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

Ferrite rejects options that request behavior it does not implement. Remove
sampling, tool, audio, structured-output, or other non-neutral options. See
[OpenAI API compatibility](openai-api.md).

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

Build the CLI with the feature:

```sh
cargo build --release --locked -p ferrite-cli --features locus-kv
```

Then verify that `--kv-max-tokens` is large enough for prompt and generated
tokens. Omit it to use Ferrite's workload-based sizing.

## A real-model test is ignored

Ignored tests require local model artifacts. Set the environment variable named
in the test, then run that test target with `--ignored`. See
[evaluation and regression gates](evaluation.md).
