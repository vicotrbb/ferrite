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

SCHEMA_VERSION = 1
REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_DIR = REPO_ROOT / "target" / "models"
EVALS_DIR = REPO_ROOT / "scripts" / "evals"
DEFAULT_PROMPT = "Write a short story about a rusty robot who learns to sail."
DEFAULT_MODEL_URL = (
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/"
    "qwen2.5-0.5b-instruct-q4_k_m.gguf"
)
API_KEY = "ferrite-eval"

Sample = namedtuple("Sample", ["t", "rss_bytes", "cpu_seconds"])


def parse_kv_lines(lines):
    """Parse ferrite's key=value stdout lines; last occurrence wins."""
    result = {}
    for line in lines:
        key, sep, value = line.partition("=")
        if sep and key and " " not in key:
            result[key] = value
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
    if len(window) >= 2 and window[-1].t > window[0].t:
        result["cpu_mean_percent"] = round(
            (window[-1].cpu_seconds - window[0].cpu_seconds)
            / (window[-1].t - window[0].t)
            * 100,
            1,
        )
        deltas = [
            (cur.cpu_seconds - prev.cpu_seconds) / (cur.t - prev.t) * 100
            for prev, cur in zip(window, window[1:])
            if cur.t > prev.t
        ]
        if deltas:
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
    ],
)


def run_cli_batch_benchmark(ferrite_bin, model_path, cfg, streams):
    """Measure aggregate decode throughput for one fixed engine batch size."""
    cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--benchmark-runs", str(cfg.benchmark_runs),
        "--benchmark-batch-streams", str(streams),
    ]
    result = {"streams": streams, "command": " ".join(cmd), "status": "ok"}
    run = run_timestamped(cmd)
    if run.returncode != 0:
        result["status"] = "failed"
        result["stderr"] = run.stderr[-2000:]
        return result

    kv = parse_kv_lines(line for _, line in run.lines)
    for source, destination, conversion in (
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
    for key in ("rss_peak_bytes", "cpu_mean_percent", "cpu_peak_percent"):
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
    ]
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
        "model_file_bytes",
        "scalar_weight_bytes",
        "kv_cache_bytes",
        "generated_stopped_on_eos",
    ):
        if key in kv:
            phase[key] = kv[key]

    if t_sleep is not None:
        load_window = aggregate_samples(run.samples, t_sleep, t_gen_start)
        if "rss_peak_bytes" in load_window:
            phase["rss_post_load_bytes"] = load_window["rss_peak_bytes"]
    whole_run = aggregate_samples(run.samples)
    if "rss_peak_bytes" in whole_run:
        phase["rss_peak_bytes"] = whole_run["rss_peak_bytes"]
    if t_gen_start is not None and t_last_stream is not None:
        gen_window = aggregate_samples(run.samples, t_gen_start, t_last_stream)
        for key in ("cpu_mean_percent", "cpu_peak_percent"):
            if key in gen_window:
                phase[key] = gen_window[key]

    bench_cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--benchmark-runs", str(cfg.benchmark_runs),
    ]
    phase["benchmark_command"] = " ".join(bench_cmd)
    bench = run_timestamped(bench_cmd)
    if bench.returncode == 0:
        bench_kv = parse_kv_lines(line for _, line in bench.lines)
        avg_ns = int(bench_kv.get("benchmark_avg_ns", "0"))
        if avg_ns > 0:
            phase["benchmark_avg_ns"] = avg_ns
            phase["decode_tokens_per_second_precise"] = round(1e9 / avg_ns, 2)
    else:
        phase["benchmark_error"] = bench.stderr[-500:]

    if cfg.batch_streams:
        phase["batch_benchmarks"] = [
            run_cli_batch_benchmark(ferrite_bin, model_path, cfg, streams)
            for streams in cfg.batch_streams
        ]
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


