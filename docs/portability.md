# CPU portability and dispatch

Ferrite ships one portable correctness provider and selectively enters proven
SIMD kernels at runtime. A binary built on one supported CPU must not assume
that another CPU exposes the same optional instructions.

## Provider policy

`auto` is the default. It detects features through Rust's architecture-specific
runtime macros before calling a target-feature function. `portable` disables
optimized entries and is intended for parity checks, incident diagnosis, and
unusual virtualized environments:

```sh
target/release/ferrite \
  --model model.gguf \
  --prompt 'Explain local inference in one sentence.' \
  --kernel-provider portable
```

Both binaries print `kernel_provider` and `cpu_features` at startup. The feature
list describes the CPU, not the set of Ferrite kernels.

| Architecture | Capability | Ferrite status |
| --- | --- | --- |
| all | portable scalar | implemented and forceable |
| Arm64 | NEON | automatic proven kernels |
| Arm64 | DotProd and I8MM | runtime-gated experimental residual activation kernels |
| Arm64 | SVE2 and SME2 | investigated, no Ferrite kernel selected |
| x86_64 | AVX2 | automatic proven kernels |
| x86_64 | F16C | runtime-gated F16 conversion with AVX2 |
| x86_64 | AVX-VNNI and AVX512-VNNI | detected for diagnostics, no Ferrite kernel selected |
| x86_64 | AMX | investigated, no Ferrite kernel selected |

