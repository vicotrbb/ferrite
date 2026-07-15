#!/usr/bin/env python3
"""Compare Ferrite with a pinned CPU-only llama.cpp server.

The comparator uses one GGUF file, prompt, token budget, thread count, and
greedy sampling configuration for both runtimes. It records exact token IDs,
client-observed streaming latency, process RSS and CPU samples, runtime
commands, source identity, and clean-host preflight state.
"""

import argparse
import hashlib
import json
import os
import platform
import shlex
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

import eval as ferrite_eval
import eval_suite


REPO_ROOT = Path(__file__).resolve().parent.parent
EVALS_DIR = REPO_ROOT / "scripts" / "evals"
DEFAULT_LLAMA_SERVER = (
    REPO_ROOT / "target" / "llama.cpp" / "build-cpu" / "bin" / "llama-server"
)
PINNED_LLAMA_CPP_REVISION = "6eddde06a4f25d55d538b5d15628dcc2b6882147"
DEFAULT_PROMPT = "Write one word about iron."
RUNTIMES = ("ferrite", "llama_cpp")
REQUEST_MODES = ("completion", "chat")


def sha256_text(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def llama_revision_matches(version_output, expected_revision):
    expected = expected_revision.strip().lower()
    if len(expected) < 8 or any(
        character not in "0123456789abcdef" for character in expected
    ):
        return False
    lowered = version_output.lower()
    return expected in lowered or expected[:8] in lowered


def ferrite_request(prompt, max_tokens):
    return {
        "model": "eval",
        "prompt": prompt,
        "max_tokens": max_tokens,
        "stream": True,
        "stream_options": {"include_usage": True, "include_obfuscation": False},
        "temperature": 0,
        "top_p": 1,
        "presence_penalty": 0,
        "frequency_penalty": 0,
        "seed": 0,
    }


def ferrite_chat_request(prompt, max_tokens):
    return {
        "model": "eval",
        "messages": [{"role": "user", "content": prompt}],
        "max_completion_tokens": max_tokens,
        "stream": True,
        "stream_options": {"include_usage": True, "include_obfuscation": False},
        "temperature": 0,
        "top_k": 0,
        "top_p": 1,
        "min_p": 0,
        "repetition_penalty": 1,
        "presence_penalty": 0,
        "frequency_penalty": 0,
        "seed": 0,
        "return_token_ids": True,
    }


def llama_cpp_request(prompt, max_tokens):
    return {
        "prompt": prompt,
        "n_predict": max_tokens,
        "stream": True,
        "cache_prompt": False,
        "return_tokens": True,
        "timings_per_token": True,
        "temperature": 0.0,
        "top_k": 0,
        "top_p": 1.0,
        "min_p": 0.0,
        "typical_p": 1.0,
        "repeat_penalty": 1.0,
        "presence_penalty": 0.0,
        "frequency_penalty": 0.0,
        "dry_multiplier": 0.0,
        "seed": 0,
        "samplers": ["temperature"],
    }


def llama_cpp_template_request(prompt):
    return {
        "messages": [{"role": "user", "content": prompt}],
        "add_generation_prompt": True,
    }


def runtime_request(
    runtime, prompt, max_tokens, request_mode="completion", rendered_prompt=None
):
    if runtime == "ferrite":
        if request_mode == "chat":
            return "/v1/chat/completions", ferrite_chat_request(prompt, max_tokens)
        return "/v1/completions", ferrite_request(prompt, max_tokens)
    if runtime == "llama_cpp":
        effective_prompt = rendered_prompt if request_mode == "chat" else prompt
        if request_mode == "chat" and not isinstance(effective_prompt, str):
            raise ValueError("chat comparison requires a rendered llama.cpp prompt")
        request = llama_cpp_request(effective_prompt, max_tokens)
        if request_mode == "chat":
            request["parse_special"] = True
        return "/completion", request
    raise ValueError(f"unknown runtime: {runtime}")


def event_token_ids(runtime, event):
    if runtime == "ferrite":
        choices = event.get("choices")
        if not isinstance(choices, list) or not choices:
            return []
        token_ids = choices[0].get("token_ids", [])
    elif runtime == "llama_cpp":
        token_ids = event.get("tokens", [])
    else:
        raise ValueError(f"unknown runtime: {runtime}")
    if not isinstance(token_ids, list) or any(
        type(token) is not int for token in token_ids
    ):
        raise RuntimeError(f"{runtime} emitted malformed token IDs")
    return token_ids


def event_visible_token_ids(runtime, event, next_event=None):
    token_ids = event_token_ids(runtime, event)
    if runtime == "llama_cpp" and event_content(runtime, event) == "":
        event_is_terminal = normalize_finish_reason(event.get("stop_type")) == "stop"
        next_event_is_terminal = (
            isinstance(next_event, dict)
            and next_event.get("stop") is True
            and normalize_finish_reason(next_event.get("stop_type")) == "stop"
        )
        if event_is_terminal or next_event_is_terminal:
            return []
    return token_ids


def stream_token_chunks(runtime, timestamped_events):
    emitted_chunks = []
    visible_chunks = []
    for index, (timestamp, event) in enumerate(timestamped_events):
        emitted_tokens = event_token_ids(runtime, event)
        if emitted_tokens:
            emitted_chunks.append((timestamp, emitted_tokens))
        next_event = (
            timestamped_events[index + 1][1]
            if index + 1 < len(timestamped_events)
            else None
        )
        visible_tokens = event_visible_token_ids(runtime, event, next_event=next_event)
        if visible_tokens:
            visible_chunks.append((timestamp, visible_tokens))
    return emitted_chunks, visible_chunks


def event_content(runtime, event, request_mode="completion"):
    if runtime == "ferrite":
        choices = event.get("choices")
        if not isinstance(choices, list) or not choices:
            return ""
        if request_mode == "chat":
            delta = choices[0].get("delta")
            content = delta.get("content", "") if isinstance(delta, dict) else ""
        else:
            content = choices[0].get("text", "")
    elif runtime == "llama_cpp":
        content = event.get("content", "")
    else:
        raise ValueError(f"unknown runtime: {runtime}")
    return content if isinstance(content, str) else ""


def normalize_finish_reason(reason):
    if reason == "limit":
        return "length"
    if reason in ("eos", "word"):
        return "stop"
    return reason if isinstance(reason, str) else None


def response_finish_reason(runtime, events):
    if runtime == "ferrite":
        for event in reversed(events):
            choices = event.get("choices")
            if not isinstance(choices, list) or not choices:
                continue
            reason = normalize_finish_reason(choices[0].get("finish_reason"))
            if reason is not None:
                return reason
        return None
    final = events[-1] if events else {}
    return normalize_finish_reason(final.get("stop_type"))


def response_usage(runtime, events):
    if runtime == "ferrite":
        usage = next(
            (
                event.get("usage")
                for event in reversed(events)
                if isinstance(event.get("usage"), dict)
            ),
            {},
        )
        return {
            "prompt_tokens": usage.get("prompt_tokens"),
            "completion_tokens": usage.get("completion_tokens"),
        }
    final = events[-1] if events else {}
    timings = final.get("timings") if isinstance(final.get("timings"), dict) else {}
    return {
        "prompt_tokens": timings.get("prompt_n", final.get("tokens_evaluated")),
        "completion_tokens": timings.get(
            "predicted_n", final.get("tokens_predicted")
        ),
    }


def summarize_stream(
    runtime, events, started, finished, token_chunks, request_mode="completion"
):
    token_ids = [token for _, tokens in token_chunks for token in tokens]
    content = "".join(
        event_content(runtime, event, request_mode=request_mode) for event in events
    )
    timestamps = [timestamp for timestamp, _ in token_chunks]
    chunk_sizes = [len(tokens) for _, tokens in token_chunks]
    result = {
        "token_ids": token_ids,
        "token_count": len(token_ids),
        "token_chunk_count": len(token_chunks),
        "all_token_chunks_nonempty": all(chunk_sizes),
        "content_sha256": sha256_text(content),
        "content_bytes": len(content.encode("utf-8")),
        "total_elapsed_ms": round((finished - started) * 1000, 3),
        "usage": response_usage(runtime, events),
        "finish_reason": response_finish_reason(runtime, events),
    }
    if timestamps:
        result["time_to_first_token_ms"] = round(
            (timestamps[0] - started) * 1000, 3
        )
    if len(timestamps) >= 2:
        latencies = [
            (current - previous) * 1000
            for previous, current in zip(timestamps, timestamps[1:])
        ]
        result["inter_chunk_latency_ms_p50"] = round(
            ferrite_eval.percentile(latencies, 50), 3
        )
        result["inter_chunk_latency_ms_p95"] = round(
            ferrite_eval.percentile(latencies, 95), 3
        )
        decode_seconds = timestamps[-1] - timestamps[0]
        tokens_after_first_chunk = sum(chunk_sizes[1:])
        if decode_seconds > 0 and tokens_after_first_chunk > 0:
            result["post_first_chunk_tokens_per_second"] = round(
                tokens_after_first_chunk / decode_seconds, 6
            )
    return result


def stream_request(
    runtime,
    port,
    prompt,
    max_tokens,
    request_mode="completion",
    rendered_prompt=None,
    timeout_seconds=600,
):
    endpoint, body = runtime_request(
        runtime,
        prompt,
        max_tokens,
        request_mode=request_mode,
        rendered_prompt=rendered_prompt,
    )
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}{endpoint}",
        data=json.dumps(body, separators=(",", ":")).encode("utf-8"),
        headers={"content-type": "application/json"},
        method="POST",
    )
    started = time.monotonic()
    events = []
    timestamped_events = []
    saw_done = False
    try:
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            if response.status != 200:
                raise RuntimeError(f"{runtime} returned HTTP {response.status}")
            for raw_line in response:
                line = raw_line.decode("utf-8").strip()
                if not line.startswith("data: "):
                    continue
                payload = line.removeprefix("data: ")
                if payload == "[DONE]":
                    saw_done = True
                    break
                event = json.loads(payload)
                events.append(event)
                timestamped_events.append((time.monotonic(), event))
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(
            f"{runtime} returned HTTP {error.code}: {detail}"
        ) from error
    finished = time.monotonic()
    if runtime == "ferrite" and not saw_done:
        raise RuntimeError("Ferrite stream ended without [DONE]")
    if not events:
        raise RuntimeError(f"{runtime} stream contained no JSON events")
    runtime_emitted_token_chunks, token_chunks = stream_token_chunks(
        runtime, timestamped_events
    )
    result = summarize_stream(
        runtime,
        events,
        started,
        finished,
        token_chunks,
        request_mode=request_mode,
    )
    result["runtime_emitted_token_ids"] = [
        token
        for _, tokens in runtime_emitted_token_chunks
        for token in tokens
    ]
    result["suppressed_terminal_token_ids"] = (
        len(result["runtime_emitted_token_ids"]) - len(result["token_ids"])
    )
    result["request"] = body
    return result, started, finished


