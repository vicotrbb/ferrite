#!/usr/bin/env python3
"""Ferrite evaluation harness.

Measures tokens/second, time-to-first-token, memory, and CPU for the
ferrite CLI and the OpenAI-compatible server, then writes a JSON record
and a Markdown report into scripts/evals/.

Stdlib only. All inference measurement happens inside Ferrite's
instrumented binaries; this script orchestrates, timestamps stdout
lines, samples `ps`, and aggregates.
"""

import argparse
import hashlib
import json
import math
import os
import platform
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from collections import namedtuple
from datetime import datetime, timezone
from pathlib import Path

SCHEMA_VERSION = 6
REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_DIR = REPO_ROOT / "target" / "models"
EVALS_DIR = REPO_ROOT / "scripts" / "evals"
DEFAULT_PROMPT = "Write a short story about a rusty robot who learns to sail."
DEFAULT_MODEL_REPOSITORY = "Qwen/Qwen2.5-0.5B-Instruct-GGUF"
DEFAULT_MODEL_REVISION = "df5bf01389a39c743ab467d734bf501681e041c5"
DEFAULT_MODEL_LICENSE = "Apache-2.0"
DEFAULT_MODEL_FILENAME = "qwen2.5-0.5b-instruct-q4_k_m.gguf"
DEFAULT_MODEL_SIZE = 491_400_032
DEFAULT_MODEL_SHA256 = (
    "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db"
)
DEFAULT_MODEL_SOURCE = f"https://huggingface.co/{DEFAULT_MODEL_REPOSITORY}"
DEFAULT_MODEL_LICENSE_URL = (
    f"{DEFAULT_MODEL_SOURCE}/blob/{DEFAULT_MODEL_REVISION}/LICENSE"
)
DEFAULT_MODEL_URL = (
    f"{DEFAULT_MODEL_SOURCE}/resolve/"
    f"{DEFAULT_MODEL_REVISION}/{DEFAULT_MODEL_FILENAME}"
)
API_KEY = "ferrite-eval"
SERVER_WORKLOADS = ("identical", "shared-prefix", "distinct", "mixed-length")
SHARED_PREFIX_TOPICS = (
    "navigation",
    "weather",
    "repair",
    "friendship",
    "maps",
    "harbors",
    "wind",
    "nightfall",
)
DISTINCT_PROMPTS = (
    "Explain in one short paragraph why iron rusts.",
    "Write a compact recipe for vegetable soup.",
    "List three practical ways to reduce household energy use.",
    "Describe how a compass works for a curious child.",
    "Summarize the water cycle in four sentences.",
    "Draft a polite note rescheduling a morning appointment.",
    "Compare a sailboat and a canoe in a concise table.",
    "Tell a brief fable about patience and a mountain trail.",
)

Sample = namedtuple("Sample", ["t", "rss_bytes", "cpu_seconds"])


def sha256_file(path):
    """Return the lowercase SHA-256 digest for a file."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_kv_lines(lines):
    """Parse ferrite's key=value stdout lines; last occurrence wins."""
    result = {}
    for line in lines:
        key, sep, value = line.partition("=")
        if sep and key and " " not in key:
            result[key] = value.rstrip("\r\n")
    return result


def percentile(values, pct):
    """Nearest-rank percentile; values need not be sorted."""
    if not values:
        raise ValueError("percentile of empty list")
    ordered = sorted(values)
    rank = max(1, math.ceil(pct / 100 * len(ordered)))
    return ordered[rank - 1]


def parse_cputime(text):
    """Parse ps cputime ([DD-]HH:MM:SS.ss or MM:SS.ss) into seconds."""
    text = text.strip()
    days = 0.0
    if "-" in text:
        day_part, text = text.split("-", 1)
        days = float(day_part)
    seconds = 0.0
    for part in text.split(":"):
        seconds = seconds * 60 + float(part)
    return days * 86400 + seconds


class ProcessSampler(threading.Thread):
    """Samples RSS and accumulated CPU time of a pid via ps every interval."""

    def __init__(self, pid, interval_s=0.1):
        super().__init__(daemon=True)
        self.pid = pid
        self.interval_s = interval_s
        self.samples = []
        self._stop_event = threading.Event()

    def run(self):
        while not self._stop_event.is_set():
            proc = subprocess.run(
                ["ps", "-o", "rss=,cputime=", "-p", str(self.pid)],
                capture_output=True,
                text=True,
            )
            fields = proc.stdout.split()
            if proc.returncode == 0 and len(fields) >= 2:
                try:
                    self.samples.append(
                        Sample(
                            t=time.monotonic(),
                            rss_bytes=int(fields[0]) * 1024,
                            cpu_seconds=parse_cputime(fields[1]),
                        )
                    )
                except ValueError:
                    pass
            self._stop_event.wait(self.interval_s)

    def stop(self):
        self._stop_event.set()
        self.join()
        return self.samples


def aggregate_samples(samples, t_start=None, t_end=None):
    """Aggregate sampler output over an optional [t_start, t_end] window."""
    window = [
        s
        for s in samples
        if (t_start is None or s.t >= t_start) and (t_end is None or s.t <= t_end)
    ]
    if not window:
        return {"sample_count": 0}
    result = {
        "sample_count": len(window),
        "rss_peak_bytes": max(s.rss_bytes for s in window),
        "rss_mean_bytes": sum(s.rss_bytes for s in window) // len(window),
    }
    pairs = list(zip(window, window[1:]))
    if any(cur.t <= prev.t for prev, cur in pairs):
        result["cpu_metrics_status"] = "non_monotonic_sample_time"
    elif any(cur.cpu_seconds < prev.cpu_seconds for prev, cur in pairs):
        result["cpu_metrics_status"] = "cumulative_counter_regressed"
    elif pairs:
        result["cpu_mean_percent"] = round(
            (window[-1].cpu_seconds - window[0].cpu_seconds)
            / (window[-1].t - window[0].t)
            * 100,
            1,
        )
        deltas = [
            (cur.cpu_seconds - prev.cpu_seconds) / (cur.t - prev.t) * 100
            for prev, cur in pairs
        ]
        result["cpu_peak_percent"] = round(max(deltas), 1)
    return result


RunResult = namedtuple("RunResult", ["t_spawn", "lines", "returncode", "stderr", "samples"])


def run_timestamped(cmd, timeout_s=1800):
    """Run cmd, timestamping each stdout line while sampling the process."""
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr_file:
        t_spawn = time.monotonic()
        proc = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=stderr_file, text=True
        )
        sampler = ProcessSampler(proc.pid)
        sampler.start()
        lines = []
        try:
            deadline = t_spawn + timeout_s
            for line in proc.stdout:
                lines.append((time.monotonic(), line.rstrip("\n")))
                if time.monotonic() > deadline:
                    proc.kill()
                    raise TimeoutError(f"{cmd[0]} exceeded {timeout_s}s")
            returncode = proc.wait(timeout=60)
        finally:
            sampler.stop()
            if proc.poll() is None:
                proc.kill()
        stderr_file.seek(0)
        stderr = stderr_file.read()
    return RunResult(t_spawn, lines, returncode, stderr, sampler.samples)


