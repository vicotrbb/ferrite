# Theory: BPE Tokenizer Throughput

Date: 2026-07-03

Status: Locally validated for tokenizer-only and OpenAI server tokenization stage

## Hypothesis

Ferrite's current simple BPE encode loop is the dominant remaining latency
source before long-prompt prompt evaluation begins. Replacing repeated whole
merge-list scans with an adjacent-pair rank strategy, and optionally caching
byte-seeded prompt-prefix symbols, may reduce long-prompt tokenization latency
enough to materially improve abandoned-request cancellation and long-chat
first-token latency.

## Mechanism

The current optimization moved token-id and merge metadata parsing out of the
per-request encode path. The local Qwen2.5-0.5B proof still measured an
approximately eight-second long-prompt encode. That suggests the next useful
work is not more metadata setup cleanup, but the merge algorithm itself.

Candidate mechanisms:

- build a pair-rank map from BPE merge metadata and repeatedly merge the best
  adjacent ranked pair instead of scanning all merge rules across the whole
  symbol list;
- cache byte-seeded symbols or partially merged symbols for stable long-chat
  prefixes;
- expose repeated tokenizer benchmark runs in one process to distinguish
  tokenizer load cost from encode cost.

## Expected Measurement

A worthwhile result should show the same long prompt and model producing the
same token IDs while reducing tokenizer-only CLI latency by a meaningful margin.
The immediate baseline is:

```text
tokenization_benchmark_total_ns=8188082917
tokenization_benchmark_token_count=19428
```

The comparable server lifecycle field after BPE metadata preparse was:

```text
prompt_tokenized_elapsed_ms=8323
```

After split timing was added to the tokenizer-only CLI benchmark, a
same-size local prompt sample reported:

```text
tokenization_benchmark_gguf_parse_ns=13452375
tokenization_benchmark_tokenizer_load_ns=86463500
tokenization_benchmark_encode_avg_ns=6792609791
tokenization_benchmark_token_count=29527
```

That places the dominant local cost in BPE encode, not GGUF parse or tokenizer
load.

The next tokenizer-only proof added a compact token-id parity signal for the
same prompt generator:

```text
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
```

Any alternate BPE encode path must preserve both values before its timing can
be compared.

The first active adjacent-pair rank loop preserved that parity target and
reduced tokenizer-only average encode time on the same generated prompt:

```text
before tokenization_benchmark_encode_avg_ns=6894344416
after  tokenization_benchmark_encode_avg_ns=4062045166
tokenization_benchmark_token_count=29527
tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0
```

That is about a `41.08%` local improvement for this sample.

For this theory to remain worth pursuing, a first algorithm experiment should
reduce the CLI tokenizer-only average by at least 20 percent on the same prompt
without changing token IDs.

## Falsification Experiment

Implement the smallest alternate BPE encode path behind tests and compare it
against the current path:

- fixture-token parity on representative BPE inputs, including multibyte UTF-8;
- malformed metadata behavior unchanged at tokenizer load;
- one-process CLI tokenizer benchmark on the long Qwen prompt;
- server lifecycle rerun only if the CLI path improves.

The active-pair scan passed the first falsification gate: parity held and the
isolated CLI benchmark improved by more than 20 percent. The next falsification
gate is server lifecycle proof. If `prompt_tokenized_elapsed_ms` does not move
meaningfully in the OpenAI path, the CLI improvement is not enough by itself.

The OpenAI server lifecycle proof then passed that gate:

```text
before prompt_tokenized_elapsed_ms=8323
after  prompt_tokenized_elapsed_ms=4331
```

That is about a `47.96%` local improvement for the same-size server-stage
probe. This validates the active-pair scan as a real request-path tokenizer
improvement for the local Qwen 0.5B model.

## Risks

- A naive priority queue can spend the saved merge-scan time on queue
  maintenance and stale-pair invalidation.
- Prefix symbol caching can increase memory use or become invalid if chat
  template normalization changes the prompt bytes.
- Tokenizer parity bugs are subtle and can silently corrupt all downstream
  prompt evaluation.
- One local Qwen 0.5B result may not represent other GGUF BPE tokenizers.

## Next Step

Run the dedicated long-chat gate with 256, 512, and 1024-token streaming
responses. After that, test whether a priority-queue active-pair structure
improves beyond the current scan loop without increasing memory too much.