def post_json(port, endpoint, body, timeout_seconds=60):
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}{endpoint}",
        data=json.dumps(body, separators=(",", ":")).encode("utf-8"),
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            if response.status != 200:
                raise RuntimeError(
                    f"llama_cpp returned HTTP {response.status} from {endpoint}"
                )
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(
            f"llama_cpp returned HTTP {error.code} from {endpoint}: {detail}"
        ) from error


def llama_cpp_render_chat_prompt(port, prompt):
    request = llama_cpp_template_request(prompt)
    response = post_json(port, "/apply-template", request)
    rendered = response.get("prompt") if isinstance(response, dict) else None
    if not isinstance(rendered, str) or not rendered:
        raise RuntimeError("llama.cpp /apply-template returned no rendered prompt")
    return rendered, request


def ferrite_server_command(binary, model, port, threads, max_tokens):
    return [
        str(binary),
        "--model",
        str(model),
        "--model-id",
        "eval",
        "--bind",
        f"127.0.0.1:{port}",
        "--default-max-tokens",
        str(max_tokens),
        "--hard-max-tokens",
        str(max_tokens),
        "--threads",
        str(threads),
    ]


def llama_cpp_server_command(binary, model, port, threads, request_mode="completion"):
    command = [
        str(binary),
        "--model",
        str(model),
        "--host",
        "127.0.0.1",
        "--port",
        str(port),
        "--threads",
        str(threads),
        "--threads-batch",
        str(threads),
        "--parallel",
        "1",
        "--n-gpu-layers",
        "0",
        "--no-warmup",
        "--no-cont-batching",
        "--cache-ram",
        "0",
        "--no-cache-prompt",
    ]
    if request_mode == "chat":
        command.append("--jinja")
    return command


