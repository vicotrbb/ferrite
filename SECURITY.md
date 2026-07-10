# Security policy

Ferrite is alpha software. After the first tagged release, only the latest
published `0.x` release is supported for security fixes. Before that point, the
latest `main` branch is the only supported version.

| Version | Security support |
| --- | --- |
| Latest tagged `0.x` release | Supported |
| Older tagged releases | Upgrade required |
| Unreleased commits | Best effort, no backport commitment |

Maintainers aim to acknowledge a complete private report within seven calendar
days. Acknowledgement is not a promise of a fix timeline. Severity,
exploitability, and a reliable reproduction determine the remediation plan.

## Report a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
[security advisory form](https://github.com/vicotrbb/ferrite/security/advisories/new)
and include:

- affected commit or version;
- architecture and operating system;
- minimal reproduction;
- impact and attacker prerequisites;
- whether malformed models, network requests, or local configuration are
  involved;
- any suggested mitigation.

Remove API keys, prompts, proprietary model data, and unrelated personal
information from the report.

## Response

Maintainers will acknowledge a complete report, reproduce it when possible,
assess severity, prepare a fix and regression test, and coordinate disclosure.
Timelines depend on impact and reproduction quality.

When a report is confirmed, maintainers will use a private GitHub Security
Advisory while a fix is prepared. Public disclosure includes the affected
versions, mitigation, fixed version, and upgrade path. A compromised release
also requires credential rotation and a new immutable patch release. Existing
release assets are not replaced or silently removed.

## Scope

Security-sensitive areas include GGUF parsing, size arithmetic, unsafe SIMD
kernels, authentication ordering, request limits, cancellation, cache
isolation, concurrency, dependency advisories, and accidental data disclosure.

Operational hardening such as TLS, internet-facing rate limiting, sandboxing,
and durable audit logs belongs in the deployment boundary and is not currently
provided by Ferrite itself.

## Release integrity

Official artifacts are published only through the repository's tag-driven
release workflow. Consumers can verify a release asset with `gh release
verify-asset` and its build provenance with `gh attestation verify`. The
official OCI image is `ghcr.io/vicotrbb/ferrite`; production deployments should
pin an immutable digest rather than a moving tag.

Ferrite does not distribute model weights. Verify a GGUF model's publisher,
license, revision, and checksum independently of the Ferrite artifact.