def run_server_phase(server_bin, throughput_bin, model_path, cfg):
    """Start ferrite-server, drive it with ferrite-openai-throughput, tear down."""
    phase = {"status": "ok"}
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
    ]
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
            client_cmd = [
                str(throughput_bin),
                "--addr", f"127.0.0.1:{port}",
                "--model", "eval",
                "--endpoint", "chat-completions",
                "--prompt", cfg.prompt,
                "--requests", str(cfg.requests),
                "--max-tokens", str(cfg.generate_tokens),
                "--stream",
                "--stream-usage",
                "--api-key", API_KEY,
            ]
            phase["client_command"] = " ".join(client_cmd)
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
                "requests_per_second",
                "elapsed_ms",
                "streaming_usage_prompt_tokens",
                "streaming_usage_completion_tokens",
            ):
                if key in kv:
                    phase[key] = kv[key]
            request_window = aggregate_samples(
                sampler.samples, t_requests_start, t_requests_end
            )
            for src, dst in (
                ("rss_peak_bytes", "server_rss_peak_bytes"),
                ("cpu_mean_percent", "server_cpu_mean_percent"),
                ("cpu_peak_percent", "server_cpu_peak_percent"),
            ):
                if src in request_window:
                    phase[dst] = request_window[src]
            return phase
        finally:
            sampler.stop()
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()


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


def download_default_model():
    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    target = MODELS_DIR / DEFAULT_MODEL_URL.rsplit("/", 1)[-1]
    print(f"downloading {DEFAULT_MODEL_URL}")
    print(f"        -> {target}")

    def report(blocks, block_size, total):
        done_mb = blocks * block_size / 1e6
        total_mb = total / 1e6 if total > 0 else float("nan")
        print(f"\r  {done_mb:8.1f} / {total_mb:.1f} MB", end="", flush=True)

    partial = target.with_suffix(".gguf.part")
    urllib.request.urlretrieve(DEFAULT_MODEL_URL, partial, reporthook=report)
    partial.rename(target)
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
            f"No models in {MODELS_DIR}. Download the ~400MB reference model "
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


def _fmt_bytes(value):
    try:
        number = int(value)
    except (TypeError, ValueError):
        return "-"
    return f"{number / (1 << 20):.1f} MiB"


def render_markdown(report):
    env = report["env"]
    lines = [
        f"# Ferrite eval — {env.get('timestamp_utc', '?')}",
        "",
        f"- host: {env.get('hostname')} — {env.get('cpu')}, "
        f"{env.get('physical_cores')} cores, {_fmt_bytes(env.get('ram_bytes'))} RAM",
        f"- os: {env.get('platform')}",
        f"- commit: {env.get('git_commit')} ({env.get('git_branch')}"
        f"{', dirty' if env.get('git_dirty') else ''})",
        f"- rustc: {env.get('rustc_version')}",
        f"- config: {json.dumps(report['config'])}",
    ]
    if report.get("tag"):
        lines.append(f"- tag: {report['tag']}")
    for entry in report["models"]:
        lines += ["", f"## {Path(entry['model_path']).name}", ""]
        cli = entry.get("cli")
        if cli:
            lines += [
                "| CLI metric | value |",
                "| --- | --- |",
                f"| status | {cli.get('status')} |",
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
                    "| Engine batch | aggregate tok/s | per-stream tok/s | step latency | peak RSS | CPU mean / peak | status |",
                    "| --- | --- | --- | --- | --- | --- | --- |",
                ]
                for batch in batches:
                    average_ns = batch.get("average_step_ns")
                    step_ms = f"{average_ns / 1e6:.2f} ms" if average_ns else "-"
                    lines.append(
                        f"| {batch.get('streams')} | "
                        f"{batch.get('aggregate_tokens_per_second', '-')} | "
                        f"{batch.get('per_stream_tokens_per_second', '-')} | "
                        f"{step_ms} | {_fmt_bytes(batch.get('rss_peak_bytes'))} | "
                        f"{batch.get('cpu_mean_percent', '-')} / "
                        f"{batch.get('cpu_peak_percent', '-')} % | "
                        f"{batch.get('status')} |"
                    )
        server = entry.get("server")
        if server:
            lines += [
                "",
                "| Server metric | value |",
                "| --- | --- |",
                f"| status | {server.get('status')} |",
                f"| TTFT | {server.get('streaming_time_to_first_token_ms', '-')} ms |",
                f"| streamed tok/s | {server.get('streaming_tokens_per_second', '-')} |",
                f"| token latency p50 / p95 | {server.get('streaming_token_latency_p50_ms', '-')} / {server.get('streaming_token_latency_p95_ms', '-')} ms |",
                f"| requests/s | {server.get('requests_per_second', '-')} |",
                f"| server RSS peak | {_fmt_bytes(server.get('server_rss_peak_bytes'))} |",
                f"| server CPU mean / peak | {server.get('server_cpu_mean_percent', '-')} / {server.get('server_cpu_peak_percent', '-')} % |",
            ]
    lines.append("")
    return "\n".join(lines)