Rust documents that architecture intrinsics require both target selection and a
runtime feature guard before an optional instruction is called. Ferrite keeps
that guard in one provider boundary. See Rust's [CPU feature detection
guidance](https://doc.rust-lang.org/stable/core/arch/) and the supported
[Arm64 feature names](https://doc.rust-lang.org/std/arch/macro.is_aarch64_feature_detected.html).

## Thread defaults

An explicit `--threads` value always wins, followed by `FERRITE_THREADS` and
`RAYON_NUM_THREADS`.

Without an override:

- macOS uses the largest homogeneous performance level;
- Linux respects the process CPU allow-list, prefers the highest-capacity core
  class on heterogeneous hosts, and otherwise avoids counting sibling hardware
  threads as independent physical cores;
- other systems use the standard library's available parallelism;
- the server reserves one automatic worker-sized CPU slot for HTTP work when
  inference would otherwise consume the full recommendation.

These are conservative defaults, not universal performance maxima. Use the eval
harness to compare fixed thread counts on the actual model and machine.

## Platform evidence

The CI workflow is configured for native Linux x86_64, Linux arm64, macOS
arm64, macOS x86_64, and Windows x86_64. A separate macOS arm64 job runs the
x86_64 provider parity test through Rosetta. A configured job becomes verified
evidence only after its workflow run passes.

On 2026-07-13, the milestone working tree based on commit
`33a11d0be6e2417a145d9aea5033a6430be4163d` also received a bounded manual
native Linux x86_64 correctness check. An ephemeral `rust:1.96.1-bookworm`
container ran on Linux 6.6.68 with AVX2 visible, a two-CPU limit, and a 4 GiB
memory limit. This command passed all six provider and batched-decode tests:

```sh
cargo test -p ferrite-inference --test batched_decode --locked
```

The container used Rust 1.96.1 and was deleted after the run. This proves the
tested native x86_64 correctness path for that dirty milestone tree. It is not
release, packaging, soak, memory, TTFT, or performance evidence.

On 2026-07-14, a broader snapshot of the same dirty milestone tree received a
second native Linux x86_64 check on homelab-02, an Intel N100 with four physical
cores, AVX2, and AVX-VNNI. The source archive excluded `.git`, `target`, eval
artifacts, models, and environment files and had SHA-256
`1d58d71f91179f09559d29ee2ec0241536f7f97df33f80376cf03837e7a35905`.
The temporary pod used Rust 1.96.1 from
`rust:1.96.1-bookworm@sha256:a339861ae23e9abb272cea45dfafde21760d2ce6577a70f8a926153677902663`
with three CPUs, 6 GiB memory, and 12 GiB ephemeral-storage limits. Both native
commands passed:

```sh
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
```

Strict Clippy finished in 53.91 seconds. The test build finished in 2 minutes
35 seconds and the full command returned success; real-model cases without
local artifacts were ignored as designed. After the successful command, the
kubelet evicted the dormant pod because the compiled target tree exceeded the
12 GiB ephemeral-storage limit. The pod and local transfer archive were then
deleted. This is native compile, lint, and execution evidence. The eviction is
an infrastructure-capacity observation and this run remains neither
real-model nor performance evidence.

The same node then ran a separate release-mode real-model correctness gate from
source snapshot
`05ad72c318fce25ea5f32d85d3ba9d35e7dc9a236331f35f2d0d452b5c7d2bf0`.
The input was the pinned SmolLM2 135M Instruct Q4_K_M artifact with SHA-256
`2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d`.
The pod used the same Rust image digest with three CPUs, 6 GiB memory, and
24 GiB ephemeral storage. This exact command passed four serial live HTTP tests
covering chat, completion, streaming chat, and streaming completion:

```sh
FERRITE_REAL_MODEL=/tmp/SmolLM2-135M-Instruct-Q4_K_M.gguf \
  cargo test --release --locked -p ferrite-server --all-features \
    --test real_models http_tier0:: -- --ignored --test-threads=1
```

The optimized build finished in 2 minutes 18 seconds, and all four tests passed
in 5.46 seconds. The target tree was 458 MiB. The temporary pod, copied model,
and local source archive were deleted afterward, and both homelab nodes
reported Ready. This extends native x86_64 evidence to one exact small-model
HTTP path. Shared-node timings are not performance evidence, and no larger
native x86_64 model claim follows from this run.

The same node later received a bounded native release-mode correctness check
for the exact Phi-3 Mini 4K Instruct 3.8B Q4 artifact. The retained evidence is
`scripts/evals/2026-07-14-085037-phi3-native-linux-x86-http.json`. Its filtered
source archive had SHA-256
`5fa60ba2bff82aeaa3c0a2ce8fc865e75415d1b78129e9ddbb190b75ca03a455`
and represented source tree
`a2b6bfa7b659e283de239029a80fa8c0a3581f1d812bc34a74d35823c0cb8651`.
The model was 2,393,231,072 bytes with SHA-256
`8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`.
The pod used the same pinned Rust image with hard limits of three CPUs, 8 GiB
memory, and 12 GiB ephemeral storage. This exact command passed:

```sh
FERRITE_PHI3_MODEL=/tmp/Phi-3-mini-4k-instruct-q4.gguf \
  cargo test --release --locked -p ferrite-server --all-features \
    --test real_models http_phi3:: -- --ignored --test-threads=1
```

The one test rehashed the model and exercised non-streaming plus streaming Chat
Completions. It verified visible content ` Steel`, visible token IDs 2443 and
295, hidden terminal token 32007, model-native EOS finish provenance, and usage
of 10 prompt plus 3 completion tokens. The pod had zero restarts and no build
or inference process remained at postflight. Build time, the 77.35 second test
time, and one live resource sample are operational observations only. The
shared node and interrupted control-plane transfer prevent throughput, TTFT,
RSS, energy, peak-resource, or build-speed claims.

KleidiAI, oneDNN, SVE2, SME2, VNNI, AVX512-VNNI, AMX, and NUMA binding are not
current execution dependencies. They remain bounded experiments. Ferrite will
adopt one only after license and supply-chain review, reference parity, and
clean repeated end-to-end measurements show a benefit for real model formats.
The primary candidate sources are [Arm KleidiAI](https://github.com/ARM-software/kleidiai)
and [oneDNN](https://github.com/uxlfoundation/oneDNN).