def server_command(runtime, args, port):
    if runtime == "ferrite":
        return ferrite_server_command(
            args.ferrite_server, args.model, port, args.threads, args.max_tokens
        )
    if runtime == "llama_cpp":
        return llama_cpp_server_command(
            args.llama_server,
            args.model,
            port,
            args.threads,
            request_mode=args.request_mode,
        )
    raise ValueError(f"unknown runtime: {runtime}")


def run_runtime(runtime, args):
    port = ferrite_eval.find_free_port()
    command = server_command(runtime, args, port)
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as server_log:
        process = subprocess.Popen(
            command,
            cwd=REPO_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
            text=True,
        )
        sampler = ferrite_eval.ProcessSampler(process.pid, interval_s=0.02)
        sampler.start()
        try:
            if not ferrite_eval.wait_for_health(port, process, timeout_s=300):
                server_log.seek(0)
                raise RuntimeError(
                    f"{runtime} server did not become healthy: "
                    + server_log.read()[-4000:]
                )
            time.sleep(0.1)
            before_samples = list(sampler.samples)
            rendered_prompt = None
            template_request = None
            if runtime == "llama_cpp" and args.request_mode == "chat":
                rendered_prompt, template_request = llama_cpp_render_chat_prompt(
                    port, args.prompt
                )
            result, request_started, request_finished = stream_request(
                runtime,
                port,
                args.prompt,
                args.max_tokens,
                request_mode=args.request_mode,
                rendered_prompt=rendered_prompt,
            )
            time.sleep(0.1)
            samples = list(sampler.samples)
            result["command"] = command
            result["request_mode"] = args.request_mode
            if rendered_prompt is not None:
                result["template_request"] = template_request
                result["rendered_prompt_sha256"] = sha256_text(rendered_prompt)
                result["rendered_prompt_bytes"] = len(
                    rendered_prompt.encode("utf-8")
                )
            result["steady_rss_before_bytes"] = (
                before_samples[-1].rss_bytes if before_samples else None
            )
            request_samples = ferrite_eval.aggregate_samples(
                samples, request_started, request_finished
            )
            for source, destination in (
                ("rss_peak_bytes", "request_rss_peak_bytes"),
                ("rss_mean_bytes", "request_rss_mean_bytes"),
                ("cpu_mean_percent", "request_cpu_mean_percent"),
                ("cpu_peak_percent", "request_cpu_peak_percent"),
            ):
                if source in request_samples:
                    result[destination] = request_samples[source]
            return result
        finally:
            sampler.stop()
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)


def metric_summary(values):
    return eval_suite.metric_summary(
        [value for value in values if isinstance(value, (int, float))]
    )


def postflight_rejection_reasons(
    snapshot,
    max_background_process_cpu_percent=(
        eval_suite.DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT
    ),
):
    return eval_suite.host_rejection_reasons(
        snapshot,
        max_load_per_core=float("inf"),
        max_background_process_cpu_percent=max_background_process_cpu_percent,
        check_load=False,
    )


def first_divergence_index(left, right):
    for index, (left_token, right_token) in enumerate(zip(left, right)):
        if left_token != right_token:
            return index
    if len(left) != len(right):
        return min(len(left), len(right))
    return None


def numerical_policy_run_matches(run, case):
    ferrite = run["results"]["ferrite"]
    llama_cpp = run["results"]["llama_cpp"]
    if ferrite.get("token_ids") != case["expected_ferrite_token_ids"]:
        return False
    if llama_cpp.get("token_ids") != case["expected_llama_cpp_token_ids"]:
        return False
    expected_llama_emitted = case.get(
        "expected_llama_cpp_runtime_emitted_token_ids"
    )
    if expected_llama_emitted is not None and (
        llama_cpp.get("runtime_emitted_token_ids") != expected_llama_emitted
    ):
        return False
    expected_ferrite_emitted = case.get(
        "expected_ferrite_runtime_emitted_token_ids"
    )
    if expected_ferrite_emitted is not None and (
        ferrite.get("runtime_emitted_token_ids") != expected_ferrite_emitted
    ):
        return False
    prompt_tokens = case["prompt_tokens"]
    if ferrite.get("usage", {}).get("prompt_tokens") != prompt_tokens:
        return False
    if llama_cpp.get("usage", {}).get("prompt_tokens") != prompt_tokens:
        return False
    if (
        llama_cpp.get("rendered_prompt_sha256")
        != case["rendered_prompt_sha256"]
    ):
        return False
    expected_finish_reason = case.get("expected_finish_reason")
    if expected_finish_reason is not None and (
        ferrite.get("finish_reason") != expected_finish_reason
        or llama_cpp.get("finish_reason") != expected_finish_reason
    ):
        return False
    return True


