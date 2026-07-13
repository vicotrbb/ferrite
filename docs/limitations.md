# Current limitations

Ferrite is alpha software with an intentionally narrow compatibility surface.
The following boundaries are part of the current contract.

## Models and generation

- Only `llama` and `qwen2` GGUF architectures are supported.
- GGUF version 3 is required.
- Inference supports F32, F16, BF16, Q4_K, Q5_0, Q6_K, and Q8_0 tensors.
- Greedy decoding is the only sampling policy.
- Ferrite uses its own focused chat rendering, not arbitrary model-provided
  Jinja templates.
- The practical model and context limit depends on host memory and the selected
  KV backend. Large-model coverage is not complete.

## API and serving

- The server loads one model per process.
- OpenAI compatibility covers models, legacy completions, and chat
  completions, not the Responses API.
- Tool execution, multimodal input, audio, embeddings, fine tuning, hosted
  files, and structured output generation are not implemented.
- TLS, durable access logs, internet-facing rate limiting, tenant isolation,
  and process sandboxing must be supplied by the deployment boundary.
- Prefix caching, residual activation kernels, and continuous batching are
  experimental and opt-in.

## Platforms

- Maintained CI targets are 64-bit Linux on x86_64 and macOS on Apple Silicon.
- Optimized aarch64 paths use NEON, DotProd, and I8MM where runtime detection
  confirms support.
- Optimized x86_64 paths require AVX2. Unsupported features fall back to the
  portable implementation where one exists.
- Windows and other architectures are not currently claimed as supported.

## Stability

- The publishable crates are version `0.2.0` and can evolve before 1.0.
- CLI `key=value` names, HTTP response shapes, and experimental flags are
  treated as compatibility surfaces, but can change with release notes.
- Performance numbers are evidence for a named model, build, machine, and
  execution policy, not universal guarantees.

Use [models and tensor formats](models.md),
[OpenAI API compatibility](openai-api.md), and
[the performance golden path](performance.md) for the exact supported path.
