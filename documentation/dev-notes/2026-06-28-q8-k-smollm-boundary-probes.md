# Q8_K SmolLM Boundary Probes

Date: 2026-06-28

## Scope

This note records temporary, uncommitted boundary probes for the SmolLM2-1.7B
Q8_K activation divergence. The probes were used only to isolate the failure
shape. The source tree was restored after each probe.

The question was whether Path B has a localized implementation hole, such as a
final output projection issue or a Q6_K-only formula problem, or whether the
documented divergence is accumulated activation-quantization drift.

## Baseline Boundaries

For `hello world`, default and Q8_K both emit the first two generated tokens:

```text
18,198
```

The first divergent prefix is:

```text
28120,905,18,198
```

Default logits choose token `3725` over token `198` by a narrow margin:

```text
default top_logits=3725:16.018656,198:15.920707,3272:15.050067
q8_k top_logits=198:16.270929,3725:15.906685,3272:14.135233
```

For `The capital of France is`, default and Q8_K both emit:

```text
7042,30
```

The first divergent prefix is:

```text
504,3575,282,4649,314,7042,30
```

Default logits choose EOS token `2` over token `198`, also by a narrow margin:

```text
default top_logits=2:14.294987,198:14.115829,378:12.799028
q8_k top_logits=198:14.161620,2:14.070060,378:12.680813
```

## Probe Matrix

These probes were run by temporarily changing the Q8_K dispatch policy and
rebuilding `ferrite-cli`. None of these changes were retained.

| Probe | `hello world` result | `The capital of France is` result | Interpretation |
| --- | --- | --- | --- |
| Force final output projection back to default while hidden layers use Q8_K | Still diverged: `18,198,198,3272,24,2334` | Still diverged: `7042,30,198,504,3575,282` | The failure is not isolated to final output projection Q8_K. |
| Block Q6_K from Q8_K while Q4_K still uses Q8_K | Still diverged: `18,198,198,19,21367,42` | Still diverged: `7042,30,198,504,3575,282` | The failure is not a Q6_K-only formula hole. |
| Block Q4_K from Q8_K while Q6_K still uses Q8_K | Matched: `18,198,3725,198,198,788` | Still diverged: `7042,30,198,504,3575,282` | Q4_K drift is enough to cross the first prompt's narrow margin, but the EOS prompt remains sensitive to Q6_K drift. |

## Conclusion

No localized Path B arithmetic or dispatch hole was found by these probes.
Combined with `documentation/dev-notes/2026-06-28-q8-k-reference-arithmetic.md`,
the current evidence supports this split:

- Path B is sound as an internal, opt-in Q4_K/Q6_K x Q8_K kernel-contract path.
- The current all-eligible-matrices execution policy is not parity-safe across
  Tier 1 models.
- SmolLM2-1.7B fails because activation-quantization drift accumulates across
  prompt-sensitive hidden states and crosses narrow output margins.

The next correctness-focused design should not be another formula tweak. It
should either add a deliberate role/model parity policy for Q8_K activation
dispatch or tighten the activation quantization strategy enough to preserve
SmolLM2 parity.
