# Q8_K Qwen Argmax Rerun

Date: 2026-06-29

## Scope

This reruns the Qwen2.5-1.5B Q4_K_M Path B divergence point after adding
argmax-index reporting to Q8_K activation-matvec comparison output.

The divergent prefix from
`documentation/dev-notes/2026-06-28-q8-k-qwen-1-5b-divergence-profile.md` is:

```text
21605,6832,4119,646,387
```

This is the failed prompt plus the first generated token for:

```text
Machine learning models can
```

## Comparison-Only Default Execution

Command:

```sh
cargo run --release -p ferrite-cli -- --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt-token-ids 21605,6832,4119,646,387 --profile-next-token --compare-q8-k-activation-matvec --top-logits 5
```

Selected output:

```text
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=1483
profile_next_token_q8_k_compare=layer.1.ffn_up:Q4_K:8960:1536:7741440:1.907654:1209.581787:917:917
profile_next_token_q8_k_compare=layer.27.ffn_down:Q6_K:1536:8960:11289600:0.232225:24.918306:1421:1421
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795508:162048.156250:1483:1483
top_logits=1483:21.185814,16176:20.841335,21091:19.790606,17779:19.504631,26075:19.173910
```

Interpretation:

- The main path stays on `default_only`.
- The default path chooses token `1483`.
- The output projection comparison on the default hidden state has matching
  reference and Q8_K candidate argmax indexes: `1483:1483`.
- This means the output projection Q8_K candidate alone does not flip the final
  decision when the hidden state came from default execution.

## Experimental Q8_K Execution

Command:

```sh
cargo run --release -p ferrite-cli -- --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt-token-ids 21605,6832,4119,646,387 --profile-next-token --experimental-q8-k-activation-matvec --compare-q8-k-activation-matvec --top-logits 5
```

Selected output:

```text
experimental_q8_k_activation_matvec=true
compare_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=experimental_parity_scoped
q8_k_activation_matvec_roles=all
next_token_id=16176
profile_next_token_q8_k_compare=layer.1.ffn_up:Q4_K:8960:1536:7741440:2.594692:345.131897:917:917
profile_next_token_q8_k_compare=layer.27.ffn_down:Q6_K:1536:8960:11289600:0.310311:14.980375:1421:1421
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795129:9687.982422:1483:16176
top_logits=16176:20.969004,1483:20.905596,21091:19.688211,17779:19.472042,26075:19.429556
```

Interpretation:

- Experimental all-role Q8_K execution chooses token `16176`.
- On the experimental hidden state, the output projection comparison reports
  reference argmax `1483` and Q8_K candidate argmax `16176`.
- This directly marks the final decision flip that was previously inferred
  from top-logit changes.

## Conclusion

The new argmax diagnostic supports the current Path B boundary. The Qwen2.5
failure is not explained by a standalone output-projection formula hole on the
default hidden state. The comparison-only run keeps the output argmax stable.
The experimental run shows accumulated Q8_K activation drift reaches the output
projection with enough state change for the Q8_K candidate to flip the final
argmax.

Path B should therefore remain experimental until a tighter activation strategy
or a narrower role policy has real model-output parity evidence.
