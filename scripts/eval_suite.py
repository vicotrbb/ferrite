#!/usr/bin/env python3
"""Run Ferrite's repeated, clean-host acceptance evaluation suite.

The suite orchestrates `eval.py`; every measured run remains an ordinary raw
JSON and Markdown eval artifact. A separate manifest groups repetitions,
records preflight state, validates exact token parity, and reports medians plus
the observed range.
"""

import argparse
import hashlib
import json
import os
import platform
import shlex
import statistics
import subprocess
import sys
import tempfile
import time
from collections import defaultdict, namedtuple
from datetime import datetime, timezone
from pathlib import Path

import eval as ferrite_eval


REPO_ROOT = Path(__file__).resolve().parent.parent
EVAL_SCRIPT = REPO_ROOT / "scripts" / "eval.py"
EVALS_DIR = REPO_ROOT / "scripts" / "evals"
DEFAULT_PROMPT = "Write a short story about a rusty robot who learns to sail."
DEFAULT_WORKLOADS = ("identical", "shared-prefix", "distinct", "mixed-length")
DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT = 50.0
PREFLIGHT_LOG_INTERVAL_SECONDS = 60.0

SuiteConfig = namedtuple(
    "SuiteConfig",
    [
        "models",
        "prompt",
        "generate_tokens",
        "benchmark_runs",
        "batch_streams",
        "server_batch_streams",
        "requests",
        "workloads",
        "repetitions",
        "tag_prefix",
        "server_soak_rounds",
        "server_soak_idle_ms",
        "server_soak_rss_tolerance_mib",
        "server_prefix_cache",
        "kernel_provider",
        "server_kv_backend",
        "server_kv_tokens_per_block",
        "server_kv_max_tokens",
        "threads",
    ],
    defaults=[False, "auto", "vec", None, None, None],
)


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_tree_sha256():
    """Hash tracked and relevant untracked source files, excluding raw evals."""
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
    )
    paths = sorted(Path(raw.decode()) for raw in result.stdout.split(b"\0") if raw)
    digest = hashlib.sha256()
    for relative in paths:
        if relative.parts[:2] == ("scripts", "evals"):
            continue
        absolute = REPO_ROOT / relative
        if not absolute.is_file():
            continue
        digest.update(str(relative).encode("utf-8"))
        digest.update(b"\0")
        with absolute.open("rb") as source:
            for chunk in iter(lambda: source.read(1 << 20), b""):
                digest.update(chunk)
        digest.update(b"\0")
    return digest.hexdigest()


def atomic_write_text(path, content):
    """Replace one checkpoint atomically so interruption cannot truncate it."""
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


def unique_manifest_path(timestamp, output_dir=EVALS_DIR):
    """Return a manifest path that cannot overwrite an existing artifact."""
    base_stem = f"{timestamp}-acceptance-suite"
    candidate = output_dir / f"{base_stem}.json"
    suffix = 2
    while candidate.exists():
        candidate = output_dir / f"{base_stem}-{suffix}.json"
        suffix += 1
    return candidate


def host_identity():
    """Return stable host and Python fields that affect comparable evidence."""
    environment = ferrite_eval.capture_env()
    return {
        "hostname": environment.get("hostname"),
        "platform": environment.get("platform"),
        "machine": platform.machine(),
        "cpu": environment.get("cpu"),
        "physical_cores": environment.get("physical_cores"),
        "logical_cores": environment.get("logical_cores"),
        "ram_bytes": environment.get("ram_bytes"),
        "python": environment.get("python"),
        "python_implementation": platform.python_implementation(),
    }


def file_stamp(path):
    stat = Path(path).stat()
    return {
        "device": stat.st_dev,
        "inode": stat.st_ino,
        "size": stat.st_size,
        "mtime_ns": stat.st_mtime_ns,
    }


def capture_file_stamps(paths):
    return {
        str(Path(path).resolve()): file_stamp(path)
        for path in paths
    }


def ensure_file_stamps_unchanged(stamps):
    for raw_path, expected in stamps.items():
        path = Path(raw_path)
        try:
            observed = file_stamp(path)
        except OSError as error:
            raise RuntimeError(f"pinned input file became unavailable: {path}") from error
        if observed != expected:
            raise RuntimeError(f"pinned input file changed during evaluation: {path}")


def thermal_status():
    if sys.platform != "darwin":
        return None
    result = subprocess.run(
        ["pmset", "-g", "therm"],
        capture_output=True,
        text=True,
        timeout=10,
    )
    if result.returncode != 0:
        return None
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def thermal_warning(lines):
    if not lines:
        return False
    return any(not line.lower().startswith("note: no ") for line in lines)


