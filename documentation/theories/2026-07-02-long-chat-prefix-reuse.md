# Theory: Long-Chat Prefix Reuse

Date: 2026-07-02

Status: Hypothesis

## Hypothesis

Ferrite's generated-context long-chat slowdown is dominated by reprocessing the
same conversation prefix on every follow-up request. Reusing the KV cache for
the unchanged prefix across repeated chat turns should reduce time to first
token for turns 2 and later without changing generated text, usage accounting,
or OpenAI-compatible response shape.

## Mechanism

The Qwen2.5-1.5B Q8_0 x86_64 generated-context proof shows a sharp TTFT jump
when the prompt grows from the seed turn to generated-context follow-up turns:

| Max tokens | Turn | Context | Prompt tokens | TTFT ms | Tok/s | RSS before |
| ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 256 | 1 | seed | 43 | 9930 | 3.391947 | 1940291584 |
| 256 | 2 | generated | 287 | 72951 | 1.807133 | 1940291584 |
| 512 | 1 | seed | 43 | 9936 | 3.621099 | 1956052992 |
| 512 | 2 | generated | 553 | 142022 | 1.696428 | 1956052992 |
| 1024 | 1 | seed | 43 | 10037 | 3.387412 | 1995456512 |
| 1024 | 2 | generated | 1080 | 309182 | 1.391519 | 1987674112 |

The follow-up turns preserve the previous assistant output as generated context,
so a large prefix is identical between adjacent turns. If Ferrite recomputes the
entire prompt for each request, TTFT scales with total prompt length. A
session-level prefix cache could avoid recomputing K/V states for the stable
prefix and only evaluate the newly appended user/assistant suffix.

## Expected Measurement

This theory is worth pursuing if a measurement-only probe confirms that TTFT is
approximately proportional to prompt-token growth while per-token decode latency
stays in a narrower range. For the current Q8_0 1024 proof, the first useful
target is reducing generated follow-up TTFT by at least 30 percent without
increasing RSS by more than 10 percent after idle.

The first implementation-worthy proof would show:

- same prompt-token and completion-token accounting as the baseline;
- same `finish_reason` behavior for length-limited runs;
- same generated-context source reporting for turns 2-4;
- lower TTFT for turns 2-4 on a repeated generated-context run;
- bounded RSS after idle, with no persistent growth across repeated sessions.

## Falsification Experiment

Before implementing a cache, run a measurement-only baseline that separates
prefill time from decode time for seed and generated-context turns. This can be
done by extending or wrapping the existing long-chat gate to record:

- prompt-token count;
- prefill elapsed time before first streamed token;
- first-token timestamp;
- decode elapsed time after first token;
- RSS before request, after request, and after idle.

The theory is falsified for the current milestone if generated follow-up TTFT is
not materially tied to prompt-token growth, or if a prototype prefix cache
reduces TTFT by less than 10 percent while adding meaningful memory retention or
complex session invalidation risk.

## Risks

- Prefix identity is subtle for chat templates, system messages, tool fields,
  stop sequences, sampling settings, and tokenizer control tokens.
- OpenAI-compatible clients do not expose a stable session identifier by
  default, so server-side cache keys must be explicit and conservative.
- KV reuse can silently corrupt generation if the cached prefix is matched at
  the string level instead of token level.
- Keeping K/V states across requests may hurt memory-fit goals unless eviction
  is strict and observable.

## Next Step

Create a measurement-only benchmark note for the current Q8_0 x86_64
generated-context logs. Then add a focused prefill/decode timing probe before
any cache implementation. If the timing split confirms prefix reprocessing as
the dominant cost, design a small token-prefix identity layer and a bounded
per-model KV prefix cache as a separate implementation slice.
