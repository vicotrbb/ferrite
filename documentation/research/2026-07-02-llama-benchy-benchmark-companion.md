# llama-benchy Benchmark Companion Candidate

## Context

During the Qwen2.5-1.5B Q8_0 prefill/decode theory work, `llama-benchy` was
identified as a possible external benchmark companion for Ferrite's
OpenAI-compatible HTTP server:

- <https://github.com/eugr/llama-benchy>
- <https://pypi.org/project/llama-benchy/>

The project describes itself as a llama-bench style benchmark tool for
OpenAI-compatible endpoints. That makes it relevant to Ferrite because the
server milestone explicitly targets local OpenAI-compatible workflows.

## Potential Fit

`llama-benchy` appears useful for benchmark slices that need:

- OpenAI-compatible `/v1/chat/completions` endpoint coverage;
- context-depth prompt processing measurements;
- token generation measurements;
- prefix-caching experiments;
- JSON or CSV output that can be archived beside Ferrite benchmark notes.

This overlaps with the prefix-reuse theory because Ferrite now has
stream-observed prefill/decode timing for 256, 512, and 1024-token
generated-context runs. `llama-benchy` may give an independent client-side
measurement view over the same OpenAI HTTP surface.

## Constraints

`llama-benchy` should not replace Ferrite's long-chat gate.

The Ferrite gate currently proves project-specific requirements that a generic
benchmark tool does not cover:

- repeated multi-turn generated-context carry-forward;
- reconnect/error behavior;
- disconnect/reconnect behavior;
- `finish_reason` and token-limit behavior;
- usage accounting validity;
- RSS sampling before, after, and idle;
- machine-readable summary markers tied to Ferrite's milestone gates.

Treat `llama-benchy` as a companion measurement tool for performance theories,
not as the authoritative Ferrite correctness gate.

## Proposed First Experiment

Run a small `llama-benchy` smoke against a Ferrite Qwen2.5-1.5B Q8_0
OpenAI-compatible server after the prefix-cache design note is written.

The first useful shape is:

- one bounded amd64 pod on the `staging` Kubernetes context;
- the same Q8_0 model artifact and server settings used by the timing theory
  probes;
- low concurrency;
- context sizes aligned to the existing 256, 512, and 1024 proof set;
- JSON output archived in a benchmark note;
- no optimization claims unless the results agree with Ferrite's own gate or a
  documented explanation accounts for differences.

## Open Questions

- What exact `llama-benchy` CLI options map cleanly to Ferrite's current
  endpoint shape?
- Does the tool count TTFT to first usable content token in the same way as the
  Ferrite timing harness?
- Can it be configured to avoid accidental prefix-cache reuse before Ferrite has
  an explicit prefix-cache implementation?
- What JSON fields should become durable Ferrite benchmark-note table columns?

## Status

Candidate accepted for a future benchmark-tool smoke. No Ferrite benchmark
claim currently depends on `llama-benchy`.