def process_cpu_usage():
    try:
        result = subprocess.run(
            ["ps", "-Ao", "pid=,pcpu=,comm="],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    current_pid = os.getpid()
    processes = []
    for line in result.stdout.splitlines():
        fields = line.strip().split(maxsplit=2)
        if len(fields) != 3:
            continue
        raw_pid, raw_cpu, command = fields
        try:
            pid = int(raw_pid)
            cpu_percent = float(raw_cpu)
        except ValueError:
            continue
        if pid == current_pid or cpu_percent < 0:
            continue
        processes.append(
            {
                "pid": pid,
                "cpu_percent": cpu_percent,
                "command": Path(command).name,
            }
        )
    processes.sort(key=lambda process: process["cpu_percent"], reverse=True)
    return {
        "total_percent": round(
            sum(process["cpu_percent"] for process in processes), 3
        ),
        "max_percent": processes[0]["cpu_percent"] if processes else 0.0,
        "top_processes": processes[:5],
    }


def ferrite_processes():
    try:
        result = subprocess.run(
            ["ps", "-Ao", "pid=,comm="],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    processes = []
    for line in result.stdout.splitlines():
        fields = line.strip().split(maxsplit=1)
        if len(fields) != 2:
            continue
        pid, command = fields
        if Path(command).name in {"ferrite", "ferrite-server", "llama-server"}:
            processes.append({"pid": int(pid), "command": command})
    return processes


def host_snapshot():
    logical_cores = os.cpu_count() or 1
    try:
        load_1m, load_5m, load_15m = os.getloadavg()
    except (AttributeError, OSError):
        load_1m = load_5m = load_15m = None
    process_cpu = process_cpu_usage()
    runtime_processes = ferrite_processes()
    return {
        "timestamp_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "logical_cores": logical_cores,
        "load_average_1m": load_1m,
        "load_average_5m": load_5m,
        "load_average_15m": load_15m,
        "load_per_core_1m": None if load_1m is None else load_1m / logical_cores,
        "ferrite_processes": runtime_processes,
        "process_observation_available": (
            process_cpu is not None and runtime_processes is not None
        ),
        "thermal_status": thermal_status(),
        "background_process_cpu_total_percent": (
            process_cpu.get("total_percent") if process_cpu else None
        ),
        "max_background_process_cpu_percent": (
            process_cpu.get("max_percent") if process_cpu else None
        ),
        "top_background_cpu_processes": (
            process_cpu.get("top_processes") if process_cpu else None
        ),
    }


def host_rejection_reasons(
    snapshot,
    max_load_per_core,
    max_background_process_cpu_percent=DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT,
    check_load=True,
):
    reasons = []
    if snapshot.get("process_observation_available") is False:
        reasons.append("host process observation is unavailable")
    load = snapshot.get("load_per_core_1m")
    if check_load and load is not None and load > max_load_per_core:
        reasons.append(
            f"one-minute load per core {load:.3f} exceeds {max_load_per_core:.3f}"
        )
    if snapshot.get("ferrite_processes"):
        reasons.append("an inference runtime process from another run is still active")
    background_cpu = snapshot.get("max_background_process_cpu_percent")
    if (
        background_cpu is not None
        and background_cpu > max_background_process_cpu_percent
    ):
        top_processes = snapshot.get("top_background_cpu_processes") or []
        process_detail = ""
        if top_processes:
            top = top_processes[0]
            process_detail = f" ({top['command']} pid {top['pid']})"
        reasons.append(
            f"background process CPU {background_cpu:.1f}% exceeds "
            f"{max_background_process_cpu_percent:.1f}%{process_detail}"
        )
    if thermal_warning(snapshot.get("thermal_status")):
        reasons.append("the operating system reports thermal or performance pressure")
    return reasons


def wait_for_clean_host(
    max_load_per_core,
    timeout_seconds,
    poll_seconds,
    max_background_process_cpu_percent=DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT,
):
    deadline = time.monotonic() + timeout_seconds
    next_log_at = None
    while True:
        snapshot = host_snapshot()
        reasons = host_rejection_reasons(
            snapshot,
            max_load_per_core,
            max_background_process_cpu_percent=max_background_process_cpu_percent,
        )
        if not reasons:
            return snapshot
        if snapshot.get("process_observation_available") is False:
            raise RuntimeError("; ".join(reasons))
        now = time.monotonic()
        if now >= deadline:
            raise RuntimeError("; ".join(reasons))
        if next_log_at is None or now >= next_log_at:
            print("host preflight rejected: " + "; ".join(reasons), flush=True)
            next_log_at = now + PREFLIGHT_LOG_INTERVAL_SECONDS
        time.sleep(poll_seconds)


def build_cases(config):
    """Build a deterministic, order-balanced list of eval.py invocations."""
    templates = [("cli", None)] + [("server", workload) for workload in config.workloads]
    cases = []
    for repetition in range(1, config.repetitions + 1):
        offset = (repetition - 1) % len(templates)
        ordered = templates[offset:] + templates[:offset]
        for kind, workload in ordered:
            label = kind if workload is None else f"server-{workload}"
            command = [sys.executable, str(EVAL_SCRIPT)]
            for model in config.models:
                command += ["--model", str(model)]
            command += [
                "--prompt",
                config.prompt,
                "--generate-tokens",
                str(config.generate_tokens),
                "--benchmark-runs",
                str(config.benchmark_runs),
                "--tag",
                f"{config.tag_prefix}-{label}-r{repetition}",
                "--skip-build",
            ]
            if config.kernel_provider != "auto":
                command += ["--kernel-provider", config.kernel_provider]
            if config.threads is not None:
                command += ["--threads", str(config.threads)]
            if kind == "cli":
                command.append("--skip-server")
                for streams in config.batch_streams:
                    command += ["--batch-streams", str(streams)]
            else:
                command += [
                    "--skip-cli",
                    "--server-workload",
                    workload,
                    "--server-batch-streams",
                    str(config.server_batch_streams),
                    "--requests",
                    str(config.requests),
                    "--server-kv-backend",
                    config.server_kv_backend,
                ]
                if config.server_kv_backend == "locus":
                    command += [
                        "--server-kv-tokens-per-block",
                        str(config.server_kv_tokens_per_block),
                        "--server-kv-max-tokens",
                        str(config.server_kv_max_tokens),
                    ]
                if workload == "identical" and config.server_soak_rounds:
                    command += [
                        "--server-soak-rounds",
                        str(config.server_soak_rounds),
                        "--server-soak-idle-ms",
                        str(config.server_soak_idle_ms),
                        "--server-soak-rss-tolerance-mib",
                        str(config.server_soak_rss_tolerance_mib),
                    ]
                if config.server_prefix_cache:
                    command.append("--server-prefix-cache")
            cases.append(
                {
                    "kind": kind,
                    "workload": workload,
                    "label": label,
                    "repetition": repetition,
                    "command": command,
                }
            )
    return cases


def case_key(case):
    return f"{case['label']}:r{case['repetition']}"


def case_identity(case, order_index):
    return {
        "key": case_key(case),
        "order_index": order_index,
        "kind": case["kind"],
        "workload": case["workload"],
        "label": case["label"],
        "repetition": case["repetition"],
        "command": case["command"],
    }


def release_binary_identity():
    binaries = ferrite_eval.release_binaries()
    missing = [str(path) for path in binaries.values() if not path.is_file()]
    if missing:
        raise ValueError("release binary file(s) not found: " + ", ".join(missing))
    return {
        name: {"path": str(path.resolve()), "sha256": sha256_file(path)}
        for name, path in sorted(binaries.items())
    }


def manifest_config(config):
    recorded = config._asdict()
    recorded["models"] = [str(model) for model in config.models]
    recorded["batch_streams"] = list(config.batch_streams)
    recorded["workloads"] = list(config.workloads)
    return recorded


def clean_host_policy(args):
    return {
        "max_load_per_core": args.max_load_per_core,
        "max_background_process_cpu_percent": (
            args.max_background_process_cpu_percent
        ),
        "clean_timeout_seconds": args.clean_timeout_seconds,
        "clean_poll_seconds": args.clean_poll_seconds,
    }


def build_resume_identity(
    config,
    cases,
    model_hashes,
    source_fingerprint,
    current_host_identity,
    policy,
):
    """Build the exact suite identity required before cases may be reused."""
    return {
        "source_tree_sha256": source_fingerprint,
        "host": current_host_identity,
        "model_sha256": model_hashes,
        "config": manifest_config(config),
        "clean_host_policy": policy,
        "release_binaries": release_binary_identity(),
        "cases": [
            case_identity(case, order_index)
            for order_index, case in enumerate(cases)
        ],
    }


def extract_json_artifact(output):
    candidates = []
    for line in output.splitlines():
        if line.startswith("wrote ") and line.endswith(".json"):
            candidates.append(Path(line.removeprefix("wrote ").strip()))
    if len(candidates) != 1 or not candidates[0].is_file():
        raise RuntimeError("eval.py did not report exactly one JSON artifact")
    return candidates[0].resolve()


def validate_report(report, expected_model_hashes):
    if not isinstance(report, dict):
        raise RuntimeError("eval artifact must be a JSON object")
    schema_version = report.get("schema_version")
    if type(schema_version) is not int or schema_version < 4:
        raise RuntimeError("acceptance suite requires eval schema version 4 or newer")
    models = report.get("models")
    if not isinstance(models, list) or any(
        not isinstance(entry, dict) for entry in models
    ):
        raise RuntimeError("eval artifact models must be a list of objects")
    observed = [entry.get("model_sha256") for entry in models]
    if observed != list(expected_model_hashes):
        raise RuntimeError("eval artifact model hashes do not match suite inputs")


def expected_eval_report_identity(case):
    args = ferrite_eval.parse_args(case["command"][2:])
    config = ferrite_eval.EvalConfig(
        prompt=args.prompt,
        generate_tokens=args.generate_tokens,
        benchmark_runs=args.benchmark_runs,
        sleep_ms=args.sleep_ms,
        requests=args.requests,
        batch_streams=tuple(dict.fromkeys(args.batch_streams)),
        server_batch_streams=args.server_batch_streams,
        server_workload=args.server_workload,
        experimental_residual_q8_activation_matvec=(
            args.experimental_residual_q8_activation_matvec
        ),
        experimental_q8_k_activation_matvec=(
            args.experimental_q8_k_activation_matvec
        ),
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
    normalized_config = json.loads(json.dumps(config._asdict()))
    return {
        "tag": args.tag,
        "config": normalized_config,
        "model_paths": [str(Path(path).resolve()) for path in args.model],
    }


def validate_case_report(report, case, expected_model_hashes):
    validate_report(report, expected_model_hashes)
    expected = expected_eval_report_identity(case)
    environment = report.get("env")
    if (
        not isinstance(environment, dict)
        or environment.get("binary_build_mode") != "prebuilt"
    ):
        raise RuntimeError("eval artifact did not use the suite's prebuilt binaries")
    if report.get("tag") != expected["tag"]:
        raise RuntimeError("eval artifact tag does not match suite case")
    if report.get("config") != expected["config"]:
        raise RuntimeError("eval artifact configuration does not match suite case")
    observed_paths = [
        str(Path(entry.get("model_path", "")).resolve())
        for entry in report["models"]
    ]
    if observed_paths != expected["model_paths"]:
        raise RuntimeError("eval artifact model paths do not match suite case")
    requested_threads = expected["config"].get("threads")
    if requested_threads is not None:
        phase_names = (
            ("cli",)
            if case["kind"] == "cli"
            else ("server", "batched_server")
        )
        for model in report["models"]:
            for phase_name in phase_names:
                actual_threads = model.get(phase_name, {}).get(
                    "inference_threads"
                )
                if actual_threads != requested_threads:
                    raise RuntimeError(
                        "eval artifact runtime thread count does not match "
                        "suite configuration"
                    )


def postflight_rejection_reasons(snapshot, policy):
    return host_rejection_reasons(
        snapshot,
        policy["max_load_per_core"],
        max_background_process_cpu_percent=(
            policy["max_background_process_cpu_percent"]
        ),
        check_load=False,
    )


def load_attempt_artifact(attempt, case, expected_model_hashes):
    recorded_path = attempt.get("artifact")
    recorded_sha256 = attempt.get("artifact_sha256")
    recorded_markdown_path = attempt.get("artifact_markdown")
    recorded_markdown_sha256 = attempt.get("artifact_markdown_sha256")
    if (
        not isinstance(recorded_path, str)
        or not isinstance(recorded_sha256, str)
        or not isinstance(recorded_markdown_path, str)
        or not isinstance(recorded_markdown_sha256, str)
    ):
        raise ValueError("suite attempt has no complete artifact identity")
    resolved_artifacts = []
    for label, raw_path, expected_sha256 in (
        ("JSON", recorded_path, recorded_sha256),
        ("Markdown", recorded_markdown_path, recorded_markdown_sha256),
    ):
        relative = Path(raw_path)
        if relative.is_absolute():
            raise ValueError("suite attempt artifact path must be repository-relative")
        artifact = (REPO_ROOT / relative).resolve()
        try:
            artifact.relative_to(EVALS_DIR.resolve())
        except ValueError as error:
            raise ValueError("suite attempt artifact is outside scripts/evals") from error
        if not artifact.is_file():
            raise ValueError(f"suite attempt {label} artifact is missing: {raw_path}")
        if sha256_file(artifact) != expected_sha256:
            raise ValueError(
                f"suite attempt {label} artifact hash does not match"
            )
        resolved_artifacts.append(artifact)
    artifact = resolved_artifacts[0]
    if resolved_artifacts[1] != artifact.with_suffix(".md"):
        raise ValueError("suite attempt artifact pair does not share one stem")
    try:
        report = json.loads(artifact.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read suite attempt artifact: {error}") from error
    try:
        validate_case_report(report, case, expected_model_hashes)
    except RuntimeError as error:
        raise ValueError(str(error)) from error
    return report


def suite_attempt_is_clean(attempt, case, policy, expected_model_hashes):
    if attempt.get("status") != "clean":
        return False
    preflight = attempt.get("preflight")
    postflight = attempt.get("postflight")
    if not isinstance(preflight, dict) or not isinstance(postflight, dict):
        return False
    if host_rejection_reasons(
        preflight,
        policy["max_load_per_core"],
        max_background_process_cpu_percent=(
            policy["max_background_process_cpu_percent"]
        ),
    ):
        return False
    if postflight_rejection_reasons(postflight, policy):
        return False
    if attempt.get("postflight_rejection_reasons") != []:
        return False
    try:
        report = load_attempt_artifact(attempt, case, expected_model_hashes)
    except ValueError:
        return False
    if case_report_rejection_reasons(case, report):
        return False
    if attempt.get("case_rejection_reasons") != []:
        return False
    return True


def validate_resume_manifest(previous, expected_identity, cases, model_hashes):
    """Validate and return retained attempts, clean cases, chain, and reports."""
    if not isinstance(previous, dict) or previous.get("schema_version") != 2:
        raise ValueError("resume manifest must use acceptance schema version 2")
    if previous.get("resume_identity") != expected_identity:
        raise ValueError("resume manifest identity does not match this suite")
    status = previous.get("status")
    if status not in {"rejected", "running"}:
        if status == "accepted":
            raise ValueError("resume manifest is already accepted")
        raise ValueError("resume manifest status must be rejected or running")
    attempts = previous.get("attempts")
    selected_runs = previous.get("runs")
    if not isinstance(attempts, list) or not isinstance(selected_runs, list):
        raise ValueError("resume manifest attempts and runs must be lists")

    identities = [case_identity(case, index) for index, case in enumerate(cases)]
    expected_by_key = {identity["key"]: identity for identity in identities}
    cases_by_key = {case_key(case): case for case in cases}
    policy = expected_identity["clean_host_policy"]
    attempt_counts = {}
    clean_seen_keys = set()
    previous_order_index = -1
    clean_attempts = []
    for attempt in attempts:
        if not isinstance(attempt, dict):
            raise ValueError("resume manifest contains a malformed attempt")
        key = attempt.get("key")
        expected = expected_by_key.get(key)
        if expected is None:
            raise ValueError("resume manifest contains an unknown suite case")
        if any(attempt.get(field) != expected[field] for field in (
            "order_index",
            "kind",
            "workload",
            "label",
            "repetition",
            "command",
        )):
            raise ValueError("resume manifest suite case identity does not match")
        if attempt["order_index"] < previous_order_index:
            raise ValueError("resume manifest attempts are out of suite order")
        previous_order_index = attempt["order_index"]
        attempt_counts[key] = attempt_counts.get(key, 0) + 1
        if attempt.get("attempt") != attempt_counts[key]:
            raise ValueError("resume manifest contains an invalid attempt number")
        attempt_status = attempt.get("status")
        if attempt_status not in {"waiting", "running", "failed", "rejected", "clean"}:
            raise ValueError("resume manifest contains an invalid attempt status")
        if key in clean_seen_keys:
            raise ValueError("resume manifest retries an already clean suite case")

        preflight = attempt.get("preflight")
        if preflight is not None:
            if not isinstance(preflight, dict) or host_rejection_reasons(
                preflight,
                policy["max_load_per_core"],
                max_background_process_cpu_percent=(
                    policy["max_background_process_cpu_percent"]
                ),
            ):
                raise ValueError("resume manifest contains a rejected case preflight")
        artifact_fields = (
            attempt.get("artifact"),
            attempt.get("artifact_sha256"),
            attempt.get("artifact_markdown"),
            attempt.get("artifact_markdown_sha256"),
        )
        if any(value is None for value in artifact_fields) and any(
            value is not None for value in artifact_fields
        ):
            raise ValueError("resume manifest contains a partial artifact identity")
        attempt_report = None
        if attempt.get("artifact") is not None:
            if preflight is None:
                raise ValueError("resume manifest artifact has no clean preflight")
            attempt_report = load_attempt_artifact(
                attempt, cases_by_key[key], model_hashes
            )
            recomputed_case_reasons = case_report_rejection_reasons(
                cases_by_key[key], attempt_report
            )
            stored_case_reasons = attempt.get("case_rejection_reasons")
            if stored_case_reasons is not None and (
                not isinstance(stored_case_reasons, list)
                or any(not isinstance(reason, str) for reason in stored_case_reasons)
                or stored_case_reasons != recomputed_case_reasons
            ):
                raise ValueError("resume manifest case result does not reproduce")
        postflight = attempt.get("postflight")
        stored_postflight_reasons = attempt.get("postflight_rejection_reasons")
        if postflight is not None:
            if (
                not isinstance(postflight, dict)
                or not isinstance(stored_postflight_reasons, list)
                or any(
                    not isinstance(reason, str)
                    for reason in stored_postflight_reasons
                )
            ):
                raise ValueError("resume manifest contains malformed host evidence")
            if stored_postflight_reasons != postflight_rejection_reasons(
                postflight, policy
            ):
                raise ValueError("resume manifest host evidence does not reproduce")
        elif stored_postflight_reasons is not None:
            raise ValueError("resume manifest contains partial postflight evidence")
        if attempt_status == "clean":
            if not suite_attempt_is_clean(
                attempt, cases_by_key[key], policy, model_hashes
            ):
                raise ValueError(
                    "resume manifest clean case is incomplete or contaminated"
                )
            clean_attempts.append(attempt)
            clean_seen_keys.add(key)
        elif attempt_status in {"failed", "rejected"} and not isinstance(
            attempt.get("error"), str
        ):
            raise ValueError("resume manifest failed attempt has no error")
        if (
            attempt_status in {"failed", "rejected"}
            and attempt_report is not None
            and attempt.get("case_rejection_reasons") is None
        ):
            raise ValueError("resume manifest rejected case has no result reasons")

    selected_by_key = {}
    run_reports = []
    for run in selected_runs:
        if run not in attempts:
            raise ValueError("resume manifest selected case is not a retained attempt")
        key = run.get("key")
        if key in selected_by_key:
            raise ValueError("resume manifest selects a suite case more than once")
        case = cases_by_key.get(key)
        if case is None or not suite_attempt_is_clean(
            run, case, policy, model_hashes
        ):
            raise ValueError("resume manifest selected case is not complete and clean")
        selected_by_key[key] = run
        run_reports.append((case, load_attempt_artifact(run, case, model_hashes)))
    if any(attempt not in selected_runs for attempt in clean_attempts):
        raise ValueError("resume manifest omits a clean case from selected runs")

    chain = previous.get("resume_chain", [])
    if not isinstance(chain, list) or any(not isinstance(path, str) for path in chain):
        raise ValueError("resume manifest contains a malformed resume chain")
    initial_started_utc = previous.get(
        "initial_started_utc", previous.get("started_utc")
    )
    if not isinstance(initial_started_utc, str) or not initial_started_utc:
        raise ValueError("resume manifest has no valid initial start time")
    selected_runs = sorted(selected_by_key.values(), key=lambda run: run["order_index"])
    order_by_key = {
        identity["key"]: identity["order_index"] for identity in identities
    }
    run_reports.sort(key=lambda item: order_by_key[case_key(item[0])])
    return attempts, selected_runs, chain, run_reports


def numeric(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def metric_summary(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {
        "count": len(values),
        "min": min(values),
        "median": statistics.median(values),
        "max": max(values),
    }


def case_model_record(case, model):
    if case["kind"] == "cli":
        phase = model.get("cli", {})
        metrics = {
            "inference_threads": numeric(phase.get("inference_threads")),
            "decode_tokens_per_second_precise": numeric(
                phase.get("decode_tokens_per_second_precise")
            ),
            "ttft_prefill_seconds": numeric(phase.get("ttft_prefill_seconds")),
            "rss_post_load_bytes": numeric(phase.get("rss_post_load_bytes")),
        }
        parity = phase.get("status") == "ok" and phase.get("benchmark_token_ids") is not None
        for batch in phase.get("batch_benchmarks", []):
            streams = batch.get("streams")
            metrics[f"batch_{streams}_aggregate_tokens_per_second"] = numeric(
                batch.get("aggregate_tokens_per_second")
            )
            metrics[f"batch_{streams}_per_stream_tokens_per_second"] = numeric(
                batch.get("per_stream_tokens_per_second")
            )
            metrics[f"batch_{streams}_rss_peak_bytes"] = numeric(
                batch.get("rss_peak_bytes")
            )
            parity &= batch.get("status") == "ok"
            parity &= batch.get("stream_0_matches_single") is True
        trace = phase.get("benchmark_token_ids")
        required_metrics = ("decode_tokens_per_second_precise", "ttft_prefill_seconds")
    else:
        default = model.get("server", {})
        phase = model.get("batched_server", {})
        metrics = {
            "inference_threads": numeric(phase.get("inference_threads")),
            "aggregate_completion_tokens_per_second": numeric(
                phase.get("aggregate_completion_tokens_per_second")
            ),
            "ttft_p50_ms": numeric(phase.get("streaming_time_to_first_token_p50_ms")),
            "ttft_p95_ms": numeric(phase.get("streaming_time_to_first_token_p95_ms")),
            "server_rss_peak_bytes": numeric(phase.get("server_rss_peak_bytes")),
            "server_cpu_mean_percent": numeric(phase.get("server_cpu_mean_percent")),
            "soak_rss_growth_bytes": numeric(phase.get("soak_rss_growth_bytes")),
            "soak_rss_tail_range_bytes": numeric(
                phase.get("soak_rss_tail_range_bytes")
            ),
            "soak_memory_growth_bytes": numeric(
                phase.get(
                    "soak_memory_growth_bytes", phase.get("soak_rss_growth_bytes")
                )
            ),
            "soak_memory_tail_range_bytes": numeric(
                phase.get(
                    "soak_memory_tail_range_bytes",
                    phase.get("soak_rss_tail_range_bytes"),
                )
            ),
        }
        parity = default.get("status") == "ok" and phase.get("status") == "ok"
        parity &= default.get("streaming_all_prompt_token_id_traces_stable") == "true"
        parity &= phase.get("streaming_all_prompt_token_id_traces_stable") == "true"
        parity &= phase.get("token_ids_match_default") is True
        if phase.get("soak_rounds") is not None:
            parity &= phase.get("soak_all_token_traces_match") is True
            parity &= phase.get(
                "soak_memory_stable", phase.get("soak_rss_stable")
            ) is True
        trace = phase.get("streaming_prompt_token_id_traces")
        required_metrics = (
            "aggregate_completion_tokens_per_second",
            "ttft_p50_ms",
            "ttft_p95_ms",
        )
        if phase.get("soak_rounds") is not None:
            required_metrics += (
                "soak_memory_growth_bytes",
                "soak_memory_tail_range_bytes",
            )
    return {
        "metrics": metrics,
        "parity": bool(parity),
        "trace": json.dumps(trace, separators=(",", ":"), sort_keys=True),
        "required_metrics_present": all(metrics.get(key) is not None for key in required_metrics),
    }


def case_report_rejection_reasons(case, report):
    reasons = []
    for model in report.get("models", []):
        model_sha256 = model.get("model_sha256", "unknown")
        record = case_model_record(case, model)
        if not record["parity"]:
            reasons.append(
                f"model {model_sha256} did not pass exact token and route parity"
            )
        if not record["required_metrics_present"]:
            reasons.append(
                f"model {model_sha256} did not provide every required metric"
            )
    return reasons


def summarize_reports(run_reports, minimum_repetitions=3):
    groups = defaultdict(list)
    for case, report in run_reports:
        for model in report.get("models", []):
            key = (case["label"], model.get("model_sha256"))
            groups[key].append(case_model_record(case, model))

    summaries = []
    for (label, model_sha256), records in sorted(groups.items()):
        metric_names = sorted(
            {name for record in records for name in record["metrics"]}
        )
        metrics = {
            name: summary
            for name in metric_names
            if (summary := metric_summary([record["metrics"].get(name) for record in records]))
            is not None
        }
        parity = all(record["parity"] for record in records)
        stable_trace = len({record["trace"] for record in records}) == 1
        complete = all(record["required_metrics_present"] for record in records)
        summaries.append(
            {
                "case": label,
                "model_sha256": model_sha256,
                "repetitions": len(records),
                "exact_parity": parity,
                "stable_trace_across_repetitions": stable_trace,
                "required_metrics_present": complete,
                "accepted": (
                    len(records) >= minimum_repetitions
                    and parity
                    and stable_trace
                    and complete
                ),
                "metrics": metrics,
            }
        )
    return summaries


def write_manifest(path, manifest):
    path.parent.mkdir(parents=True, exist_ok=True)
    atomic_write_text(path, json.dumps(manifest, indent=2) + "\n")


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Repeated clean-host Ferrite acceptance evaluation suite."
    )
    parser.add_argument("--model", action="append")
    parser.add_argument("--prompt", default=DEFAULT_PROMPT)
    parser.add_argument("--generate-tokens", type=int, default=64)
    parser.add_argument("--benchmark-runs", type=int, default=64)
    parser.add_argument(
        "--kernel-provider", choices=("auto", "portable"), default="auto"
    )
    parser.add_argument(
        "--threads",
        type=int,
        default=None,
        metavar="N",
        help="explicit inference thread count propagated to every child phase",
    )
    parser.add_argument("--batch-streams", action="append", type=int, default=None)
    parser.add_argument("--server-batch-streams", type=int, default=4)
    parser.add_argument("--requests", type=int, default=4)
    parser.add_argument(
        "--workload",
        action="append",
        choices=DEFAULT_WORKLOADS,
        default=None,
    )
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--server-soak-rounds", type=int, default=0)
    parser.add_argument("--server-soak-idle-ms", type=int, default=500)
    parser.add_argument("--server-soak-rss-tolerance-mib", type=int, default=16)
    parser.add_argument(
        "--server-prefix-cache",
        action="store_true",
        help="prewarm and trace bounded prefix reuse in every server case",
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
    parser.add_argument("--tag-prefix", default="acceptance")
    parser.add_argument("--max-load-per-core", type=float, default=0.25)
    parser.add_argument(
        "--max-background-process-cpu-percent",
        type=float,
        default=DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT,
        help="reject preflight when one background process exceeds this ps CPU percentage",
    )
    parser.add_argument("--clean-timeout-seconds", type=int, default=600)
    parser.add_argument("--clean-poll-seconds", type=int, default=15)
    parser.add_argument(
        "--preflight-only",
        action="store_true",
        help="print one clean-host snapshot without requiring a model or building",
    )
    parser.add_argument(
        "--resume-artifact",
        type=Path,
        help=(
            "resume only complete clean cases from a rejected schema-v2 "
            "manifest with identical source, binaries, host, models, and settings"
        ),
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="reuse existing release binaries whose hashes are pinned in the manifest",
    )
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args(argv)


def validate_host_policy(args):
    if args.max_load_per_core <= 0:
        raise ValueError("--max-load-per-core must be positive")
    if args.max_background_process_cpu_percent <= 0:
        raise ValueError("--max-background-process-cpu-percent must be positive")
    if args.clean_timeout_seconds < 0 or args.clean_poll_seconds < 1:
        raise ValueError("clean-host timing values are invalid")


def preflight_report(snapshot, max_load_per_core, max_background_process_cpu_percent):
    reasons = host_rejection_reasons(
        snapshot,
        max_load_per_core,
        max_background_process_cpu_percent=max_background_process_cpu_percent,
    )
    return {
        "schema_version": 1,
        "status": "clean" if not reasons else "rejected",
        "clean_host_policy": {
            "max_load_per_core": max_load_per_core,
            "max_background_process_cpu_percent": (
                max_background_process_cpu_percent
            ),
        },
        "snapshot": snapshot,
        "rejection_reasons": reasons,
    }


def validated_config(args):
    validate_host_policy(args)
    if not args.model:
        raise ValueError("--model is required unless --preflight-only is used")
    models = tuple(Path(model).resolve() for model in args.model)
    missing = [str(model) for model in models if not model.is_file()]
    if missing:
        raise ValueError(f"model file(s) not found: {', '.join(missing)}")
    if args.repetitions < 3:
        raise ValueError("--repetitions must be at least 3")
    if args.threads is not None and args.threads < 1:
        raise ValueError("--threads must be positive")
    if args.server_soak_rounds != 0 and args.server_soak_rounds < 3:
        raise ValueError("--server-soak-rounds must be 0 or at least 3")
    if args.server_soak_idle_ms < 1:
        raise ValueError("--server-soak-idle-ms must be positive")
    if args.server_soak_rss_tolerance_mib < 0:
        raise ValueError("--server-soak-rss-tolerance-mib must not be negative")
    if args.server_kv_backend == "locus":
        if args.server_kv_max_tokens is None:
            raise ValueError(
                "--server-kv-backend locus requires --server-kv-max-tokens N"
            )
        if args.server_kv_max_tokens < 1:
            raise ValueError("--server-kv-max-tokens must be positive")
        if (
            args.server_kv_tokens_per_block is not None
            and args.server_kv_tokens_per_block < 1
        ):
            raise ValueError("--server-kv-tokens-per-block must be positive")
    elif (
        args.server_kv_tokens_per_block is not None
        or args.server_kv_max_tokens is not None
    ):
        raise ValueError("server KV sizing requires --server-kv-backend locus")
    if args.server_batch_streams < 2:
        raise ValueError("--server-batch-streams must be at least 2")
    if args.requests < args.server_batch_streams:
        raise ValueError("--requests must be at least --server-batch-streams")
    if args.generate_tokens < 2 or args.benchmark_runs < 2:
        raise ValueError("token and benchmark counts must be at least 2")
    batch_streams = tuple(dict.fromkeys(args.batch_streams or (4, 8)))
    if any(streams < 2 for streams in batch_streams):
        raise ValueError("--batch-streams values must be at least 2")
    workloads = tuple(dict.fromkeys(args.workload or DEFAULT_WORKLOADS))
    return SuiteConfig(
        models=models,
        prompt=args.prompt,
        generate_tokens=args.generate_tokens,
        benchmark_runs=args.benchmark_runs,
        batch_streams=batch_streams,
        server_batch_streams=args.server_batch_streams,
        requests=args.requests,
        workloads=workloads,
        repetitions=args.repetitions,
        tag_prefix=args.tag_prefix,
        server_soak_rounds=args.server_soak_rounds,
        server_soak_idle_ms=args.server_soak_idle_ms,
        server_soak_rss_tolerance_mib=args.server_soak_rss_tolerance_mib,
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


def main(argv=None):
    args = parse_args(argv)
    if args.resume_artifact is not None:
        args.resume_artifact = args.resume_artifact.resolve()
        if not args.resume_artifact.is_file():
            raise SystemExit(f"resume manifest not found: {args.resume_artifact}")
        if args.preflight_only or args.dry_run:
            raise SystemExit(
                "--resume-artifact cannot be combined with --preflight-only "
                "or --dry-run"
            )
    try:
        validate_host_policy(args)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if args.preflight_only:
        report = preflight_report(
            host_snapshot(),
            args.max_load_per_core,
            args.max_background_process_cpu_percent,
        )
        print(json.dumps(report, indent=2))
        return 0 if report["status"] == "clean" else 1
    try:
        config = validated_config(args)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    cases = build_cases(config)
    if args.dry_run:
        for case in cases:
            print(shlex.join(case["command"]))
        return 0

    if not args.skip_build:
        build_command = [
            "cargo",
            "build",
            "--release",
            "--locked",
            "-p",
            "ferrite-cli",
            "-p",
            "ferrite-server",
        ]
        if config.server_kv_backend == "locus":
            build_command += ["--features", "ferrite-server/locus-kv"]
        print("$ " + shlex.join(build_command), flush=True)
        subprocess.run(build_command, cwd=REPO_ROOT, check=True)

    started = datetime.now(timezone.utc)
    timestamp = started.strftime("%Y-%m-%d-%H%M%S")
    EVALS_DIR.mkdir(parents=True, exist_ok=True)
    manifest_path = unique_manifest_path(timestamp, EVALS_DIR)
    source_fingerprint = source_tree_sha256()
    model_hashes = [sha256_file(model) for model in config.models]
    policy = clean_host_policy(args)
    current_host_identity = host_identity()
    try:
        resume_identity = build_resume_identity(
            config,
            cases,
            model_hashes,
            source_fingerprint,
            current_host_identity,
            policy,
        )
    except ValueError as error:
        raise SystemExit(str(error)) from error
    invocation_file_stamps = capture_file_stamps(
        [
            *config.models,
            *(
                binary["path"]
                for binary in resume_identity["release_binaries"].values()
            ),
        ]
    )

    started_utc = started.strftime("%Y-%m-%dT%H:%M:%SZ")
    attempts = []
    selected_runs = []
    resume_chain = []
    run_reports = []
    initial_started_utc = started_utc
    if args.resume_artifact is not None:
        try:
            previous = json.loads(args.resume_artifact.read_text(encoding="utf-8"))
            attempts, selected_runs, resume_chain, run_reports = (
                validate_resume_manifest(
                    previous,
                    resume_identity,
                    cases,
                    model_hashes,
                )
            )
        except (OSError, ValueError, json.JSONDecodeError) as error:
            raise SystemExit(f"cannot resume acceptance suite: {error}") from error
        resume_chain = [*resume_chain, str(args.resume_artifact)]
        initial_started_utc = previous.get(
            "initial_started_utc", previous.get("started_utc", started_utc)
        )

    manifest = {
        "schema_version": 2,
        "status": "running",
        "started_utc": started_utc,
        "initial_started_utc": initial_started_utc,
        "host": platform.platform(),
        "host_identity": current_host_identity,
        "source_tree_sha256": source_fingerprint,
        "model_sha256": model_hashes,
        "config": manifest_config(config),
        "clean_host_policy": policy,
        "release_binaries": resume_identity["release_binaries"],
        "invocation_file_stamps": invocation_file_stamps,
        "resume_identity": resume_identity,
        "resume_chain": resume_chain,
        "attempts": attempts,
        "runs": selected_runs,
        "summaries": [],
    }
    if args.resume_artifact is not None:
        manifest["resumed_from"] = str(args.resume_artifact)
    write_manifest(manifest_path, manifest)

    execution_error = None
    try:
        selected_keys = {run["key"] for run in manifest["runs"]}
        for order_index, case in enumerate(cases):
            key = case_key(case)
            if key in selected_keys:
                print(
                    f"acceptance suite {key}: reusing complete clean case",
                    flush=True,
                )
                continue
            ensure_file_stamps_unchanged(invocation_file_stamps)
            if source_tree_sha256() != source_fingerprint:
                raise RuntimeError("source tree changed during the acceptance suite")
            attempt_number = 1 + sum(
                attempt.get("key") == key for attempt in manifest["attempts"]
            )
            attempt = {
                **case_identity(case, order_index),
                "attempt": attempt_number,
                "status": "waiting",
            }
            manifest["attempts"].append(attempt)
            write_manifest(manifest_path, manifest)
            try:
                preflight = wait_for_clean_host(
                    policy["max_load_per_core"],
                    policy["clean_timeout_seconds"],
                    policy["clean_poll_seconds"],
                    max_background_process_cpu_percent=(
                        policy["max_background_process_cpu_percent"]
                    ),
                )
                attempt["preflight"] = preflight
                attempt["status"] = "running"
                write_manifest(manifest_path, manifest)
                if source_tree_sha256() != source_fingerprint:
                    raise RuntimeError(
                        "source tree changed during the acceptance suite"
                    )
                ensure_file_stamps_unchanged(invocation_file_stamps)
                print("$ " + shlex.join(case["command"]), flush=True)
                result = subprocess.run(
                    case["command"],
                    cwd=REPO_ROOT,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                )
                try:
                    postflight = host_snapshot()
                finally:
                    # Rendering a complete child report can briefly consume
                    # meaningful CPU in the terminal process. Capture the
                    # postflight state first so the suite cannot contaminate
                    # its own background-process gate.
                    print(result.stdout, end="", flush=True)
                postflight_reasons = postflight_rejection_reasons(
                    postflight, policy
                )
                attempt["returncode"] = result.returncode
                if result.returncode != 0:
                    attempt["status"] = "failed"
                    attempt["output_tail"] = result.stdout[-4000:]
                    write_manifest(manifest_path, manifest)
                artifact = extract_json_artifact(result.stdout)
                try:
                    artifact_relative = artifact.relative_to(REPO_ROOT)
                except ValueError as error:
                    raise RuntimeError(
                        "eval.py artifact is outside the repository"
                    ) from error
                report = json.loads(artifact.read_text(encoding="utf-8"))
                validate_case_report(report, case, model_hashes)
                markdown_artifact = artifact.with_suffix(".md")
                if not markdown_artifact.is_file():
                    raise RuntimeError("eval.py did not produce its Markdown artifact")
                attempt.update(
                    {
                        "artifact": str(artifact_relative),
                        "artifact_sha256": sha256_file(artifact),
                        "artifact_markdown": str(
                            markdown_artifact.relative_to(REPO_ROOT)
                        ),
                        "artifact_markdown_sha256": sha256_file(
                            markdown_artifact
                        ),
                    }
                )
                write_manifest(manifest_path, manifest)
                case_reasons = case_report_rejection_reasons(case, report)
                attempt["postflight"] = postflight
                attempt["postflight_rejection_reasons"] = postflight_reasons
                attempt["case_rejection_reasons"] = case_reasons
                write_manifest(manifest_path, manifest)
                ensure_file_stamps_unchanged(invocation_file_stamps)
                if source_tree_sha256() != source_fingerprint:
                    raise RuntimeError(
                        "source tree changed during the acceptance suite"
                    )
                rejection_reasons = []
                if result.returncode != 0:
                    rejection_reasons.append(
                        f"child eval exited with code {result.returncode}"
                    )
                rejection_reasons.extend(
                    f"case result: {reason}" for reason in case_reasons
                )
                rejection_reasons.extend(
                    f"postflight: {reason}" for reason in postflight_reasons
                )
                if rejection_reasons:
                    raise RuntimeError(
                        f"{case['label']} repetition {case['repetition']} "
                        "rejected: " + "; ".join(rejection_reasons)
                    )
                attempt["status"] = "clean"
                manifest["runs"].append(attempt)
                selected_keys.add(key)
                run_reports.append((case, report))
                write_manifest(manifest_path, manifest)
            except (
                OSError,
                RuntimeError,
                subprocess.SubprocessError,
                json.JSONDecodeError,
            ) as error:
                if attempt.get("status") not in {"failed", "rejected"}:
                    attempt["status"] = "rejected"
                attempt["error"] = str(error)
                write_manifest(manifest_path, manifest)
                raise
    except (
        OSError,
        RuntimeError,
        subprocess.SubprocessError,
        json.JSONDecodeError,
    ) as error:
        execution_error = str(error)

    manifest["summaries"] = summarize_reports(
        run_reports, minimum_repetitions=config.repetitions
    )
    if execution_error is None and source_tree_sha256() != source_fingerprint:
        execution_error = "source tree changed during the acceptance suite"
    if execution_error is not None:
        manifest["status"] = "rejected"
        manifest["error"] = execution_error
    else:
        manifest["status"] = (
            "accepted"
            if len(manifest["runs"]) == len(cases)
            and manifest["summaries"]
            and all(summary["accepted"] for summary in manifest["summaries"])
            else "rejected"
        )
    manifest["finished_utc"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    write_manifest(manifest_path, manifest)
    print(f"wrote {manifest_path}")
    return 0 if manifest["status"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
