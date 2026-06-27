# Benchmark Notes

Benchmark notes record measurement protocols and results. Performance claims in
Ferrite should cite benchmark notes or command output.

Create files as:

`YYYY-MM-DD-short-topic.md`

## Template

~~~markdown
# Benchmark: Topic

Date: YYYY-MM-DD

## Purpose

What claim or regression risk does this benchmark address?

## Environment

- Commit or tree state:
- Hardware:
- OS:
- CPU features:
- Execution target: local macOS | homelab staging pod | other

## Model

- Name:
- Source:
- Format:
- Quantization:
- Context:

## Command

```bash
<command>
```

## Results

- Decode throughput:
- Prefill throughput:
- TTFT:
- Peak RSS:
- Notes:

## Interpretation

What does this prove, contradict, or leave unknown?
~~~