def summarize_runs(
    runs,
    repetitions,
    max_tokens,
    allow_early_stop=False,
    numerical_policy_case=None,
):
    summaries = {}
    for runtime in RUNTIMES:
        records = [run["results"][runtime] for run in runs]
        traces = [record.get("token_ids") for record in records]
        summaries[runtime] = {
            "repetitions": len(records),
            "stable_token_ids": bool(traces)
            and len({tuple(trace or []) for trace in traces}) == 1,
            "all_token_budgets_complete": all(
                record.get("token_count") == max_tokens for record in records
            ),
            "all_token_counts_within_budget": all(
                isinstance(record.get("token_count"), int)
                and 0 < record["token_count"] <= max_tokens
                for record in records
            ),
            "metrics": {
                name: metric_summary([record.get(name) for record in records])
                for name in (
                    "time_to_first_token_ms",
                    "inter_chunk_latency_ms_p50",
                    "inter_chunk_latency_ms_p95",
                    "post_first_chunk_tokens_per_second",
                    "steady_rss_before_bytes",
                    "request_rss_peak_bytes",
                    "request_cpu_mean_percent",
                )
            },
        }
    exact_pairs = [
        run["results"]["ferrite"].get("token_ids")
        == run["results"]["llama_cpp"].get("token_ids")
        for run in runs
    ]
    prompt_token_pairs = [
        run["results"]["ferrite"].get("usage", {}).get("prompt_tokens")
        == run["results"]["llama_cpp"].get("usage", {}).get("prompt_tokens")
        for run in runs
    ]
    completion_token_pairs = [
        run["results"]["ferrite"].get("usage", {}).get("completion_tokens")
        == run["results"]["llama_cpp"].get("usage", {}).get("completion_tokens")
        for run in runs
    ]
    finish_reason_pairs = [
        run["results"]["ferrite"].get("finish_reason")
        == run["results"]["llama_cpp"].get("finish_reason")
        for run in runs
    ]
    policy_matches = (
        [
            numerical_policy_run_matches(run, numerical_policy_case)
            for run in runs
        ]
        if numerical_policy_case is not None
        else []
    )
    reviewed_policy_match = bool(policy_matches) and all(policy_matches)
    exact_match = bool(exact_pairs) and all(exact_pairs)
    token_budget_gate = all(
        summary[
            "all_token_counts_within_budget"
            if allow_early_stop
            else "all_token_budgets_complete"
        ]
        for summary in summaries.values()
    )
    comparison = {
        "repetitions": len(runs),
        "all_exact_token_id_pairs_match": exact_match,
        "all_prompt_token_counts_match": bool(prompt_token_pairs)
        and all(prompt_token_pairs),
        "all_completion_token_counts_match": bool(completion_token_pairs)
        and all(completion_token_pairs),
        "all_finish_reasons_match": bool(finish_reason_pairs)
        and all(finish_reason_pairs),
        "allow_early_stop": allow_early_stop,
        "reviewed_numerical_policy_match": reviewed_policy_match,
        "numerical_policy_case_id": (
            numerical_policy_case.get("case_id")
            if numerical_policy_case is not None
            else None
        ),
        "accepted_under": (
            "exact_token_ids"
            if exact_match
            else "reviewed_numerical_policy"
            if reviewed_policy_match
            else None
        ),
        "accepted": (
            len(runs) >= repetitions
            and (exact_match or reviewed_policy_match)
            and all(prompt_token_pairs)
            and all(completion_token_pairs)
            and all(finish_reason_pairs)
            and all(summary["stable_token_ids"] for summary in summaries.values())
            and token_budget_gate
        ),
    }
    return {"runtimes": summaries, "comparison": comparison}


def format_metric(summary, unit=""):
    if not summary:
        return "-"
    suffix = f" {unit}" if unit else ""
    return (
        f"{summary['median']:.3f}{suffix} "
        f"[{summary['min']:.3f}, {summary['max']:.3f}]"
    )


def render_markdown(report):
    comparison = report.get("summary", {}).get("comparison", {})
    lines = [
        f"# Ferrite and llama.cpp reference comparison, {report['started_utc']}",
        "",
        f"- status: {report['status']}",
        f"- model SHA-256: `{report['model_sha256']}`",
        f"- prompt SHA-256: `{report['prompt_sha256']}`",
        f"- request mode: {report['config']['request_mode']}",
        f"- token budget: {report['config']['max_tokens']}",
        f"- threads: {report['config']['threads']}",
        f"- llama.cpp revision: `{report['llama_cpp']['expected_revision']}`",
        f"- exact token-ID parity: {comparison.get('all_exact_token_id_pairs_match', False)}",
        f"- reviewed numerical policy match: {comparison.get('reviewed_numerical_policy_match', False)}",
        f"- accepted under: {comparison.get('accepted_under') or '-'}",
        "",
        "| runtime | TTFT median [min, max] | decode tok/s median [min, max] | steady RSS median [min, max] | exact trace stable |",
        "| --- | ---: | ---: | ---: | --- |",
    ]
    runtime_summaries = report.get("summary", {}).get("runtimes", {})
    for runtime in RUNTIMES:
        summary = runtime_summaries.get(runtime, {})
        metrics = summary.get("metrics", {})
        lines.append(
            f"| {runtime} | "
            f"{format_metric(metrics.get('time_to_first_token_ms'), 'ms')} | "
            f"{format_metric(metrics.get('post_first_chunk_tokens_per_second'))} | "
            f"{format_metric(metrics.get('steady_rss_before_bytes'), 'bytes')} | "
            f"{summary.get('stable_token_ids', False)} |"
        )
    if report["config"]["request_mode"] == "chat":
        lines += [
            "",
            "Chat-mode latency and resource samples are diagnostic only. llama.cpp "
            "uses `/apply-template` followed by `/completion` so exact generated "
            "token IDs remain observable.",
        ]
    policy = report.get("numerical_policy")
    if isinstance(policy, dict):
        lines += [
            "",
            f"- numerical policy: `{policy['policy_id']}`",
            f"- numerical policy case: `{policy['case']['case_id']}`",
            f"- numerical policy SHA-256: `{policy['sha256']}`",
        ]
    if report.get("error"):
        lines += ["", f"Rejected reason: {report['error']}"]
    lines.append("")
    return "\n".join(lines)


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


