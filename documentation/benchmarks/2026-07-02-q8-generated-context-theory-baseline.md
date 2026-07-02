# Qwen 1.5B Q8 Generated-Context Theory Baseline

## Scope

This note extracts a measurement-only baseline from the existing x86_64
`Qwen2.5-1.5B-Instruct-Q8_0` generated-context long-chat proof logs. It supports
the first theory batch in `documentation/theories/` and does not run a new
benchmark, change code, or make an optimization claim.

Source proof logs:

- `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.log`
- `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.log`
- `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.log`

Committed benchmark notes:

- `documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-generated-context-probe-256.md`
- `documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-generated-context-probe-512.md`
- `documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-generated-context-probe-1024.md`

## Extracted Baseline

| Max tokens | Turn | Context | Prompt tokens | Completion tokens | TTFT ms | Total ms | Tok/s | RSS before | RSS after |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 256 | 1 | seed | 43 | 256 | 9930 | 77783 | 3.391947 | 1940291584 | 1940291584 |
| 256 | 2 | generated | 287 | 256 | 72951 | 144228 | 1.807133 | 1940291584 | 1954840576 |
| 256 | 3 | generated | 287 | 256 | 70276 | 141397 | 1.843847 | 1954840576 | 1954840576 |
| 256 | 4 | generated | 282 | 256 | 68826 | 140057 | 1.861732 | 1954840576 | 1955495936 |
| 512 | 1 | seed | 43 | 512 | 9936 | 143687 | 3.621099 | 1956052992 | 1956052992 |
| 512 | 2 | generated | 553 | 512 | 142022 | 304416 | 1.696428 | 1956052992 | 1986985984 |
| 512 | 3 | generated | 543 | 512 | 139712 | 303111 | 1.703776 | 1986985984 | 1986985984 |
| 512 | 4 | generated | 533 | 512 | 139110 | 300110 | 1.720930 | 1986985984 | 1987117056 |
| 1024 | 1 | seed | 43 | 1024 | 10037 | 304614 | 3.387412 | 1995456512 | 1987674112 |
| 1024 | 2 | generated | 1080 | 1024 | 309182 | 738636 | 1.391519 | 1987674112 | 2048884736 |
| 1024 | 3 | generated | 1054 | 1024 | 305399 | 721161 | 1.425325 | 2048884736 | 2060681216 |
| 1024 | 4 | generated | 1054 | 1024 | 304311 | 717430 | 1.432747 | 2060681216 | 2041733120 |

## Aggregates

| Max tokens | Seed TTFT ms | Avg generated prompt tokens | Avg generated TTFT ms | Avg generated tok/s |
| ---: | ---: | ---: | ---: | ---: |
| 256 | 9930 | 285.33 | 70684.33 | 1.837571 |
| 512 | 9936 | 543.00 | 140281.33 | 1.707045 |
| 1024 | 10037 | 1062.67 | 306297.33 | 1.416530 |

## Interpretation

The seed TTFT stays near ten seconds across the three Q8_0 proof lengths because
the seed prompt remains `43` tokens. Generated-context TTFT rises as the
follow-up prompt grows:

- 256-token proof: generated prompts average about `285` tokens and TTFT
  averages about `70.7s`;
- 512-token proof: generated prompts average about `543` tokens and TTFT
  averages about `140.3s`;
- 1024-token proof: generated prompts average about `1063` tokens and TTFT
  averages about `306.3s`.

This supports the `long-chat-prefix-reuse` and `generated-context-windowing`
theories as high-value next experiments. It does not prove that prefix reuse or
windowing will improve Ferrite; it only shows that generated-context follow-up
turns are currently dominated by prompt growth and first-token latency.

RSS growth during these single proof runs was bounded, but this baseline does
not prove steady-state memory behavior. The `kv-cache-memory-pressure` theory
still needs repeated-session measurement in one server process.

## Next Measurement

Add a measurement-only timing split that records prefill elapsed time separately
from decode elapsed time for seed and generated-context turns. If prefill time
tracks prompt-token count, the next implementation design should target token
prefix identity and bounded KV prefix reuse. If prefill is not dominant, the
next design should move toward decode/kernel profiling instead.
