# Current limitations

Ferrite is alpha software with an intentionally narrow compatibility surface.
The following boundaries are part of the current contract.

## Models and generation

- Only `llama`, `qwen2`, and `phi3` GGUF architectures are supported.
- GGUF version 3 is required.
- Inference supports F32, F16, BF16, Q4_K, Q5_0, Q5_K, Q6_K, and Q8_0
  tensors. Q5_K has no architecture-specific optimized kernel.
- Greedy and common seeded sampling controls are supported. Sampled requests
  do not yet enter the experimental continuous batch scheduler.
- Ferrite recognizes bounded Qwen ChatML, Llama 3, Llama 2, and Phi-3 template
  families from GGUF metadata. It never executes arbitrary model-provided
  Jinja, and other templates use the documented fallback renderer.
- The practical model and context limit depends on host memory and the selected
  KV backend. Large-model coverage is not complete.
- Ferrite bounds untrusted-input allocations and configured KV capacity, but it
  does not provide a global process memory governor or claim recovery after host
  allocator exhaustion.
- The optional server Locus backend bounds each session with fixed-size blocks,
  but its pool is per session. Prefix snapshots are shared immutably and copied
  into each request's independently mutable KV session on restore. Locus copies
  borrowed rows directly into mapped blocks; the vector backend still clones
  the owned snapshot because it becomes that session's storage.

## API and serving

- The server loads one model per process.
- OpenAI compatibility covers models, legacy completions, chat completions,
  and a bounded non-streaming text subset of `POST /v1/responses`.
- Chat function definitions and tool-call parsing require a Qwen-compatible
  ChatML template. Tool streaming and tool use through the Responses endpoint
  are not supported. Ferrite never executes a parsed tool call.
- Chat JSON-object mode uses a bounded grammar. JSON Schema constrained output
  and streaming structured output are not implemented.
- Multimodal input, audio, embeddings, fine tuning, and hosted files are not
  implemented.
- TLS, durable access logs, internet-facing rate limiting, tenant isolation,
  and process sandboxing must be supplied by the deployment boundary.
- Prefix caching, residual activation kernels, and continuous batching are
  experimental and opt-in.
- SVE2, SME2, AVX-VNNI, AVX512-VNNI, AMX, KleidiAI, oneDNN, and NUMA binding
  are not execution paths. Some CPU capabilities are reported only to make
  future experiments reproducible.
- Continuous batching covers fused-greedy streaming and non-streaming requests,
  including prefix-cache and trace requests. Sampling policies that require full
  logits remain on the normal path.

## Platforms

- Maintained native CI targets are Linux x86_64, Linux arm64, macOS arm64,
  macOS x86_64, and Windows x86_64. Release archives currently cover macOS
  arm64 and Linux x86_64 only.
- Optimized aarch64 paths use NEON, DotProd, and I8MM where runtime detection
  confirms support.
- Optimized x86_64 paths require AVX2; the F16 dense path also requires F16C.
  Unsupported features fall back to the portable implementation where one
  exists.
- Windows x86_64 is a source-build correctness target. No Windows release
  archive or retained native real-model performance artifact is claimed.

## Stability

- The publishable crates are version `0.2.0` and can evolve before 1.0.
- CLI `key=value` names, HTTP response shapes, and experimental flags are
  treated as compatibility surfaces, but can change with release notes.
- Performance numbers are evidence for a named model, build, machine, and
  execution policy, not universal guarantees.

Use [models and tensor formats](models.md),
[OpenAI API compatibility](openai-api.md), and
[the performance golden path](performance.md) for the exact supported path.
