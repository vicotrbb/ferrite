# 2026-06-27 Tier 0 SmolLM2 360M Probe

## Scope

This slice extends Tier 0 evidence from the local SmolLM2-135M probe to the
larger Tier 0 SmolLM2-360M-Instruct Q4_K_M GGUF artifact.

It is a validation and evidence slice. It does not change Ferrite runtime code.

## Model

- Repo: `bartowski/SmolLM2-360M-Instruct-GGUF`
- File: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Local file size: 270,590,880 bytes
- Hugging Face repo revision recorded by local cache:
  `7be6f65f1db715fe5dc5a4634c0d459b4eed42ec`
- Hugging Face blob id recorded by local cache:
  `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`

Download command:

```sh
huggingface-cli download bartowski/SmolLM2-360M-Instruct-GGUF SmolLM2-360M-Instruct-Q4_K_M.gguf --local-dir target/models --max-workers 1
```

## Ferrite Probe

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --stream
```

Output:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
stream_token_id=18
stream_text="
stream_token_id=284
stream_text= and
stream_token_id=476
stream_text= "
stream_token_id=28120
stream_text=hello
stream_token_id=905
stream_text= world
stream_token_id=18
stream_text="
generated_cached_tokens=8
generated_token_ids=18,284,476,28120,905,18
generated_text=" and "hello world"
model_file_bytes=270590880
model_file_retained_bytes=0
scalar_weight_bytes=268803840
kv_cache_bytes=655360
```

## Reference Comparison

`llama.cpp` local tools:

- `target/reference/llama.cpp/build/bin/llama-simple`
- `target/reference/llama.cpp/build/bin/llama-tokenize`

Greedy default `llama-simple` output for six tokens was:

```text
hello world" and "hello world"
```

The continuation text tokenizes to the same generated token IDs Ferrite
reported:

```sh
printf '" and "hello world"' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[18, 284, 476, 28120, 905, 18]
```

## Caveat

Forcing `llama-simple` with `-ngl 0` produced a different greedy continuation
for the same prompt:

```text
hello world"
print(word)
```

Tokenizing that continuation produced:

```text
[18, 198, 3272, 24, 3002, 25]
```

This means the 360M probe proves Ferrite can load, run, stream, and decode the
larger Tier 0 model, and that it matches one local `llama.cpp` greedy reference
path. It does not close the CPU-only reference parity question for SmolLM2-360M.

## Result

- GGUF parser loads the 360M Q4_K_M model.
- Scalar forward pass produces output.
- CLI streaming mode works for the 360M model.
- Multi-token IDs match the default local `llama.cpp` greedy run after
  tokenizing the generated continuation.
- CPU-only `llama.cpp -ngl 0` parity remains unproven and needs a follow-up
  investigation before Tier 0 is marked complete.