def compute_cli_metrics(t_spawn, lines, sleep_ms):
    """Timing metrics from a timestamped `--stream` generation run.

    Relies on ferrite's output ordering: `sleep_after_load_ms=` is printed
    and flushed after weight load but before the sleep; `prompt_token_ids=`
    is the first line printed after prefill; each generated token prints a
    `stream_token_id=` line as it is produced.
    """

    def first_ts(prefix):
        for t, line in lines:
            if line.startswith(prefix):
                return t
        return None

    metrics = {}
    t_sleep = first_ts("sleep_after_load_ms=")
    if t_sleep is None:
        return metrics
    t_gen_start = t_sleep + sleep_ms / 1000
    metrics["load_seconds"] = round(t_sleep - t_spawn, 3)
    metrics["t_sleep"] = t_sleep
    metrics["t_gen_start"] = t_gen_start
    t_prefill_done = first_ts("prompt_token_ids=")
    if t_prefill_done is not None:
        metrics["ttft_prefill_seconds"] = round(t_prefill_done - t_gen_start, 3)
    stream_ts = [t for t, line in lines if line.startswith("stream_token_id=")]
    if len(stream_ts) >= 2:
        deltas_ms = [(b - a) * 1000 for a, b in zip(stream_ts, stream_ts[1:])]
        metrics["stream_token_count"] = len(stream_ts)
        metrics["decode_tokens_per_second_streamed"] = round(
            (len(stream_ts) - 1) / (stream_ts[-1] - stream_ts[0]), 2
        )
        metrics["token_latency_ms_mean"] = round(sum(deltas_ms) / len(deltas_ms), 1)
        metrics["token_latency_ms_p50"] = round(percentile(deltas_ms, 50), 1)
        metrics["token_latency_ms_p95"] = round(percentile(deltas_ms, 95), 1)
        metrics["t_last_stream"] = stream_ts[-1]
    return metrics


EvalConfig = namedtuple(
    "EvalConfig",
    [
        "prompt",
        "generate_tokens",
        "benchmark_runs",
        "sleep_ms",
        "requests",
        "batch_streams",
        "server_batch_streams",
        "server_workload",
        "experimental_residual_q8_activation_matvec",
        "experimental_q8_k_activation_matvec",
        "experimental_q8_k_activation_roles",
        "server_soak_rounds",
        "server_soak_idle_ms",
        "server_soak_rss_tolerance_bytes",
        "server_prefix_cache",
        "kernel_provider",
        "server_kv_backend",
        "server_kv_tokens_per_block",
        "server_kv_max_tokens",
        "threads",
    ],
    defaults=[False, "auto", "vec", None, None, None],
)


def server_workload_prompts(workload, base_prompt, requests):
    """Return the deterministic prompt set for one server workload."""
    if requests < 1:
        raise ValueError("server workloads require at least one request")
    if workload == "identical":
        return (base_prompt,)
    if workload == "shared-prefix":
        return tuple(
            f"{base_prompt}\n\nWrite a distinct ending focused on "
            f"{SHARED_PREFIX_TOPICS[index % len(SHARED_PREFIX_TOPICS)]}; "
            f"variant {index + 1}."
            for index in range(requests)
        )
    if workload == "distinct":
        return tuple(
            f"{DISTINCT_PROMPTS[index % len(DISTINCT_PROMPTS)]} "
            f"Request variant {index + 1}."
            for index in range(requests)
        )
    if workload == "mixed-length":
        return tuple(
            f"{DISTINCT_PROMPTS[index % len(DISTINCT_PROMPTS)]} "
            f"Mixed-length variant {index + 1}."
            for index in range(requests)
        )
    raise ValueError(f"unknown server workload: {workload}")


def server_workload_token_budgets(workload, max_tokens):
    """Return deterministic per-prompt token budgets for a server workload."""
    if workload != "mixed-length":
        return (max_tokens,)
    candidates = (1, max(1, max_tokens // 4), max(1, max_tokens // 2), max_tokens)
    return tuple(dict.fromkeys(candidates))


def server_token_traces_match(default, batched):
    """Compare exact per-prompt traces across default and batched routes."""
    default_traces = default.get("streaming_prompt_token_id_traces")
    batched_traces = batched.get("streaming_prompt_token_id_traces")
    if isinstance(default_traces, list) and isinstance(batched_traces, list):
        return (
            bool(default_traces)
            and default.get("streaming_all_prompt_token_id_traces_stable") == "true"
            and batched.get("streaming_all_prompt_token_id_traces_stable") == "true"
            and default_traces == batched_traces
        )

    default_trace = default.get("streaming_token_id_trace")
    batched_trace = batched.get("streaming_token_id_trace")
    return (
        default_trace is not None
        and batched_trace is not None
        and default.get("streaming_all_token_id_traces_match") == "true"
        and batched.get("streaming_all_token_id_traces_match") == "true"
        and default_trace == batched_trace
    )


def server_kv_flags(cfg):
    """Return the explicit server KV configuration for one eval phase."""
    flags = ["--kv-backend", cfg.server_kv_backend]
    if cfg.server_kv_backend == "locus":
        flags += [
            "--kv-tokens-per-block",
            str(cfg.server_kv_tokens_per_block),
            "--kv-max-tokens",
            str(cfg.server_kv_max_tokens),
        ]
    return flags


def cli_execution_flags(cfg):
    flags = []
    if cfg.threads is not None:
        flags += ["--threads", str(cfg.threads)]
    if cfg.kernel_provider != "auto":
        flags += ["--kernel-provider", cfg.kernel_provider]
    if cfg.experimental_residual_q8_activation_matvec:
        flags.append("--experimental-residual-q8-activation-matvec")
    elif cfg.experimental_q8_k_activation_matvec:
        flags.append("--experimental-q8-k-activation-matvec")
    if cfg.experimental_q8_k_activation_roles:
        flags += [
            "--experimental-q8-k-activation-roles",
            cfg.experimental_q8_k_activation_roles,
        ]
    return flags


def run_cli_batch_benchmark(ferrite_bin, model_path, cfg, streams):
    """Measure aggregate decode throughput for one fixed engine batch size."""
    cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--benchmark-runs", str(cfg.benchmark_runs),
        "--benchmark-batch-streams", str(streams),
    ] + cli_execution_flags(cfg)
    result = {"streams": streams, "command": " ".join(cmd), "status": "ok"}
    run = run_timestamped(cmd)
    if run.returncode != 0:
        result["status"] = "failed"
        result["stderr"] = run.stderr[-2000:]
        return result

    kv = parse_kv_lines(line for _, line in run.lines)
    for source, destination, conversion in (
        ("inference_threads", "inference_threads", int),
        ("kernel_provider", "kernel_provider", str),
        ("cpu_features", "cpu_features", str),
        ("benchmark_runs", "decode_steps", int),
        ("benchmark_total_ns", "total_ns", int),
        ("benchmark_avg_ns", "average_step_ns", int),
        ("benchmark_batch_tokens_per_second", "aggregate_tokens_per_second", float),
        ("benchmark_token_ids", "stream_0_token_ids", str),
        ("kv_cache_bytes", "kv_cache_bytes", int),
    ):
        if source in kv:
            result[destination] = conversion(kv[source])

    aggregate = result.get("aggregate_tokens_per_second")
    if aggregate is not None:
        result["per_stream_tokens_per_second"] = round(aggregate / streams, 2)

    samples = aggregate_samples(run.samples)
    for key in (
        "rss_peak_bytes",
        "cpu_mean_percent",
        "cpu_peak_percent",
        "cpu_metrics_status",
    ):
        if key in samples:
            result[key] = samples[key]
    return result


def run_cli_phase(ferrite_bin, model_path, cfg):
    """Generation run (wall-clock + ps sampling), then precise benchmark run."""
    phase = {"status": "ok"}
    gen_cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--sleep-after-load-ms", str(cfg.sleep_ms),
        "--generate-tokens", str(cfg.generate_tokens),
        "--stream",
    ] + cli_execution_flags(cfg)
    phase["generation_command"] = " ".join(gen_cmd)
    run = run_timestamped(gen_cmd)
    if run.returncode != 0:
        phase["status"] = "failed"
        phase["stderr"] = run.stderr[-2000:]
        return phase

    metrics = compute_cli_metrics(run.t_spawn, run.lines, cfg.sleep_ms)
    t_sleep = metrics.pop("t_sleep", None)
    t_gen_start = metrics.pop("t_gen_start", None)
    t_last_stream = metrics.pop("t_last_stream", None)
    phase.update(metrics)

    kv = parse_kv_lines(line for _, line in run.lines)
    for key in (
        "inference_threads",
        "kernel_provider",
        "cpu_features",
        "model_file_bytes",
        "scalar_weight_bytes",
        "kv_cache_bytes",
        "generated_stopped_on_eos",
        "q8_k_activation_matvec_policy",
    ):
        if key in kv:
            phase[key] = int(kv[key]) if key == "inference_threads" else kv[key]

    if t_sleep is not None:
        load_window = aggregate_samples(run.samples, t_sleep, t_gen_start)
        if "rss_peak_bytes" in load_window:
            phase["rss_post_load_bytes"] = load_window["rss_peak_bytes"]
    whole_run = aggregate_samples(run.samples)
    if "rss_peak_bytes" in whole_run:
        phase["rss_peak_bytes"] = whole_run["rss_peak_bytes"]
    if t_gen_start is not None and t_last_stream is not None:
        gen_window = aggregate_samples(run.samples, t_gen_start, t_last_stream)
        for key in ("cpu_mean_percent", "cpu_peak_percent", "cpu_metrics_status"):
            if key in gen_window:
                phase[key] = gen_window[key]

    bench_cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--benchmark-runs", str(cfg.benchmark_runs),
    ] + cli_execution_flags(cfg)
    phase["benchmark_command"] = " ".join(bench_cmd)
    bench = run_timestamped(bench_cmd)
    if bench.returncode == 0:
        bench_kv = parse_kv_lines(line for _, line in bench.lines)
        avg_ns = int(bench_kv.get("benchmark_avg_ns", "0"))
        if avg_ns > 0:
            phase["benchmark_avg_ns"] = avg_ns
            phase["decode_tokens_per_second_precise"] = round(1e9 / avg_ns, 2)
        if "benchmark_token_ids" in bench_kv:
            phase["benchmark_token_ids"] = bench_kv["benchmark_token_ids"]
    else:
        phase["benchmark_error"] = bench.stderr[-500:]

    if cfg.batch_streams:
        phase["batch_benchmarks"] = [
            run_cli_batch_benchmark(ferrite_bin, model_path, cfg, streams)
            for streams in cfg.batch_streams
        ]
        single_token_ids = phase.get("benchmark_token_ids")
        if single_token_ids is not None:
            for batch in phase["batch_benchmarks"]:
                batch_token_ids = batch.get("stream_0_token_ids")
                if batch_token_ids is not None:
                    batch["stream_0_matches_single"] = batch_token_ids == single_token_ids
    return phase


