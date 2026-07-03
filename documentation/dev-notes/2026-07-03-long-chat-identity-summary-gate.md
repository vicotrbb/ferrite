# Long-Chat Identity Summary Gate

Date: 2026-07-03

## Purpose

The response/context identity trace made generated-context carry-forward
observable per scenario. The run summary still required manual log inspection to
prove that each generated response became the next turn's assistant context.

This slice promotes that check into the long-chat gate summary.

## Change

`format_run_summary` now emits:

- `long_chat_summary_generated_context_identity_required`
- `long_chat_summary_generated_context_identity_links`
- `long_chat_summary_matching_generated_context_identity_links`
- `long_chat_summary_all_generated_context_identity_links_present`
- `long_chat_summary_all_generated_context_identities_match_previous_response`

When generated follow-up turns exist, `long_chat_summary_run_complete=true`
requires every generated follow-up turn to have a previous-response-to-current-
assistant-context identity link, and every link must match.

The comparison is lane-aware. It tracks the previous streamed response per
`(model, token_length)` so interleaved 256, 512, and 1024-token scenarios do not
compare against the wrong previous row.

## Validation

The new fixture coverage proves:

- a generated-context run created through the injected runner reports three
  matching identity links across four turns;
- a manually constructed run with generated follow-up turns but missing
  assistant-context identity evidence is incomplete;
- the integrated long-chat summary fixture includes identity evidence and still
  reports `long_chat_summary_run_complete=true`.

## Next Proof Use

Future local or x86 long-chat proof notes should copy the new summary fields
alongside prompt-cache trace fields. A run that reports generated follow-up
turns but lacks matching identity links is not a complete generated-context
proof, even if the cache and timing fields are otherwise present.
