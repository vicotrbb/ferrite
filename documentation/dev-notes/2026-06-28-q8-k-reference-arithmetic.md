# Q8_K Reference Arithmetic Audit

Date: 2026-06-28

## Scope

This note audits Ferrite's internal Path B arithmetic against the local
`llama.cpp` reference checkout under `target/reference/llama.cpp`.

Path B is the ADR 0007 activation-side `q8_K` dot path for Q4_K and Q6_K
matvecs. The open question after the SmolLM2 divergence diagnostic was whether
the Q8_K route had an arithmetic design hole, or whether the observed model
divergence was expected activation-quantization drift.

## Reference Findings

`quantize_row_q8_K_ref` in llama.cpp:

- chooses the signed value with the largest absolute magnitude;
- uses `iscale = -127.f / max`;
- writes 256 signed activation quants;
- stores 16 sums over 16 quantized values;
- stores `d = 1 / iscale`.

Ferrite's `BlockQ8K` follows the same contract. Ferrite also clamps the lower
bound to `-127`; with the reference scale selection, valid values are already in
range, so the extra clamp is defensive rather than a semantic change.

The follow-up guard `test: pin q8 k activation contract` pins this contract in
`crates/ferrite-inference/src/scalar/q8_k.rs` for positive-dominant,
negative-dominant, and all-zero activation blocks. These tests assert the sign
of `d`, representative quantized lanes, and the 16-wide group sums used by the
Q4_K/Q6_K x Q8_K dot identities below.

The generic Q4_K x Q8_K reference computes:

```text
y.d * (x.d * sum(scale[j] * dot(q4[j], q8[j]))
       - x.dmin * sum(min[j] * q8_group_sum[j]))
```

Ferrite's Q4_K NEON helper uses the same identity:

```text
activation.d * (d * weighted_sum - dmin * min_sum)
```

The generic Q6_K x Q8_K reference decodes Q6 lanes as signed values by
subtracting 32 before the scaled dot. llama.cpp's ARM NEON route uses the
equivalent split form:

```text
x.d * y.d * (sum(scale[j] * dot(q6_raw[j], q8[j]))
             - 32 * sum(scale[j] * q8_group_sum[j]))
```

Ferrite's Q6_K NEON helper uses that same split form:

```text
activation.d * super_scale * (weighted_sum - 32 * correction_sum)
```

## Source Anchors

- `target/reference/llama.cpp/ggml/src/ggml-quants.c:2696`
  `quantize_row_q8_K_ref`
- `target/reference/llama.cpp/ggml/src/ggml-cpu/quants.c:645`
  `ggml_vec_dot_q4_K_q8_K_generic`
- `target/reference/llama.cpp/ggml/src/ggml-cpu/quants.c:800`
  `ggml_vec_dot_q6_K_q8_K_generic`
- `target/reference/llama.cpp/ggml/src/ggml-cpu/arch/arm/quants.c:2715`
  ARM NEON Q4_K x Q8_K route
- `target/reference/llama.cpp/ggml/src/ggml-cpu/arch/arm/quants.c:3418`
  ARM NEON Q6_K x Q8_K route

## Conclusion

No reference-arithmetic hole was found in Ferrite's Path B design.

The Q8_K activation quantizer, Q4_K formula, and Q6_K formula line up with
llama.cpp's generic and ARM NEON contracts. Ferrite's tests pin the quantizer
contract and compare target-specific Q8_K helpers against the scalar Q8_K
adapters. The SmolLM2 failures documented in
`documentation/dev-notes/2026-06-28-q8-k-divergence-diagnostic.md` therefore
remain best explained as activation-quantization drift crossing a narrow output
margin, not as a proven Q4_K/Q6_K x Q8_K formula bug.

Path B is sound as an internal, opt-in research path. It is not sound as a
default dispatch path until real model-output parity is proven for the target
model family or the activation quantization strategy is tightened enough to
restore that parity.