def write_report(report, output_stem):
    EVALS_DIR.mkdir(parents=True, exist_ok=True)
    json_path = EVALS_DIR / f"{output_stem}.json"
    markdown_path = EVALS_DIR / f"{output_stem}.md"
    atomic_write_text(json_path, json.dumps(report, indent=2) + "\n")
    atomic_write_text(markdown_path, render_markdown(report))
    return json_path, markdown_path


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


def runtime_order(repetition):
    """Return the fixed, order-balanced runtime sequence for one repetition."""
    return RUNTIMES if repetition % 2 else tuple(reversed(RUNTIMES))


def comparison_config(args):
    """Return the result-affecting configuration recorded in every artifact."""
    return {
        "prompt": args.prompt,
        "request_mode": args.request_mode,
        "max_tokens": args.max_tokens,
        "threads": args.threads,
        "repetitions": args.repetitions,
        "max_load_per_core": args.max_load_per_core,
        "max_background_process_cpu_percent": (
            args.max_background_process_cpu_percent
        ),
        "allow_early_stop": args.allow_early_stop,
        "performance_comparable": args.request_mode == "completion",
    }


def resume_policy_identity(numerical_policy):
    if numerical_policy is None:
        return None
    return {
        "sha256": numerical_policy["sha256"],
        "policy_id": numerical_policy["policy_id"],
        "case_id": numerical_policy["case"]["case_id"],
    }


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


def build_resume_identity(
    args,
    version,
    source_fingerprint,
    model_sha256,
    prompt_sha256,
    numerical_policy,
    current_host_identity,
):
    """Build the exact identity required before a clean run may be resumed."""
    return {
        "source_tree_sha256": source_fingerprint,
        "host": current_host_identity,
        "model_path": str(args.model),
        "model_sha256": model_sha256,
        "prompt_sha256": prompt_sha256,
        "config": comparison_config(args),
        "ferrite_server": {
            "path": str(args.ferrite_server),
            "sha256": ferrite_eval.sha256_file(args.ferrite_server),
        },
        "llama_cpp": {
            "path": str(args.llama_server),
            "sha256": ferrite_eval.sha256_file(args.llama_server),
            "expected_revision": args.llama_revision,
            "version": version,
        },
        "numerical_policy": resume_policy_identity(numerical_policy),
    }


def run_is_clean(run, config):
    """Return whether a complete pair passed every recorded host gate."""
    repetition = run.get("repetition")
    if type(repetition) is not int or not 1 <= repetition <= config["repetitions"]:
        return False
    if run.get("order") != list(runtime_order(repetition)):
        return False
    results = run.get("results")
    if not isinstance(results, dict) or set(results) != set(RUNTIMES):
        return False
    for runtime in RUNTIMES:
        result = results.get(runtime)
        if not isinstance(result, dict):
            return False
        preflight = result.get("preflight")
        postflight = result.get("postflight")
        if not isinstance(preflight, dict) or not isinstance(postflight, dict):
            return False
        if eval_suite.host_rejection_reasons(
            preflight,
            config["max_load_per_core"],
            max_background_process_cpu_percent=(
                config["max_background_process_cpu_percent"]
            ),
        ):
            return False
        recomputed_postflight_reasons = postflight_rejection_reasons(
            postflight,
            max_background_process_cpu_percent=(
                config["max_background_process_cpu_percent"]
            ),
        )
        if recomputed_postflight_reasons:
            return False
        if result.get("postflight_rejection_reasons") != []:
            return False
    return True


