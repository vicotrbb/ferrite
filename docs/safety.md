# Safety policy

Ferrite uses unsafe Rust only where architecture intrinsics or inline assembly
are required for measured CPU kernels.

## Enforced boundaries

The workspace denies `unsafe_code` by default. A small architecture-specific
module can opt in only with an explicit reason. The workspace also denies
unsafe operations outside an explicit unsafe block, and Clippy denies unsafe
blocks without an adjacent safety rationale.

Each safe wrapper must establish all relevant conditions before entering a
kernel:

- target architecture and runtime CPU feature support;
- input and output lengths;
- block layout and tensor dimensions;
- pointer validity and load width;
- alignment when an intrinsic requires it;
- non-overlap when writes could alias;
- finite scale and value requirements;
- valid lane and offset ranges.

## Review requirements

An unsafe change requires:

1. A narrow module and block-level safety explanation.
2. A safe reference implementation or independently checked expected result.
3. Boundary tests for short, malformed, and unusual inputs at the safe wrapper.
4. Direct parity tests for the optimized kernel.
5. Strict Clippy on every supported architecture.
6. Real-model token parity and a comparable benchmark when the hot path changes.

Inline assembly must declare accurate options and must not rely on undocumented
register, memory, or stack effects.

## Failure behavior

Malformed GGUF data, invalid API input, unsupported tensor formats, capacity
limits, and poisoned synchronization state return errors. They must not cross
an unsafe boundary with unchecked assumptions.

Input-sized GGUF and rendered-prompt buffers apply hard count or byte limits
and use fallible capacity reservation. The optional Locus KV backend returns a
capacity error and does not fall back to unbounded vector allocation. Ferrite
does not claim recovery after global process allocator exhaustion outside these
bounded paths.

Built-in model acquisition uses an immutable registry entry, HTTPS-only
redirect policy, partial-file resume, exact size and SHA-256 verification,
atomic publication, read-only final files, and symbolic-link rejection. An
explicit model path never causes a download.

Generated function calls are untrusted response data. Ferrite bounds and
parses their JSON envelopes, but never validates application authorization or
executes the named function. The caller must validate arguments against its
own schema and policy before performing an action.

Panic, `unwrap`, and `expect` are denied by Clippy for workspace code. Tests can
use assertion panics as part of the Rust test contract.

## Dependency safety

CI checks RustSec advisories, licenses, duplicate crate versions, wildcard
requirements, and non-registry sources. The repository does not ignore known
advisories without a documented exception.

See [SECURITY.md](../SECURITY.md) for vulnerability reporting.