def output_stem(timestamp, model_paths):
    stem = Path(model_paths[0]).stem.lower().replace(" ", "-")
    if len(model_paths) > 1:
        stem += "-multi"
    return f"{timestamp}-{stem}"


def write_outputs(report, stem):
    EVALS_DIR.mkdir(parents=True, exist_ok=True)
    json_path = EVALS_DIR / f"{stem}.json"
    md_path = EVALS_DIR / f"{stem}.md"
    json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(report), encoding="utf-8")
    return json_path, md_path


def build_binaries():
    cmd = ["cargo", "build", "--release", "-p", "ferrite-cli", "-p", "ferrite-server"]
    print("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, cwd=REPO_ROOT)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)
    release = REPO_ROOT / "target" / "release"
    return {
        "ferrite": release / "ferrite",
        "server": release / "ferrite-server",
        "throughput": release / "ferrite-openai-throughput",
    }


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
        "--batch-streams",
        action="append",
        type=int,
        default=[],
        metavar="N",
        help="also benchmark an N-stream engine batch (repeatable; N must be >= 2)",
    )
    parser.add_argument(
        "--sleep-ms", type=int, default=2000,
        help="post-load pause used to sample the load-only RSS footprint",
    )
    parser.add_argument("--requests", type=int, default=4, help="server phase request count")
    parser.add_argument("--skip-cli", action="store_true")
    parser.add_argument("--skip-server", action="store_true")
    parser.add_argument("--download", action="store_true",
                        help="download the reference model without asking")
    parser.add_argument("--no-download", action="store_true",
                        help="never download; exit 2 if no model is available")
    parser.add_argument("--tag", default=None,
                        help="free-form label recorded in the output (e.g. locus-kv)")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    if any(streams < 2 for streams in args.batch_streams):
        raise SystemExit("--batch-streams values must be at least 2")
    models = resolve_models(args)
    bins = build_binaries()
    env = capture_env()
    cfg = EvalConfig(
        prompt=args.prompt,
        generate_tokens=args.generate_tokens,
        benchmark_runs=args.benchmark_runs,
        sleep_ms=args.sleep_ms,
        requests=args.requests,
        batch_streams=tuple(dict.fromkeys(args.batch_streams)),
    )
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d-%H%M%S")
    model_results = []
    for model_path in models:
        print(f"\n=== evaluating {model_path.name} ===")
        entry = {"model_path": str(model_path)}
        if not args.skip_cli:
            print("cli phase: generation + benchmark runs ...")
            entry["cli"] = run_cli_phase(bins["ferrite"], model_path, cfg)
        if not args.skip_server:
            print("server phase: ferrite-server + throughput client ...")
            entry["server"] = run_server_phase(
                bins["server"], bins["throughput"], model_path, cfg
            )
        model_results.append(entry)
    report = build_report(env, cfg, model_results, tag=args.tag)
    json_path, md_path = write_outputs(report, output_stem(timestamp, models))
    print()
    print(render_markdown(report))
    print(f"wrote {json_path}")
    print(f"wrote {md_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