def validate_resume_report(previous, expected_identity):
    """Validate and return retained attempts plus uniquely clean pairs."""
    if not isinstance(previous, dict) or previous.get("schema_version") != 2:
        raise ValueError("resume artifact must use reference schema version 2")
    if previous.get("resume_identity") != expected_identity:
        raise ValueError("resume artifact identity does not match this comparison")
    status = previous.get("status")
    if status not in {"rejected", "running"}:
        if status == "accepted":
            raise ValueError("resume artifact is already accepted")
        raise ValueError("resume artifact status must be rejected or running")
    attempts = previous.get("attempts")
    selected_runs = previous.get("runs")
    if not isinstance(attempts, list) or not isinstance(selected_runs, list):
        raise ValueError("resume artifact attempts and runs must be lists")
    config = expected_identity["config"]
    attempt_counts = {}
    for attempt in attempts:
        if not isinstance(attempt, dict):
            raise ValueError("resume artifact contains a malformed attempt")
        repetition = attempt.get("repetition")
        if type(repetition) is not int or not 1 <= repetition <= config["repetitions"]:
            raise ValueError("resume artifact contains an invalid repetition")
        if attempt.get("order") != list(runtime_order(repetition)):
            raise ValueError("resume artifact contains an invalid runtime order")
        attempt_counts[repetition] = attempt_counts.get(repetition, 0) + 1
        if attempt.get("attempt") != attempt_counts[repetition]:
            raise ValueError("resume artifact contains an invalid attempt number")
        results = attempt.get("results")
        if not isinstance(results, dict) or not set(results).issubset(set(RUNTIMES)):
            raise ValueError("resume artifact contains malformed runtime results")
        completed_prefix = set(attempt["order"][: len(results)])
        if set(results) != completed_prefix:
            raise ValueError("resume artifact contains out-of-order runtime results")
        for result in results.values():
            if not isinstance(result, dict):
                raise ValueError("resume artifact contains a malformed runtime result")
            preflight = result.get("preflight")
            postflight = result.get("postflight")
            stored_postflight_reasons = result.get("postflight_rejection_reasons")
            if (
                not isinstance(preflight, dict)
                or not isinstance(postflight, dict)
                or not isinstance(stored_postflight_reasons, list)
                or any(
                    not isinstance(reason, str)
                    for reason in stored_postflight_reasons
                )
            ):
                raise ValueError("resume artifact contains malformed host evidence")
            if eval_suite.host_rejection_reasons(
                preflight,
                config["max_load_per_core"],
                max_background_process_cpu_percent=(
                    config["max_background_process_cpu_percent"]
                ),
            ):
                raise ValueError(
                    "resume artifact contains a rejected runtime preflight"
                )
            recomputed_postflight_reasons = postflight_rejection_reasons(
                postflight,
                max_background_process_cpu_percent=(
                    config["max_background_process_cpu_percent"]
                ),
            )
            if stored_postflight_reasons != recomputed_postflight_reasons:
                raise ValueError("resume artifact host evidence does not reproduce")
    clean_by_repetition = {}
    for run in selected_runs:
        if run not in attempts:
            raise ValueError(
                "resume artifact selected run is not retained as an attempt"
            )
        if not run_is_clean(run, config):
            raise ValueError(
                "resume artifact selected run is incomplete or contaminated"
            )
        repetition = run["repetition"]
        if repetition in clean_by_repetition:
            raise ValueError("resume artifact selects a repetition more than once")
        clean_by_repetition[repetition] = run
    chain = previous.get("resume_chain", [])
    if not isinstance(chain, list) or any(not isinstance(path, str) for path in chain):
        raise ValueError("resume artifact contains a malformed resume chain")
    initial_started_utc = previous.get(
        "initial_started_utc", previous.get("started_utc")
    )
    if not isinstance(initial_started_utc, str) or not initial_started_utc:
        raise ValueError("resume artifact has no valid initial start time")
    clean_runs = [clean_by_repetition[key] for key in sorted(clean_by_repetition)]
    return attempts, clean_runs, chain


def _validated_token_trace(case, field):
    trace = case.get(field)
    if not isinstance(trace, list) or not trace or any(
        type(token) is not int or token < 0 for token in trace
    ):
        raise ValueError(f"numerical policy {field} must be non-empty token IDs")
    return trace


def load_numerical_policy(path, identity):
    try:
        policy = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read numerical policy {path}: {error}") from error
    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        raise ValueError("numerical policy must use schema_version 1")
    if policy.get("review_status") != "reviewed":
        raise ValueError("numerical policy must have review_status reviewed")
    policy_id = policy.get("policy_id")
    if not isinstance(policy_id, str) or not policy_id.strip():
        raise ValueError("numerical policy must have a non-empty policy_id")
    cases = policy.get("cases")
    if not isinstance(cases, list):
        raise ValueError("numerical policy cases must be a list")
    matches = [
        case
        for case in cases
        if isinstance(case, dict)
        and all(case.get(field) == value for field, value in identity.items())
    ]
    if len(matches) != 1:
        raise ValueError(
            "numerical policy must match exactly one case for this model, prompt, "
            "request mode, token budget, and llama.cpp revision"
        )
    case = matches[0]
    case_id = case.get("case_id")
    if not isinstance(case_id, str) or not case_id.strip():
        raise ValueError("numerical policy case must have a non-empty case_id")
    ferrite_trace = _validated_token_trace(case, "expected_ferrite_token_ids")
    llama_trace = _validated_token_trace(case, "expected_llama_cpp_token_ids")
    for optional_trace in (
        "expected_ferrite_runtime_emitted_token_ids",
        "expected_llama_cpp_runtime_emitted_token_ids",
    ):
        if optional_trace in case:
            _validated_token_trace(case, optional_trace)
    divergence = first_divergence_index(ferrite_trace, llama_trace)
    if divergence is None or case.get("first_divergence_index") != divergence:
        raise ValueError(
            "numerical policy must record the exact first divergent token index"
        )
    prompt_tokens = case.get("prompt_tokens")
    if type(prompt_tokens) is not int or prompt_tokens < 1:
        raise ValueError("numerical policy prompt_tokens must be positive")
    rendered_hash = case.get("rendered_prompt_sha256")
    if (
        not isinstance(rendered_hash, str)
        or len(rendered_hash) != 64
        or any(character not in "0123456789abcdef" for character in rendered_hash)
    ):
        raise ValueError("numerical policy rendered prompt hash is invalid")
    logit_gap = case.get("recorded_absolute_logit_gap")
    if not isinstance(logit_gap, (int, float)) or not 0 <= logit_gap <= 0.001:
        raise ValueError(
            "numerical policy recorded logit gap must be between 0 and 0.001"
        )
    if case.get("decision") != "accept_exact_recorded_near_tie_trace":
        raise ValueError("numerical policy decision is not recognized")
    return {
        "path": str(path),
        "sha256": ferrite_eval.sha256_file(path),
        "policy_id": policy_id,
        "review_status": policy["review_status"],
        "case": case,
    }


