# Q8_K Qwen2.5-1.5B Divergence Profile

Date: 2026-06-28

## Scope

This note profiles the all-role experimental Q8_K divergence recorded in
`documentation/dev-notes/2026-06-28-q8-k-qwen-1-5b-parity-probe.md`.

The failed prompt was:

```text
Machine learning models can
```

Default generated:

```text
387,1483,311,7023,279,28636
```

Experimental all-role Q8_K generated:

```text
387,16176,389,264,8045,315
```

The divergent prefix is therefore the prompt tokens plus first generated token:

```text
21605,6832,4119,646,387
```

## Default Profile

```text
q8_k_activation_matvec_policy=default_only
next_token_id=1483
top_logits=1483:21.185814,16176:20.841335,21091:19.790606,17779:19.504631,26075:19.173910
```

Default chooses token `1483` over token `16176` by about `0.344479`.

## Experimental Q8_K Profile

```text
q8_k_activation_matvec_policy=experimental_parity_scoped
q8_k_activation_matvec_roles=all
next_token_id=16176
top_logits=16176:20.969004,1483:20.905596,21091:19.688211,17779:19.472042,26075:19.429556
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795129:9687.982422
```

Experimental Q8_K chooses token `16176` over token `1483` by about `0.063408`.

Selected layer-level comparison highlights from the same profile:

```text
profile_next_token_q8_k_compare=layer.1.ffn_up:Q4_K:8960:1536:7741440:2.594692:345.131897
profile_next_token_q8_k_compare=layer.27.ffn_down:Q6_K:1536:8960:11289600:0.310311:14.980375
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795129:9687.982422
```

## Conclusion

The Qwen2.5-1.5B Q8_K parity failure is another narrow-margin activation-drift
case. The default margin for the expected token is only about `0.344479`, and
all-role Q8_K shifts the top pair enough to invert the decision.

This supports the current policy boundary: Q8_K remains useful for opt-in
research and benchmark experiments, but it is not eligible for default dispatch
without a tighter activation quantization strategy or stronger parity evidence.
