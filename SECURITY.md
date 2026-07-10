# Security policy

Ferrite is pre-release software. The latest `main` branch is the only supported
version until tagged releases are published.

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

## Scope

Security-sensitive areas include GGUF parsing, size arithmetic, unsafe SIMD
kernels, authentication ordering, request limits, cancellation, cache
isolation, concurrency, dependency advisories, and accidental data disclosure.

Operational hardening such as TLS, internet-facing rate limiting, sandboxing,
and durable audit logs belongs in the deployment boundary and is not currently
provided by Ferrite itself.