def parse_args(argv=None):
    parser = argparse.ArgumentParser(
        description="Repeated clean-host exact-token comparison with pinned llama.cpp."
    )
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--llama-server", type=Path, default=DEFAULT_LLAMA_SERVER)
    parser.add_argument("--llama-revision", default=PINNED_LLAMA_CPP_REVISION)
    parser.add_argument(
        "--ferrite-server",
        type=Path,
        default=REPO_ROOT / "target" / "release" / "ferrite-server",
    )
    parser.add_argument("--prompt", default=DEFAULT_PROMPT)
    parser.add_argument(
        "--request-mode", choices=REQUEST_MODES, default="completion"
    )
    parser.add_argument("--max-tokens", type=int, default=64)
    parser.add_argument("--threads", type=int, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--max-load-per-core", type=float, default=0.25)
    parser.add_argument(
        "--max-background-process-cpu-percent",
        type=float,
        default=eval_suite.DEFAULT_MAX_BACKGROUND_PROCESS_CPU_PERCENT,
    )
    parser.add_argument("--clean-timeout-seconds", type=int, default=600)
    parser.add_argument("--clean-poll-seconds", type=int, default=15)
    parser.add_argument("--allow-early-stop", action="store_true")
    parser.add_argument("--numerical-policy", type=Path)
    parser.add_argument(
        "--resume-artifact",
        type=Path,
        help=(
            "resume only complete clean repetitions from a rejected schema-v2 "
            "artifact with identical source, binaries, model, host, and settings"
        ),
    )
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args(argv)


def validate_args(args):
    args.model = args.model.resolve()
    args.llama_server = args.llama_server.resolve()
    args.ferrite_server = args.ferrite_server.resolve()
    if args.numerical_policy is not None:
        args.numerical_policy = args.numerical_policy.resolve()
    if args.resume_artifact is not None:
        args.resume_artifact = args.resume_artifact.resolve()
    if not args.model.is_file():
        raise ValueError(f"model file not found: {args.model}")
    if not args.llama_server.is_file():
        raise ValueError(f"llama-server not found: {args.llama_server}")
    if args.repetitions < 3:
        raise ValueError("--repetitions must be at least 3")
    if args.numerical_policy is not None and not args.numerical_policy.is_file():
        raise ValueError(f"numerical policy file not found: {args.numerical_policy}")
    if args.resume_artifact is not None and not args.resume_artifact.is_file():
        raise ValueError(f"resume artifact file not found: {args.resume_artifact}")
    if args.dry_run and args.resume_artifact is not None:
        raise ValueError("--resume-artifact cannot be combined with --dry-run")
    if args.max_tokens < 1:
        raise ValueError("--max-tokens must be positive")
    if args.threads < 1:
        raise ValueError("--threads must be positive")
    if not args.prompt.strip():
        raise ValueError("--prompt must contain non-whitespace text")
    if args.max_load_per_core <= 0:
        raise ValueError("--max-load-per-core must be positive")
    if args.max_background_process_cpu_percent <= 0:
        raise ValueError("--max-background-process-cpu-percent must be positive")
    if args.clean_timeout_seconds < 0 or args.clean_poll_seconds < 1:
        raise ValueError("clean-host timing values are invalid")


def llama_version(binary):
    result = subprocess.run(
        [str(binary), "--version"], capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"llama-server --version failed: {result.stderr[-2000:]}"
        )
    return (result.stdout + result.stderr).strip()


def build_ferrite():
    command = [
        "cargo",
        "build",
        "--release",
        "--locked",
        "-p",
        "ferrite-server",
    ]
    print("$ " + shlex.join(command), flush=True)
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def dry_run(args, version, numerical_policy=None):
    print(f"model_sha256={ferrite_eval.sha256_file(args.model)}")
    print(f"prompt_sha256={sha256_text(args.prompt)}")
    print(f"llama_cpp_version={version}")
    if numerical_policy is not None:
        print(f"numerical_policy_id={numerical_policy['policy_id']}")
        print(f"numerical_policy_sha256={numerical_policy['sha256']}")
    for runtime in RUNTIMES:
        print("$ " + shlex.join(server_command(runtime, args, 18080)))
        rendered_prompt = None
        if runtime == "llama_cpp" and args.request_mode == "chat":
            template_request = llama_cpp_template_request(args.prompt)
            print("llama_cpp_template_endpoint=/apply-template")
            print(
                "llama_cpp_template_request="
                + json.dumps(template_request, separators=(",", ":"))
            )
            rendered_prompt = "<rendered by llama.cpp /apply-template>"
        endpoint, request = runtime_request(
            runtime,
            args.prompt,
            args.max_tokens,
            request_mode=args.request_mode,
            rendered_prompt=rendered_prompt,
        )
        print(f"{runtime}_endpoint={endpoint}")
        print(
            f"{runtime}_request={json.dumps(request, separators=(',', ':'))}"
        )


def main(argv=None):
    args = parse_args(argv)
    try:
        validate_args(args)
        version = llama_version(args.llama_server)
        if not llama_revision_matches(version, args.llama_revision):
            raise ValueError(
                "llama-server version does not match --llama-revision: " + version
            )
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error)) from error

    model_sha256 = ferrite_eval.sha256_file(args.model)
    prompt_sha256 = sha256_text(args.prompt)
    numerical_policy = None
    if args.numerical_policy is not None:
        identity = {
            "model_sha256": model_sha256,
            "prompt_sha256": prompt_sha256,
            "request_mode": args.request_mode,
            "max_tokens": args.max_tokens,
            "llama_cpp_revision": args.llama_revision,
        }
        try:
            numerical_policy = load_numerical_policy(
                args.numerical_policy, identity
            )
        except ValueError as error:
            raise SystemExit(str(error)) from error

    if args.dry_run:
        dry_run(args, version, numerical_policy=numerical_policy)
        return 0
    if not args.skip_build:
        build_ferrite()
    if not args.ferrite_server.is_file():
        raise SystemExit(f"Ferrite server not found: {args.ferrite_server}")

    started = datetime.now(timezone.utc)
    timestamp = started.strftime("%Y-%m-%d-%H%M%S")
    started_utc = started.strftime("%Y-%m-%dT%H:%M:%SZ")
    source_fingerprint = eval_suite.source_tree_sha256()
    config = comparison_config(args)
    current_host_identity = host_identity()
    resume_identity = build_resume_identity(
        args,
        version,
        source_fingerprint,
        model_sha256,
        prompt_sha256,
        numerical_policy,
        current_host_identity,
    )
    invocation_file_stamps = eval_suite.capture_file_stamps(
        [args.model, args.ferrite_server, args.llama_server]
    )
    attempts = []
    selected_runs = []
    resume_chain = []
    initial_started_utc = started_utc
    if args.resume_artifact is not None:
        try:
            previous = json.loads(args.resume_artifact.read_text(encoding="utf-8"))
            attempts, selected_runs, resume_chain = validate_resume_report(
                previous, resume_identity
            )
        except (OSError, ValueError, json.JSONDecodeError) as error:
            raise SystemExit(f"cannot resume comparison: {error}") from error
        resume_chain = [*resume_chain, str(args.resume_artifact)]
        initial_started_utc = previous.get(
            "initial_started_utc", previous.get("started_utc", started_utc)
        )

    report = {
        "schema_version": 2,
        "status": "running",
        "started_utc": started_utc,
        "initial_started_utc": initial_started_utc,
        "host": platform.platform(),
        "logical_cores": os.cpu_count(),
        "host_identity": current_host_identity,
        "source_tree_sha256": source_fingerprint,
        "model_path": str(args.model),
        "model_sha256": model_sha256,
        "prompt_sha256": prompt_sha256,
        "config": config,
        "ferrite_server": resume_identity["ferrite_server"],
        "invocation_file_stamps": invocation_file_stamps,
        "llama_cpp": {
            "server_path": str(args.llama_server),
            "server_sha256": resume_identity["llama_cpp"]["sha256"],
            "expected_revision": args.llama_revision,
            "version": version,
        },
        "resume_identity": resume_identity,
        "resume_chain": resume_chain,
        "attempts": attempts,
        "runs": selected_runs,
    }
    if args.resume_artifact is not None:
        report["resumed_from"] = str(args.resume_artifact)
    if numerical_policy is not None:
        report["numerical_policy"] = numerical_policy

    output_stem = unique_output_stem(
        f"{timestamp}-{args.model.stem.lower()}-{args.request_mode}-"
        "llama-cpp-reference"
    )
    write_report(report, output_stem)
    execution_error = None
    try:
        selected_repetitions = {run["repetition"] for run in report["runs"]}
        for repetition in range(1, args.repetitions + 1):
            if repetition in selected_repetitions:
                print(
                    f"reference comparison repetition {repetition}: "
                    "reusing complete clean pair",
                    flush=True,
                )
                continue
            order = runtime_order(repetition)
            attempt_number = 1 + sum(
                attempt.get("repetition") == repetition
                for attempt in report["attempts"]
            )
            run = {
                "repetition": repetition,
                "attempt": attempt_number,
                "order": list(order),
                "results": {},
            }
            report["attempts"].append(run)
            write_report(report, output_stem)
            for runtime in order:
                eval_suite.ensure_file_stamps_unchanged(invocation_file_stamps)
                if eval_suite.source_tree_sha256() != source_fingerprint:
                    raise RuntimeError(
                        "source tree changed during the reference comparison"
                    )
                preflight = eval_suite.wait_for_clean_host(
                    args.max_load_per_core,
                    args.clean_timeout_seconds,
                    args.clean_poll_seconds,
                    max_background_process_cpu_percent=(
                        args.max_background_process_cpu_percent
                    ),
                )
                if eval_suite.source_tree_sha256() != source_fingerprint:
                    raise RuntimeError(
                        "source tree changed during the reference comparison"
                    )
                eval_suite.ensure_file_stamps_unchanged(invocation_file_stamps)
                print(
                    f"reference comparison repetition {repetition}: {runtime}",
                    flush=True,
                )
                result = run_runtime(runtime, args)
                postflight = eval_suite.host_snapshot()
                post_reasons = postflight_rejection_reasons(
                    postflight,
                    max_background_process_cpu_percent=(
                        args.max_background_process_cpu_percent
                    ),
                )
                result["preflight"] = preflight
                result["postflight"] = postflight
                result["postflight_rejection_reasons"] = post_reasons
                run["results"][runtime] = result
                write_report(report, output_stem)
                eval_suite.ensure_file_stamps_unchanged(invocation_file_stamps)
                if eval_suite.source_tree_sha256() != source_fingerprint:
                    raise RuntimeError(
                        "source tree changed during the reference comparison"
                    )
                if post_reasons:
                    raise RuntimeError(
                        f"{runtime} repetition {repetition} postflight rejected: "
                        + "; ".join(post_reasons)
                    )
            if not run_is_clean(run, config):
                raise RuntimeError(
                    f"repetition {repetition} did not produce a complete clean pair"
                )
            report["runs"].append(run)
            selected_repetitions.add(repetition)
            write_report(report, output_stem)
    except (
        OSError,
        RuntimeError,
        subprocess.SubprocessError,
        json.JSONDecodeError,
    ) as error:
        execution_error = str(error)

    report["summary"] = summarize_runs(
        report["runs"],
        args.repetitions,
        args.max_tokens,
        allow_early_stop=args.allow_early_stop,
        numerical_policy_case=(
            numerical_policy["case"] if numerical_policy is not None else None
        ),
    )
    if execution_error is not None:
        report["status"] = "rejected"
        report["error"] = execution_error
    else:
        report["status"] = (
            "accepted"
            if report["summary"]["comparison"]["accepted"]
            else "rejected"
        )
    report["finished_utc"] = datetime.now(timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )
    json_path, markdown_path = write_report(report, output_stem)
    print(render_markdown(report))
    print(f"wrote {json_path}")
    print(f"wrote {markdown_path}")
    return 0 if report["status"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
