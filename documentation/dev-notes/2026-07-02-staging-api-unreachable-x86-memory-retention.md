# Staging API Unreachable During x86 Memory-Retention Probe

## Scope

This note records an attempted bounded x86_64 memory-retention probe for the
Ferrite long-chat gate. The probe was not run because the Kubernetes `staging`
API server refused connections before pod creation.

This is infrastructure reachability evidence, not Ferrite runtime evidence.

## Attempt

The planned pod was `ferrite-avx2-memory-qwen05-3x128`, using the established
bounded amd64 shape:

- image: `rust:1.96-bookworm`;
- node selector: `kubernetes.io/arch: amd64`;
- CPU request: `500m`;
- CPU limit: `2`;
- memory request: `1Gi`;
- memory limit: `6Gi`;
- ephemeral-storage request: `6Gi`;
- ephemeral-storage limit: `10Gi`;
- `emptyDir` size limit: `10Gi`;
- priority class: `homelab-low`.

Before any Kubernetes operation, the current context was verified:

```sh
kubectl config current-context
```

Output:

```text
staging
```

The configured API endpoint was:

```text
https://192.168.50.132:6443
```

## Failure

The lightweight node read failed:

```text
error: Get "https://192.168.50.132:6443/api/v1/nodes?limit=500": dial tcp 192.168.50.132:6443: connect: connection refused - error from a previous attempt: unexpected EOF
```

The pod apply failed before creating a pod:

```text
error: error when retrieving current configuration of:
Resource: "/v1, Resource=pods", GroupVersionKind: "/v1, Kind=Pod"
Name: "ferrite-avx2-memory-qwen05-3x128", Namespace: "default"
from server for: "STDIN": Get "https://192.168.50.132:6443/api/v1/namespaces/default/pods/ferrite-avx2-memory-qwen05-3x128": dial tcp 192.168.50.132:6443: connect: connection refused - error from a previous attempt: unexpected EOF
```

Follow-up read-only diagnostics also failed:

```sh
kubectl --context staging cluster-info
kubectl --context staging get --raw=/readyz
```

Outputs:

```text
The connection to the server 192.168.50.132:6443 was refused - did you specify the right host or port?
The connection to the server 192.168.50.132:6443 was refused - did you specify the right host or port?
```

## Result

No Ferrite x86_64 pod was created, and no x86 memory-retention benchmark was
run in this slice. The next x86 memory-retention attempt should retry only
after `kubectl --context staging get --raw=/readyz` succeeds.

## Retry

Later on 2026-07-02, `kubectl --context staging get --raw=/readyz` returned
`ok`, and the bounded x86_64 retry completed. The benchmark note is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-0-5b-memory-retention-3x128.md`.
