# Q8_K Divergence Diagnostic

Date: 2026-06-28

## Scope

This slice adds and uses a diagnostic comparison mode for the opt-in Q4_K/Q6_K x
Q8_K activation matvec route.

The diagnostic flag is:

```sh
--compare-q8-k-activation-matvec
```

It requires a profiling mode. For each Q4_K/Q6_K profiled matvec, it compares
the default matvec output and the Q8_K activation output using the same input
vector, then prints max absolute and relative deltas.

Note: the original implementation implied
`--experimental-q8-k-activation-matvec`. The later
`documentation/dev-notes/2026-06-29-q8-k-noninvasive-comparison.md` guardrail
decoupled comparison from execution, so comparison-only runs now keep the main
execution policy on `default_only` and compute the Q8_K candidate separately.

## Red-Green Evidence

The CLI test started red because the comparison flag did not exist:

```text
unknown argument --compare-q8-k-activation-matvec
```

The implementation added comparison records to profiled next-token and benchmark
token paths without changing default execution.

## SmolLM2-1.7B Divergent Prefix

The failed opt-in `hello world` sequence was:

```text
expected: 18,198,3725,198,198,788
q8_k:    18,198,198,3272,24,2334
```

The first divergent next-token step is therefore the prefix:

```text
28120,905,18,198
```

Default path:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt-token-ids 28120,905,18,198 --expect-token-id 3725
```

Result:

```text
experimental_q8_k_activation_matvec=false
next_token_id=3725
match=true
```

Q8_K diagnostic path:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt-token-ids 28120,905,18,198 --profile-next-token --compare-q8-k-activation-matvec
```

Result:

```text
experimental_q8_k_activation_matvec=true
compare_q8_k_activation_matvec=true
next_token_id=198
```

The comparison emitted 169 Q4_K/Q6_K matvec records.

Layer 0 already differs:

```text
layer.0.q_proj:Q4_K:2048:2048:2359296:0.113009:68.512711
layer.0.k_proj:Q4_K:2048:2048:2359296:0.115046:30.040007
layer.0.v_proj:Q6_K:2048:2048:3440640:0.029860:300.961487
layer.0.ffn_down:Q6_K:2048:8192:13762560:0.032465:7911.978027
```

The largest absolute differences in that step were:

```text
layer.23.ffn_down:Q4_K:2048:8192:9437184:2.812501:16.362047
output:Q6_K:49152:2048:82575360:1.082870:8145.145508
layer.23.o_proj:Q4_K:2048:2048:2359296:0.939255:8.090786
layer.7.ffn_down:Q6_K:2048:8192:13762560:0.708174:94.337463
layer.22.ffn_down:Q4_K:2048:8192:9437184:0.697522:42.725090
```

Top-logit check for the same prefix:

```text
default top_logits=3725:16.018656,198:15.920707,3272:15.050067,77:13.147774,2334:12.538136
q8_k top_logits=198:16.270929,3725:15.906685,3272:14.135233,2:13.816008,19:13.468991
```

The default margin between `3725` and `198` is narrow enough that accumulated
Q8_K activation drift flips the argmax.

## Qwen2.5-1.5B Working Contrast

The same diagnostic shape on a Qwen2.5-1.5B prefix that still matched:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt-token-ids 14990,1879,198,9707 --profile-next-token --compare-q8-k-activation-matvec
```

Result:

```text
next_token_id=11
```

The comparison emitted 197 Q4_K/Q6_K matvec records. It also had non-zero
differences, including:

```text
layer.0.q_proj:Q4_K:1536:1536:1327104:0.076105:9.169116
layer.0.ffn_down:Q6_K:1536:8960:11289600:0.027084:120.133797
output:Q6_K:151936:1536:191439360:0.569760:6063.106934
```

This means non-zero matvec deltas alone do not prove failure. The failure occurs
when accumulated deltas cross a model-output margin.

## Current Conclusion

The Q8_K path should stay opt-in. The SmolLM failure is not isolated to one
late output projection; the route introduces measurable differences from the
first layer, and those differences accumulate enough to flip a narrow top-logit
margin.

Follow-up boundary probes in
`documentation/dev-notes/2026-06-28-q8-k-smollm-boundary-probes.md` did not find
a localized output-projection or Q6_K-only formula hole. Blocking Q4_K from the
Q8_K route restored the `hello world` prompt but not the EOS-sensitive France
prompt, which supports the activation-drift conclusion rather than a single
kernel arithmetic bug.

Next work should add a role or tolerance policy before any default routing:
either selectively enable Q8_K only where model-output parity is proven, or
tighten the activation quantization/accumulation strategy so SmolLM parity
survives.

Follow-up: `documentation/dev-notes/2026-06-28-q8-k-q6-argmax-options.md`
records a fix for the experimental token-id-only Q6_K output argmax path so it
honors Q8_K activation execution options instead of silently using default Q6_K
argmax semantics.

`documentation/benchmarks/2026-06-28-tier1-q8-k-activation-dot.md` was refreshed
after that fix. Qwen2.5-1.5B still matches both fixed opt-in Q8_K prompts, while
SmolLM2-1.7B still diverges on both fixed opt-in Q8_K prompts and still matches
both fixed default-path prompts on the same current release binary.

## Verification

Focused checks passed for the diagnostic implementation:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
cargo test -p ferrite-cli
cargo test -p ferrite-inference
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p ferrite-cli
```
