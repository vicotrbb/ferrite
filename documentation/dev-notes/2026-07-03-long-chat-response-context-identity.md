# Long-Chat Response Context Identity

Date: 2026-07-03

## Purpose

The generated-context fixed-point cache theory needs a way to compare what a
long-chat turn generated with what the next turn used as assistant context,
without printing full model output into proof logs.

This slice adds deterministic, non-disclosing text identity fields to the proof
tooling:

- `long_chat_result_assistant_context_bytes`
- `long_chat_result_assistant_context_hash`
- `long_chat_result_generated_response_bytes`
- `long_chat_result_generated_response_chunks`
- `long_chat_result_generated_response_hash`

The hash format is `fnv64:{016x}` and is intended only for local diagnostics,
not cryptographic comparison.

## Validation

Fixture validation covers:

- deterministic streaming text identity for chunked SSE text;
- long-chat scenario formatting for context and generated response identity;
- runner-level propagation of the exact assistant context identity across
  repeated generated-context turns and token lengths.

## Next Proof Use

The next real-model long-chat trace should compare:

- turn `N` generated response hash;
- turn `N+1` assistant context hash;
- prompt-cache prompt hash and selected-entry hash;
- TTFT and cached prompt token count.

If turn `N` generated response hash equals turn `N+1` assistant context hash,
the carry-forward behavior is directly confirmed. If the context hashes differ
but the prompt token hash still repeats, the next theory to test is prompt
rendering or tokenization normalization.