def find_free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_health(port, proc, timeout_s=300):
    """Poll GET /health until 200, the process dies, or timeout."""
    deadline = time.monotonic() + timeout_s
    url = f"http://127.0.0.1:{port}/health"
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        try:
            with urllib.request.urlopen(url, timeout=2) as response:
                if response.status == 200:
                    return True
        except OSError:
            pass
        time.sleep(0.5)
    return False


def process_rss_bytes(pid):
    result = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)],
        capture_output=True,
        text=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise RuntimeError(f"failed to sample RSS for pid {pid}")
    fields = result.stdout.split()
    if not fields:
        raise RuntimeError(f"RSS sample for pid {pid} was empty")
    return int(fields[0]) * 1024


def parse_macos_phys_footprint_bytes(output):
    """Parse the byte-formatted phys_footprint value from Apple's tool."""
    for line in output.splitlines():
        key, separator, value = line.strip().partition(":")
        if separator and key == "phys_footprint":
            fields = value.split()
            if len(fields) != 2 or fields[1] != "B":
                break
            try:
                return int(fields[0])
            except ValueError:
                break
    raise ValueError("footprint output did not contain a byte phys_footprint value")


def process_macos_phys_footprint_bytes(pid):
    result = subprocess.run(
        ["footprint", "-p", str(pid), "-f", "bytes", "--noCategories"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise RuntimeError(
            f"failed to sample macOS physical footprint for pid {pid}: {detail}"
        )
    return parse_macos_phys_footprint_bytes(result.stdout)


def server_trace_identity(kv):
    per_prompt = kv.get("streaming_prompt_token_id_traces")
    if per_prompt is not None:
        traces = json.loads(per_prompt)
        if not isinstance(traces, list) or not traces or any(not trace for trace in traces):
            raise RuntimeError("server response contained an empty per-prompt token trace")
        return json.dumps(traces, separators=(",", ":"))
    trace = kv.get("streaming_token_id_trace")
    if not trace:
        raise RuntimeError("server response did not contain a token trace")
    return trace


def summarize_soak_rss(rss_idle_bytes, tolerance_bytes):
    if len(rss_idle_bytes) < 3:
        raise ValueError("soak RSS summary requires at least three rounds")
    tail = rss_idle_bytes[len(rss_idle_bytes) // 2 :]
    growth = rss_idle_bytes[-1] - rss_idle_bytes[0]
    tail_range = max(tail) - min(tail)
    return {
        "soak_rss_growth_bytes": growth,
        "soak_rss_tail_range_bytes": tail_range,
        "soak_rss_tolerance_bytes": tolerance_bytes,
        "soak_rss_stable": growth <= tolerance_bytes and tail_range <= tolerance_bytes,
    }


def summarize_soak_physical_footprint(footprint_idle_bytes, tolerance_bytes):
    if len(footprint_idle_bytes) < 3:
        raise ValueError("soak physical-footprint summary requires at least three rounds")
    tail = footprint_idle_bytes[len(footprint_idle_bytes) // 2 :]
    growth = footprint_idle_bytes[-1] - footprint_idle_bytes[0]
    tail_range = max(tail) - min(tail)
    return {
        "soak_physical_footprint_growth_bytes": growth,
        "soak_physical_footprint_tail_range_bytes": tail_range,
        "soak_physical_footprint_stable": (
            growth <= tolerance_bytes and tail_range <= tolerance_bytes
        ),
    }


def run_server_soak(client_cmd, server_pid, cfg, expected_trace):
    rss_idle_bytes = []
    physical_footprint_idle_bytes = []
    use_macos_footprint = platform.system() == "Darwin"

    def run_cohort():
        client = subprocess.run(
            client_cmd, capture_output=True, text=True, timeout=1800
        )
        if client.returncode != 0:
            raise RuntimeError((client.stderr or client.stdout)[-2000:])
        kv = parse_kv_lines(client.stdout.splitlines())
        return server_trace_identity(kv) == expected_trace and (
            kv.get("streaming_all_prompt_token_id_traces_stable", "true") == "true"
        )

    # Exercise request, scheduler, and allocator paths once more before the
    # measured rounds. The timed cohort above proves the route, while this
    # unmeasured cohort establishes the steady-state memory baseline used by
    # the leak gate.
    traces_match = run_cohort()
    time.sleep(cfg.server_soak_idle_ms / 1000)
    soak_started = time.monotonic()
    for _ in range(cfg.server_soak_rounds):
        traces_match &= run_cohort()
        time.sleep(cfg.server_soak_idle_ms / 1000)
        rss_idle_bytes.append(process_rss_bytes(server_pid))
        if use_macos_footprint:
            physical_footprint_idle_bytes.append(
                process_macos_phys_footprint_bytes(server_pid)
            )
    soak_finished = time.monotonic()

    result = {
        "soak_warmup_requests": cfg.requests,
        "soak_rounds": cfg.server_soak_rounds,
        "soak_requests": cfg.server_soak_rounds * cfg.requests,
        "soak_elapsed_seconds": round(soak_finished - soak_started, 3),
        "soak_rss_idle_bytes": rss_idle_bytes,
        "soak_all_token_traces_match": traces_match,
    }
    result.update(
        summarize_soak_rss(rss_idle_bytes, cfg.server_soak_rss_tolerance_bytes)
    )
    if use_macos_footprint:
        result["soak_physical_footprint_idle_bytes"] = physical_footprint_idle_bytes
        result.update(
            summarize_soak_physical_footprint(
                physical_footprint_idle_bytes,
                cfg.server_soak_rss_tolerance_bytes,
            )
        )
        result.update(
            {
                "soak_memory_stability_metric": "macos_phys_footprint",
                "soak_memory_growth_bytes": result[
                    "soak_physical_footprint_growth_bytes"
                ],
                "soak_memory_tail_range_bytes": result[
                    "soak_physical_footprint_tail_range_bytes"
                ],
                "soak_memory_stable": result["soak_physical_footprint_stable"],
            }
        )
    else:
        result.update(
            {
                "soak_memory_stability_metric": "rss",
                "soak_memory_growth_bytes": result["soak_rss_growth_bytes"],
                "soak_memory_tail_range_bytes": result[
                    "soak_rss_tail_range_bytes"
                ],
                "soak_memory_stable": result["soak_rss_stable"],
            }
        )
    return result


def server_client_command(
    throughput_bin,
    port,
    requests,
    concurrency,
    prompts,
    token_budgets,
    workload,
    prefix_cache,
):
    command = [
        str(throughput_bin),
        "--addr",
        f"127.0.0.1:{port}",
        "--model",
        "eval",
        "--endpoint",
        "chat-completions",
        "--requests",
        str(requests),
        "--concurrency",
        str(concurrency),
        "--stream",
        "--stream-usage",
        "--api-key",
        API_KEY,
    ]
    for prompt in prompts:
        command += ["--prompt", prompt]
    for max_tokens in token_budgets:
        command += ["--max-tokens", str(max_tokens)]
    if prefix_cache:
        command += [
            "--prompt-cache-key",
            f"eval:{workload}",
            "--prompt-cache-trace",
        ]
    return command


def run_server_phase(server_bin, throughput_bin, model_path, cfg, batch_streams=None):
    """Start ferrite-server, drive it with ferrite-openai-throughput, tear down."""
    token_budgets = server_workload_token_budgets(
        cfg.server_workload, cfg.generate_tokens
    )
    prompt_count = len(token_budgets) if cfg.server_workload == "mixed-length" else cfg.requests
    prompts = server_workload_prompts(cfg.server_workload, cfg.prompt, prompt_count)
    phase = {
        "status": "ok",
        "kernel_provider": cfg.kernel_provider,
        "kv_backend": cfg.server_kv_backend,
        "kv_tokens_per_block": cfg.server_kv_tokens_per_block,
        "kv_max_tokens": cfg.server_kv_max_tokens,
        "configured_inference_threads": cfg.threads,
        "workload": cfg.server_workload,
        "workload_prompt_count": len(prompts),
        "workload_prompt_sha256": [
            hashlib.sha256(prompt.encode("utf-8")).hexdigest() for prompt in prompts
        ],
        "workload_max_token_budgets": token_budgets,
    }
    port = find_free_port()
    server_cmd = [
        str(server_bin),
        "--model", str(model_path),
        "--model-id", "eval",
        "--bind", f"127.0.0.1:{port}",
        "--api-key", API_KEY,
        "--default-max-tokens", str(cfg.generate_tokens),
        "--hard-max-tokens", str(cfg.generate_tokens),
        "--inference-wait-ms", "120000",
    ] + server_kv_flags(cfg)
    if cfg.threads is not None:
        server_cmd += ["--threads", str(cfg.threads)]
    if cfg.kernel_provider != "auto":
        server_cmd += ["--kernel-provider", cfg.kernel_provider]
    if batch_streams is not None:
        server_cmd += [
            "--experimental-batched-decode",
            "--max-batch-streams", str(batch_streams),
        ]
    elif cfg.experimental_residual_q8_activation_matvec:
        server_cmd.append("--experimental-residual-q8-activation-matvec")
    if cfg.server_prefix_cache:
        server_cmd.append("--experimental-prefix-cache")
    phase["server_command"] = " ".join(server_cmd)
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as server_log:
        server = subprocess.Popen(server_cmd, stdout=server_log, stderr=subprocess.STDOUT)
        sampler = ProcessSampler(server.pid)
        sampler.start()
        try:
            if not wait_for_health(port, server):
                server_log.seek(0)
                phase["status"] = "failed"
                phase["error"] = "server never became healthy"
                phase["server_log"] = server_log.read()[-2000:]
                return phase
            client_cmd = server_client_command(
                throughput_bin,
                port,
                cfg.requests,
                batch_streams or 1,
                prompts,
                token_budgets,
                cfg.server_workload,
                cfg.server_prefix_cache,
            )
            phase["client_command"] = " ".join(client_cmd)
            if cfg.server_prefix_cache:
                warmup_cmd = server_client_command(
                    throughput_bin,
                    port,
                    1,
                    1,
                    prompts[:1],
                    token_budgets[:1],
                    cfg.server_workload,
                    True,
                )
                phase["cache_warmup_command"] = " ".join(warmup_cmd)
                warmup = subprocess.run(
                    warmup_cmd, capture_output=True, text=True, timeout=1800
                )
                if warmup.returncode != 0:
                    phase["status"] = "failed"
                    phase["error"] = (warmup.stderr or warmup.stdout)[-2000:]
                    return phase
            t_requests_start = time.monotonic()
            client = subprocess.run(client_cmd, capture_output=True, text=True, timeout=1800)
            t_requests_end = time.monotonic()
            if client.returncode != 0:
                phase["status"] = "failed"
                phase["error"] = (client.stderr or client.stdout)[-2000:]
                return phase
            kv = parse_kv_lines(client.stdout.splitlines())
            for key in (
                "streaming_time_to_first_token_ms",
                "streaming_tokens_per_second",
                "streaming_token_latency_p50_ms",
                "streaming_token_latency_p95_ms",
                "streaming_token_events",
                "streaming_timed_requests",
                "streaming_time_to_first_token_p50_ms",
                "streaming_time_to_first_token_p95_ms",
                "requests_per_second",
                "elapsed_ms",
                "openai_http_configured_prompts",
                "openai_http_distinct_prompts",
                "openai_http_configured_max_token_budgets",
                "openai_http_distinct_max_token_budgets",
                "openai_http_max_token_budgets",
                "streaming_usage_prompt_tokens",
                "streaming_usage_cached_prompt_tokens",
                "streaming_usage_completion_tokens",
                "streaming_usage_request_count",
                "streaming_usage_prompt_tokens_total",
                "streaming_usage_completion_tokens_total",
                "streaming_usage_total_tokens_total",
                "streaming_finish_reason",
                "streaming_usage_finish_source",
                "streaming_token_ids",
                "streaming_token_id_trace",
                "streaming_all_token_id_traces_match",
                "streaming_all_prompt_token_id_traces_stable",
                "streaming_text_bytes",
                "streaming_usage_prompt_cache_lookup",
                "streaming_usage_prompt_cache_shared_prefix_tokens",
            ):
                if key in kv:
                    phase[key] = kv[key]
            if "streaming_prompt_token_id_traces" in kv:
                try:
                    phase["streaming_prompt_token_id_traces"] = json.loads(
                        kv["streaming_prompt_token_id_traces"]
                    )
                except json.JSONDecodeError as error:
                    phase["status"] = "failed"
                    phase["error"] = f"invalid per-prompt token trace JSON: {error}"
                    return phase
            phase["concurrency"] = batch_streams or 1
            if (
                "streaming_usage_completion_tokens_total" in phase
                and "elapsed_ms" in phase
                and float(phase["elapsed_ms"]) > 0
            ):
                phase["aggregate_completion_tokens_per_second"] = round(
                    float(phase["streaming_usage_completion_tokens_total"])
                    / (float(phase["elapsed_ms"]) / 1000),
                    2,
                )
            elif "requests_per_second" in phase:
                phase["aggregate_completion_tokens_per_second"] = round(
                    float(phase["requests_per_second"]) * cfg.generate_tokens, 2
                )
            request_window = aggregate_samples(
                sampler.samples, t_requests_start, t_requests_end
            )
            for src, dst in (
                ("rss_peak_bytes", "server_rss_peak_bytes"),
                ("cpu_mean_percent", "server_cpu_mean_percent"),
                ("cpu_peak_percent", "server_cpu_peak_percent"),
                ("cpu_metrics_status", "server_cpu_metrics_status"),
            ):
                if src in request_window:
                    phase[dst] = request_window[src]
            if cfg.server_soak_rounds:
                try:
                    expected_trace = server_trace_identity(kv)
                    soak = run_server_soak(client_cmd, server.pid, cfg, expected_trace)
                except (OSError, RuntimeError, subprocess.SubprocessError, json.JSONDecodeError) as error:
                    phase["status"] = "failed"
                    phase["error"] = f"server soak failed: {error}"
                    return phase
                phase.update(soak)
                soak_samples = aggregate_samples(sampler.samples, t_requests_end)
                if "rss_peak_bytes" in soak_samples:
                    phase["soak_rss_peak_bytes"] = soak_samples["rss_peak_bytes"]
                if not soak["soak_all_token_traces_match"]:
                    phase["status"] = "failed"
                    phase["error"] = "server soak token traces drifted"
                elif not soak["soak_memory_stable"]:
                    phase["status"] = "failed"
                    metric = soak["soak_memory_stability_metric"].replace("_", " ")
                    phase["error"] = (
                        f"server soak {metric} did not return to the configured stable range"
                    )
            return phase
        finally:
            sampler.stop()
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
            server_log.seek(0)
            runtime_identity = parse_kv_lines(server_log)
            if "inference_threads" in runtime_identity:
                try:
                    phase["inference_threads"] = int(
                        runtime_identity["inference_threads"]
                    )
                except ValueError:
                    phase["runtime_identity_status"] = (
                        "invalid_inference_threads"
                    )
            for key in ("kernel_provider", "cpu_features"):
                if key in runtime_identity:
                    phase[key] = runtime_identity[key]


def _cmd_output(cmd, cwd=None):
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.stdout.strip() if proc.returncode == 0 else None


def capture_env():
    env = {
        "timestamp_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "hostname": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "logical_cores": os.cpu_count(),
        "git_commit": _cmd_output(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT),
        "git_branch": _cmd_output(["git", "branch", "--show-current"], cwd=REPO_ROOT),
        "git_dirty": bool(_cmd_output(["git", "status", "--porcelain"], cwd=REPO_ROOT)),
        "rustc_version": _cmd_output(["rustc", "--version"]),
        "cargo_profile": "release",
        "rustflags": os.environ.get("RUSTFLAGS", ""),
        "cargo_target_dir": os.environ.get("CARGO_TARGET_DIR", "target"),
    }
    if sys.platform == "darwin":
        env["cpu"] = _cmd_output(["sysctl", "-n", "machdep.cpu.brand_string"])
        physical = _cmd_output(["sysctl", "-n", "hw.physicalcpu"])
        env["physical_cores"] = int(physical) if physical else None
        memsize = _cmd_output(["sysctl", "-n", "hw.memsize"])
        env["ram_bytes"] = int(memsize) if memsize else None
    else:
        try:
            with open("/proc/cpuinfo", encoding="utf-8") as handle:
                for line in handle:
                    if line.startswith("model name"):
                        env["cpu"] = line.partition(":")[2].strip()
                        break
            with open("/proc/meminfo", encoding="utf-8") as handle:
                for line in handle:
                    if line.startswith("MemTotal:"):
                        env["ram_bytes"] = int(line.split()[1]) * 1024
                        break
        except OSError:
            pass
    return env


def verify_default_model(path):
    path = Path(path)
    size = path.stat().st_size
    if size != DEFAULT_MODEL_SIZE:
        raise RuntimeError(
            f"default model size mismatch: expected {DEFAULT_MODEL_SIZE}, got {size}"
        )
    digest = sha256_file(path)
    if digest != DEFAULT_MODEL_SHA256:
        raise RuntimeError(
            "default model SHA-256 mismatch: "
            f"expected {DEFAULT_MODEL_SHA256}, got {digest}"
        )


def default_model_provenance(model_sha256):
    if model_sha256 != DEFAULT_MODEL_SHA256:
        return None
    return {
        "repository": DEFAULT_MODEL_REPOSITORY,
        "source": DEFAULT_MODEL_SOURCE,
        "revision": DEFAULT_MODEL_REVISION,
        "license": DEFAULT_MODEL_LICENSE,
        "license_url": DEFAULT_MODEL_LICENSE_URL,
        "filename": DEFAULT_MODEL_FILENAME,
        "size_bytes": DEFAULT_MODEL_SIZE,
        "sha256": DEFAULT_MODEL_SHA256,
        "url": DEFAULT_MODEL_URL,
    }


def download_default_model():
    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    target = MODELS_DIR / DEFAULT_MODEL_FILENAME
    print(f"downloading {DEFAULT_MODEL_URL}")
    print(f"        -> {target}")

    def report(blocks, block_size, total):
        done_mb = blocks * block_size / 1e6
        total_mb = total / 1e6 if total > 0 else float("nan")
        print(f"\r  {done_mb:8.1f} / {total_mb:.1f} MB", end="", flush=True)

    partial = target.with_suffix(".gguf.part")
    try:
        urllib.request.urlretrieve(DEFAULT_MODEL_URL, partial, reporthook=report)
        verify_default_model(partial)
        partial.replace(target)
    except Exception:
        partial.unlink(missing_ok=True)
        raise
    print()
    return target


def resolve_models(args):
    if args.model:
        models = [Path(m) for m in args.model]
        missing = [str(m) for m in models if not m.is_file()]
        if missing:
            print(f"model file(s) not found: {', '.join(missing)}", file=sys.stderr)
            raise SystemExit(2)
        return models
    found = sorted(MODELS_DIR.glob("*.gguf"))
    if found:
        for model in found:
            if model.name == DEFAULT_MODEL_FILENAME:
                verify_default_model(model)
        return found
    hint = (
        f"No .gguf models found in {MODELS_DIR}.\n"
        f"Download one manually, e.g.:\n"
        f"  curl -L -o {MODELS_DIR}/qwen2.5-0.5b-instruct-q4_k_m.gguf \\\n"
        f"    {DEFAULT_MODEL_URL}"
    )
    if args.no_download:
        print(hint, file=sys.stderr)
        raise SystemExit(2)
    if not args.download:
        answer = input(
            f"No models in {MODELS_DIR}. Download the pinned 491 MB reference model "
            "(Qwen2.5-0.5B-Instruct Q4_K_M)? [y/N] "
        )
        if answer.strip().lower() not in ("y", "yes"):
            print(hint, file=sys.stderr)
            raise SystemExit(2)
    return [download_default_model()]


def build_report(env, cfg, model_results, tag=None):
    return {
        "schema_version": SCHEMA_VERSION,
        "tag": tag,
        "env": env,
        "config": cfg._asdict(),
        "models": model_results,
    }


def report_succeeded(report):
    models = report.get("models")
    if not isinstance(models, list) or not models:
        return False
    for model in models:
        phases = [
            model[name]
            for name in ("cli", "server", "batched_server")
            if name in model
        ]
        if not phases or any(phase.get("status") != "ok" for phase in phases):
            return False
        if (
            "batched_server" in model
            and model["batched_server"].get("token_ids_match_default") is not True
        ):
            return False
    return True


def _fmt_bytes(value):
    try:
        number = int(value)
    except (TypeError, ValueError):
        return "-"
    return f"{number / (1 << 20):.1f} MiB"


def render_markdown(report):
    env = report["env"]
    lines = [
        f"# Ferrite eval, {env.get('timestamp_utc', '?')}",
        "",
        f"- host: {env.get('hostname')}, {env.get('cpu')}, "
        f"{env.get('physical_cores')} cores, {_fmt_bytes(env.get('ram_bytes'))} RAM",
        f"- os: {env.get('platform')}",
        f"- commit: {env.get('git_commit')} ({env.get('git_branch')}"
        f"{', dirty' if env.get('git_dirty') else ''})",
        f"- rustc: {env.get('rustc_version')}",
        f"- build: cargo profile {env.get('cargo_profile', 'release')}, "
        f"RUSTFLAGS={env.get('rustflags') or '<unset>'}, "
        f"target dir {env.get('cargo_target_dir', 'target')}, "
        f"binary mode {env.get('binary_build_mode', 'cargo-build')}",
        f"- config: {json.dumps(report['config'])}",
    ]
    if report.get("tag"):
        lines.append(f"- tag: {report['tag']}")
    for entry in report["models"]:
        lines += [
            "",
            f"## {Path(entry['model_path']).name}",
            "",
            f"- model SHA-256: `{entry.get('model_sha256', '-')}`",
            "",
        ]
        cli = entry.get("cli")
        if cli:
            lines += [
                "| CLI metric | value |",
                "| --- | --- |",
                f"| status | {cli.get('status')} |",
                f"| inference threads | {cli.get('inference_threads', '-')} |",
                f"| kernel provider | {cli.get('kernel_provider', '-')} |",
                f"| detected CPU features | {cli.get('cpu_features', '-')} |",
                f"| activation matvec policy | {cli.get('q8_k_activation_matvec_policy', '-')} |",
                f"| load | {cli.get('load_seconds', '-')} s |",
                f"| TTFT (prefill, load excluded) | {cli.get('ttft_prefill_seconds', '-')} s |",
                f"| decode tok/s (precise, in-process) | {cli.get('decode_tokens_per_second_precise', '-')} |",
                f"| decode tok/s (streamed wall-clock) | {cli.get('decode_tokens_per_second_streamed', '-')} |",
                f"| token latency p50 / p95 | {cli.get('token_latency_ms_p50', '-')} / {cli.get('token_latency_ms_p95', '-')} ms |",
                f"| RSS post-load / peak | {_fmt_bytes(cli.get('rss_post_load_bytes'))} / {_fmt_bytes(cli.get('rss_peak_bytes'))} |",
                f"| CPU mean / peak (generation) | {cli.get('cpu_mean_percent', '-')} / {cli.get('cpu_peak_percent', '-')} % |",
                f"| model file / weights / kv cache | {_fmt_bytes(cli.get('model_file_bytes'))} / {_fmt_bytes(cli.get('scalar_weight_bytes'))} / {_fmt_bytes(cli.get('kv_cache_bytes'))} |",
            ]
            batches = cli.get("batch_benchmarks", [])
            if batches:
                lines += [
                    "",
                    "| Engine batch | aggregate tok/s | per-stream tok/s | step latency | stream-0 parity | peak RSS | CPU mean / peak | status |",
                    "| --- | --- | --- | --- | --- | --- | --- | --- |",
                ]
                for batch in batches:
                    average_ns = batch.get("average_step_ns")
                    step_ms = f"{average_ns / 1e6:.2f} ms" if average_ns else "-"
                    lines.append(
                        f"| {batch.get('streams')} | "
                        f"{batch.get('aggregate_tokens_per_second', '-')} | "
                        f"{batch.get('per_stream_tokens_per_second', '-')} | "
                        f"{step_ms} | {batch.get('stream_0_matches_single', '-')} | "
                        f"{_fmt_bytes(batch.get('rss_peak_bytes'))} | "
                        f"{batch.get('cpu_mean_percent', '-')} / "
                        f"{batch.get('cpu_peak_percent', '-')} % | "
                        f"{batch.get('status')} |"
                    )
        for key, title in (
            ("server", "Server"),
            ("batched_server", "Continuous-batched server"),
        ):
            server = entry.get(key)
            if server:
                lines += [
                    "",
                    f"| {title} metric | value |",
                    "| --- | --- |",
                    f"| status | {server.get('status')} |",
                    f"| inference threads | {server.get('inference_threads', server.get('configured_inference_threads', '-'))} |",
                    f"| workload | {server.get('workload', report['config'].get('server_workload', 'identical'))} |",
                    f"| KV backend | {server.get('kv_backend', report['config'].get('server_kv_backend', 'vec'))} |",
                    f"| KV block / token cap | {server.get('kv_tokens_per_block', '-')} / {server.get('kv_max_tokens', '-')} |",
                    f"| configured prompts | {server.get('workload_prompt_count', server.get('openai_http_configured_prompts', 1))} |",
                    f"| concurrency | {server.get('concurrency', '-')} |",
                    f"| first response TTFT | {server.get('streaming_time_to_first_token_ms', '-')} ms |",
                    f"| TTFT p50 / p95 | {server.get('streaming_time_to_first_token_p50_ms', '-')} / {server.get('streaming_time_to_first_token_p95_ms', '-')} ms |",
                    f"| first-stream tok/s | {server.get('streaming_tokens_per_second', '-')} |",
                    f"| aggregate completion tok/s | {server.get('aggregate_completion_tokens_per_second', '-')} |",
                    f"| all request token-ID traces match | {server.get('streaming_all_token_id_traces_match', '-')} |",
                    f"| per-prompt token-ID traces stable | {server.get('streaming_all_prompt_token_id_traces_stable', '-')} |",
                    f"| token IDs match default | {server.get('token_ids_match_default', '-')} |",
                    f"| token latency p50 / p95 | {server.get('streaming_token_latency_p50_ms', '-')} / {server.get('streaming_token_latency_p95_ms', '-')} ms |",
                    f"| requests/s | {server.get('requests_per_second', '-')} |",
                    f"| server RSS peak | {_fmt_bytes(server.get('server_rss_peak_bytes'))} |",
                    f"| server CPU mean / peak | {server.get('server_cpu_mean_percent', '-')} / {server.get('server_cpu_peak_percent', '-')} % |",
                ]
                if server.get("soak_rounds") is not None:
                    lines += [
                        f"| soak warm-up requests | {server.get('soak_warmup_requests')} |",
                        f"| soak rounds / requests | {server.get('soak_rounds')} / {server.get('soak_requests')} |",
                        f"| soak exact traces stable | {server.get('soak_all_token_traces_match')} |",
                        f"| soak memory gate / stable | {server.get('soak_memory_stability_metric', 'rss')} / {server.get('soak_memory_stable', server.get('soak_rss_stable'))} |",
                        f"| soak RSS stable | {server.get('soak_rss_stable')} |",
                        f"| soak RSS growth / tail range | {_fmt_bytes(server.get('soak_rss_growth_bytes'))} / {_fmt_bytes(server.get('soak_rss_tail_range_bytes'))} |",
                    ]
                    if server.get("soak_physical_footprint_idle_bytes") is not None:
                        lines += [
                            f"| soak physical footprint stable | {server.get('soak_physical_footprint_stable')} |",
                            f"| soak physical footprint growth / tail range | {_fmt_bytes(server.get('soak_physical_footprint_growth_bytes'))} / {_fmt_bytes(server.get('soak_physical_footprint_tail_range_bytes'))} |",
                        ]
    lines.append("")
    return "\n".join(lines)


def output_stem(timestamp, model_paths):
    stem = Path(model_paths[0]).stem.lower().replace(" ", "-")
    if len(model_paths) > 1:
        stem += "-multi"
    return f"{timestamp}-{stem}"


def atomic_write_text(path, content):
    """Replace one artifact atomically so interruption cannot truncate it."""
    temporary_path = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            delete=False,
        ) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(content)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def unique_output_stem(base_stem, output_dir=EVALS_DIR):
    """Return a stem that cannot overwrite an existing raw artifact pair."""
    candidate = base_stem
    suffix = 2
    while any(
        (output_dir / f"{candidate}{extension}").exists()
        for extension in (".json", ".md")
    ):
        candidate = f"{base_stem}-{suffix}"
        suffix += 1
    return candidate


def write_outputs(report, stem):
    EVALS_DIR.mkdir(parents=True, exist_ok=True)
    stem = unique_output_stem(stem, EVALS_DIR)
    json_path = EVALS_DIR / f"{stem}.json"
    md_path = EVALS_DIR / f"{stem}.md"
    atomic_write_text(json_path, json.dumps(report, indent=2) + "\n")
    atomic_write_text(md_path, render_markdown(report))
    return json_path, md_path


def release_binaries():
    release = REPO_ROOT / "target" / "release"
    return {
        "ferrite": release / "ferrite",
        "server": release / "ferrite-server",
        "throughput": release / "ferrite-openai-throughput",
    }


def existing_release_binaries():
    bins = release_binaries()
    missing = [str(path) for path in bins.values() if not path.is_file()]
    if missing:
        raise SystemExit(
            "--skip-build requires existing release binaries: " + ", ".join(missing)
        )
    return bins


def build_binaries(server_kv_backend="vec"):
    cmd = [
        "cargo",
        "build",
        "--release",
        "--locked",
        "-p",
        "ferrite-cli",
        "-p",
        "ferrite-server",
    ]
    if server_kv_backend == "locus":
        cmd += ["--features", "ferrite-server/locus-kv"]
    print("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, cwd=REPO_ROOT)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)
    return existing_release_binaries()


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Ferrite evaluation harness: tokens/s, TTFT, memory, CPU."
    )
    parser.add_argument(
        "--model", action="append",
        help="path to a .gguf model (repeatable); default: target/models/*.gguf",
    )
    parser.add_argument("--prompt", default=DEFAULT_PROMPT)
    parser.add_argument("--generate-tokens", type=int, default=64)
    parser.add_argument("--benchmark-runs", type=int, default=64)
    parser.add_argument(
        "--kernel-provider",
        choices=("auto", "portable"),
        default="auto",
        help="CPU kernel provider used by CLI and server phases",
    )
    parser.add_argument(
        "--threads",
        type=int,
        default=None,
        metavar="N",
        help="explicit inference thread count used by CLI and server phases",
    )
    parser.add_argument(
        "--experimental-residual-q8-activation-matvec",
        action="store_true",
        help="benchmark Ferrite's opt-in residual-Q8/I8MM activation matvec policy",
    )
    parser.add_argument(
        "--experimental-q8-k-activation-matvec",
        action="store_true",
        help="benchmark Ferrite's opt-in one-pass Q8_K activation matvec policy",
    )
    parser.add_argument(
        "--experimental-q8-k-activation-roles",
        default=None,
        metavar="ROLE[,ROLE...]",
        help="restrict the selected activation matvec policy to named projection roles",
    )
    parser.add_argument(
        "--batch-streams",
        action="append",
        type=int,
        default=[],
        metavar="N",
        help="also benchmark an N-stream engine batch (repeatable; N must be >= 2)",
    )
    parser.add_argument(
        "--server-batch-streams",
        type=int,
        default=None,
        metavar="N",
        help="also benchmark opt-in continuous-batched HTTP streaming at concurrency N",
    )
    parser.add_argument(
        "--server-workload",
        choices=SERVER_WORKLOADS,
        default="identical",
        help="server prompt topology; default: identical",
    )
    parser.add_argument(
        "--server-prefix-cache",
        action="store_true",
        help="enable, prewarm, trace, and measure bounded prefix reuse in server phases",
    )
    parser.add_argument(
        "--server-kv-backend",
        choices=("vec", "locus"),
        default="vec",
        help="server KV backend; locus requires explicit block and token bounds",
    )
    parser.add_argument(
        "--server-kv-tokens-per-block",
        type=int,
        default=None,
        metavar="N",
        help="Locus server KV block granularity; default: 16",
    )
    parser.add_argument(
        "--server-kv-max-tokens",
        type=int,
        default=None,
        metavar="N",
        help="required per-session token capacity for the Locus server KV backend",
    )
    parser.add_argument(
        "--sleep-ms", type=int, default=2000,
        help="post-load pause used to sample the load-only RSS footprint",
    )
    parser.add_argument("--requests", type=int, default=4, help="server phase request count")
    parser.add_argument(
        "--server-soak-rounds",
        type=int,
        default=0,
        help="repeat the server cohort N additional times; 0 disables soak",
    )
    parser.add_argument(
        "--server-soak-idle-ms",
        type=int,
        default=500,
        help="idle delay before each post-round RSS sample",
    )
    parser.add_argument(
        "--server-soak-rss-tolerance-mib",
        type=int,
        default=16,
        help="maximum accepted RSS growth and tail range in MiB",
    )
    parser.add_argument("--skip-cli", action="store_true")
    parser.add_argument("--skip-server", action="store_true")
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="reuse existing release binaries; intended for a suite that just built them",
    )
    parser.add_argument("--download", action="store_true",
                        help="download the reference model without asking")
    parser.add_argument("--no-download", action="store_true",
                        help="never download; exit 2 if no model is available")
    parser.add_argument("--tag", default=None,
                        help="free-form label recorded in the output (e.g. locus-kv)")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    if args.generate_tokens < 1 or args.benchmark_runs < 1:
        raise SystemExit("token and benchmark counts must be positive")
    if args.requests < 1:
        raise SystemExit("--requests must be positive")
    if args.threads is not None and args.threads < 1:
        raise SystemExit("--threads must be positive")
    if args.server_soak_rounds != 0 and args.server_soak_rounds < 3:
        raise SystemExit("--server-soak-rounds must be 0 or at least 3")
    if args.server_soak_idle_ms < 1:
        raise SystemExit("--server-soak-idle-ms must be positive")
    if args.server_soak_rss_tolerance_mib < 0:
        raise SystemExit("--server-soak-rss-tolerance-mib must not be negative")
    if args.server_kv_backend == "locus":
        if args.server_kv_max_tokens is None:
            raise SystemExit(
                "--server-kv-backend locus requires --server-kv-max-tokens N"
            )
        if args.server_kv_max_tokens < 1:
            raise SystemExit("--server-kv-max-tokens must be positive")
        if (
            args.server_kv_tokens_per_block is not None
            and args.server_kv_tokens_per_block < 1
        ):
            raise SystemExit("--server-kv-tokens-per-block must be positive")
    elif (
        args.server_kv_tokens_per_block is not None
        or args.server_kv_max_tokens is not None
    ):
        raise SystemExit("server KV sizing requires --server-kv-backend locus")
    if (
        args.experimental_q8_k_activation_matvec
        and args.experimental_residual_q8_activation_matvec
    ):
        raise SystemExit("activation matvec policies are mutually exclusive")
    if args.experimental_q8_k_activation_roles and not (
        args.experimental_q8_k_activation_matvec
        or args.experimental_residual_q8_activation_matvec
    ):
        raise SystemExit(
            "--experimental-q8-k-activation-roles requires an activation matvec policy"
        )
    if (
        args.experimental_q8_k_activation_matvec
        or args.experimental_q8_k_activation_roles
    ) and not args.skip_server:
        raise SystemExit(
            "the server does not expose the selected activation policy; use --skip-server"
        )
    if any(streams < 2 for streams in args.batch_streams):
        raise SystemExit("--batch-streams values must be at least 2")
    if args.server_batch_streams is not None and args.server_batch_streams < 2:
        raise SystemExit("--server-batch-streams must be at least 2")
    if args.server_batch_streams is not None and args.requests < args.server_batch_streams:
        raise SystemExit("--requests must be at least --server-batch-streams")
    mixed_budget_count = len(
        server_workload_token_budgets(args.server_workload, args.generate_tokens)
    )
    if args.requests < mixed_budget_count:
        raise SystemExit(
            "--requests must cover every configured mixed-length token budget"
        )
    models = resolve_models(args)
    bins = (
        existing_release_binaries()
        if args.skip_build
        else build_binaries(args.server_kv_backend)
    )
    env = capture_env()
    env["binary_build_mode"] = "prebuilt" if args.skip_build else "cargo-build"
    cfg = EvalConfig(
        prompt=args.prompt,
        generate_tokens=args.generate_tokens,
        benchmark_runs=args.benchmark_runs,
        sleep_ms=args.sleep_ms,
        requests=args.requests,
        batch_streams=tuple(dict.fromkeys(args.batch_streams)),
        server_batch_streams=args.server_batch_streams,
        server_workload=args.server_workload,
        experimental_residual_q8_activation_matvec=args.experimental_residual_q8_activation_matvec,
        experimental_q8_k_activation_matvec=args.experimental_q8_k_activation_matvec,
        experimental_q8_k_activation_roles=args.experimental_q8_k_activation_roles,
        server_soak_rounds=args.server_soak_rounds,
        server_soak_idle_ms=args.server_soak_idle_ms,
        server_soak_rss_tolerance_bytes=args.server_soak_rss_tolerance_mib << 20,
        server_prefix_cache=args.server_prefix_cache,
        kernel_provider=args.kernel_provider,
        server_kv_backend=args.server_kv_backend,
        server_kv_tokens_per_block=(
            (args.server_kv_tokens_per_block or 16)
            if args.server_kv_backend == "locus"
            else None
        ),
        server_kv_max_tokens=args.server_kv_max_tokens,
        threads=args.threads,
    )
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d-%H%M%S")
    model_results = []
    for model_path in models:
        print(f"\n=== evaluating {model_path.name} ===")
        model_sha256 = sha256_file(model_path)
        entry = {
            "model_path": str(model_path),
            "model_sha256": model_sha256,
        }
        provenance = default_model_provenance(model_sha256)
        if provenance is not None:
            entry["model_source"] = provenance
        if not args.skip_cli:
            print("cli phase: generation + benchmark runs ...")
            entry["cli"] = run_cli_phase(bins["ferrite"], model_path, cfg)
        if not args.skip_server:
            print("server phase: ferrite-server + throughput client ...")
            entry["server"] = run_server_phase(
                bins["server"], bins["throughput"], model_path, cfg
            )
            if cfg.server_batch_streams is not None:
                print(
                    "batched server phase: "
                    f"continuous batching x{cfg.server_batch_streams} ..."
                )
                entry["batched_server"] = run_server_phase(
                    bins["server"],
                    bins["throughput"],
                    model_path,
                    cfg,
                    batch_streams=cfg.server_batch_streams,
                )
                entry["batched_server"]["token_ids_match_default"] = (
                    server_token_traces_match(entry["server"], entry["batched_server"])
                )
        model_results.append(entry)
    report = build_report(env, cfg, model_results, tag=args.tag)
    json_path, md_path = write_outputs(report, output_stem(timestamp, models))
    print()
    print(render_markdown(report))
    print(f"wrote {json_path}")
    print(f"wrote {md_path}")
    return 0 if report_succeeded(report) else 1


if __name__ == "__main__":
    sys.exit(main())
