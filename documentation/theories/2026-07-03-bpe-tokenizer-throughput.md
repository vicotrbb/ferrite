# Theory: BPE Tokenizer Throughput

Date: 2026-07-03

Status: Testing

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

If parity requires excessive special casing or the isolated CLI benchmark does
not improve by at least 20 percent, park the algorithm rewrite and prioritize
prompt prefill or cache reuse instead.

## Risks

- A naive priority queue can spend the saved merge-scan time on queue
  maintenance and stale-pair invalidation.
- Prefix symbol caching can increase memory use or become invalid if chat
  template normalization changes the prompt bytes.
- Tokenizer parity bugs are subtle and can silently corrupt all downstream
  prompt evaluation.
- One local Qwen 0.5B result may not represent other GGUF BPE tokenizers.

## Next Step

First add a repeated-run tokenizer benchmark path so load cost and encode cost
can be separated. Then prototype the adjacent-pair rank algorithm behind token
parity tests before making it the default.
