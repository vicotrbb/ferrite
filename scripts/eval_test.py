#!/usr/bin/env python3
"""Unit tests for the pure logic in scripts/eval.py (stdlib unittest)."""

import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
ev = importlib.import_module("eval")
suite = importlib.import_module("eval_suite")
reference = importlib.import_module("reference_compare")


class ParseKvLinesTest(unittest.TestCase):
    def test_last_occurrence_wins_and_noise_is_skipped(self):
        lines = [
            "next_token_id=42",
            "stream_text=hello world",
            "stream_text=again",
            "not a kv line",
            "benchmark_avg_ns=1000000",
        ]
        kv = ev.parse_kv_lines(lines)
        self.assertEqual(kv["next_token_id"], "42")
        self.assertEqual(kv["stream_text"], "again")
        self.assertEqual(kv["benchmark_avg_ns"], "1000000")
        self.assertEqual(len(kv), 3)

    def test_value_may_contain_equals(self):
        kv = ev.parse_kv_lines(["generated_text=a=b"])
        self.assertEqual(kv["generated_text"], "a=b")

    def test_record_terminator_is_removed_without_trimming_text(self):
        kv = ev.parse_kv_lines([
            "kernel_provider=auto\n",
            "generated_text= hello \r\n",
        ])
        self.assertEqual(kv["kernel_provider"], "auto")
        self.assertEqual(kv["generated_text"], " hello ")


class PercentileTest(unittest.TestCase):
    def test_nearest_rank(self):
        values = [40.0, 10.0, 30.0, 20.0]
        self.assertEqual(ev.percentile(values, 50), 20.0)
        self.assertEqual(ev.percentile(values, 95), 40.0)
        self.assertEqual(ev.percentile([5.0], 50), 5.0)

    def test_empty_raises(self):
        with self.assertRaises(ValueError):
            ev.percentile([], 50)


class ParseCputimeTest(unittest.TestCase):
    def test_minutes_seconds(self):
        self.assertAlmostEqual(ev.parse_cputime("0:01.25"), 1.25)
        self.assertAlmostEqual(ev.parse_cputime("2:03.50"), 123.5)

    def test_hours_and_days(self):
        self.assertAlmostEqual(ev.parse_cputime("1:02:03"), 3723.0)
        self.assertAlmostEqual(ev.parse_cputime("1-01:02:03"), 86400 + 3723.0)


class AggregateSamplesTest(unittest.TestCase):
    SAMPLES = [
        ev.Sample(t=0.0, rss_bytes=100 << 20, cpu_seconds=0.0),
        ev.Sample(t=1.0, rss_bytes=200 << 20, cpu_seconds=0.5),
        ev.Sample(t=2.0, rss_bytes=150 << 20, cpu_seconds=1.5),
    ]

    def test_whole_run_aggregation(self):
        agg = ev.aggregate_samples(self.SAMPLES)
        self.assertEqual(agg["sample_count"], 3)
        self.assertEqual(agg["rss_peak_bytes"], 200 << 20)
        self.assertEqual(agg["rss_mean_bytes"], (450 << 20) // 3)
        self.assertAlmostEqual(agg["cpu_mean_percent"], 75.0)
        self.assertAlmostEqual(agg["cpu_peak_percent"], 100.0)

    def test_windowed_aggregation(self):
        agg = ev.aggregate_samples(self.SAMPLES, t_start=1.0, t_end=2.0)
        self.assertEqual(agg["sample_count"], 2)
        self.assertAlmostEqual(agg["cpu_mean_percent"], 100.0)

    def test_empty_window(self):
        agg = ev.aggregate_samples(self.SAMPLES, t_start=5.0)
        self.assertEqual(agg, {"sample_count": 0})

    def test_cpu_counter_regression_omits_cpu_metrics(self):
        samples = [
            ev.Sample(t=0.0, rss_bytes=100 << 20, cpu_seconds=1.0),
            ev.Sample(t=1.0, rss_bytes=200 << 20, cpu_seconds=2.0),
            ev.Sample(t=2.0, rss_bytes=150 << 20, cpu_seconds=1.5),
        ]
        agg = ev.aggregate_samples(samples)
        self.assertEqual(agg["cpu_metrics_status"], "cumulative_counter_regressed")
        self.assertNotIn("cpu_mean_percent", agg)
        self.assertNotIn("cpu_peak_percent", agg)
        self.assertEqual(agg["rss_peak_bytes"], 200 << 20)

    def test_non_monotonic_sample_time_omits_cpu_metrics(self):
        samples = [
            ev.Sample(t=1.0, rss_bytes=100 << 20, cpu_seconds=1.0),
            ev.Sample(t=1.0, rss_bytes=200 << 20, cpu_seconds=2.0),
        ]
        agg = ev.aggregate_samples(samples)
        self.assertEqual(agg["cpu_metrics_status"], "non_monotonic_sample_time")
        self.assertNotIn("cpu_mean_percent", agg)
        self.assertNotIn("cpu_peak_percent", agg)


class ComputeCliMetricsTest(unittest.TestCase):
    def test_synthetic_generation_timeline(self):
        lines = [
            (1.0, "sleep_after_load_ms=2000"),
            (3.5, "prompt_token_ids=1,2,3"),
            (3.55, "next_token_id=7"),
            (3.6, "stream_token_id=7"),
            (3.6, "stream_text=a"),
            (3.7, "stream_token_id=8"),
            (3.8, "stream_token_id=9"),
            (3.9, "stream_token_id=10"),
        ]
        metrics = ev.compute_cli_metrics(t_spawn=0.0, lines=lines, sleep_ms=2000)
        self.assertAlmostEqual(metrics["load_seconds"], 1.0)
        self.assertAlmostEqual(metrics["ttft_prefill_seconds"], 0.5)
        self.assertEqual(metrics["stream_token_count"], 4)
        self.assertAlmostEqual(metrics["decode_tokens_per_second_streamed"], 10.0)
        self.assertAlmostEqual(metrics["token_latency_ms_p50"], 100.0)
        self.assertAlmostEqual(metrics["t_sleep"], 1.0)
        self.assertAlmostEqual(metrics["t_gen_start"], 3.0)
        self.assertAlmostEqual(metrics["t_last_stream"], 3.9)

    def test_missing_sleep_marker_returns_empty(self):
        self.assertEqual(
            ev.compute_cli_metrics(0.0, [(1.0, "next_token_id=5")], 2000), {}
        )


class OutputStemTest(unittest.TestCase):
    def test_single_model(self):
        stem = ev.output_stem(
            "2026-07-09-120000", [Path("target/models/Qwen2.5 0.5B.gguf")]
        )
        self.assertEqual(stem, "2026-07-09-120000-qwen2.5-0.5b")

    def test_multi_model(self):
        stem = ev.output_stem(
            "2026-07-09-120000",
            [Path("a/first.gguf"), Path("b/second.gguf")],
        )
        self.assertEqual(stem, "2026-07-09-120000-first-multi")

    def test_unique_stem_and_atomic_write_preserve_existing_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            output_dir = Path(directory)
            existing = output_dir / "evaluation.json"
            existing.write_text("old", encoding="utf-8")

            stem = ev.unique_output_stem("evaluation", output_dir)
            replacement = output_dir / f"{stem}.json"
            ev.atomic_write_text(replacement, "new\n")

            self.assertEqual(stem, "evaluation-2")
            self.assertEqual(existing.read_text(encoding="utf-8"), "old")
            self.assertEqual(replacement.read_text(encoding="utf-8"), "new\n")


class ReleaseBinaryTest(unittest.TestCase):
    def test_skip_build_fails_when_release_binaries_are_missing(self):
        with tempfile.TemporaryDirectory() as root:
            with mock.patch.object(ev, "REPO_ROOT", Path(root)):
                with self.assertRaisesRegex(
                    SystemExit, "--skip-build requires existing release binaries"
                ):
                    ev.existing_release_binaries()


class Sha256FileTest(unittest.TestCase):
    def test_hashes_file_bytes(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "model.gguf"
            path.write_bytes(b"Ferrite")
            self.assertEqual(
                ev.sha256_file(path),
                "deb633a52c55fc7d804f5aabd4bab825a9ad00b616ae77099a42b9d668d24544",
            )

    def test_default_model_verification_checks_size_and_hash(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / ev.DEFAULT_MODEL_FILENAME
            path.write_bytes(b"abc")
            with (
                mock.patch.object(ev, "DEFAULT_MODEL_SIZE", 3),
                mock.patch.object(
                    ev,
                    "DEFAULT_MODEL_SHA256",
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                ),
            ):
                ev.verify_default_model(path)
                path.write_bytes(b"abd")
                with self.assertRaisesRegex(RuntimeError, "SHA-256 mismatch"):
                    ev.verify_default_model(path)

    def test_default_model_provenance_is_hash_scoped_and_revision_pinned(self):
        provenance = ev.default_model_provenance(ev.DEFAULT_MODEL_SHA256)

        self.assertIsNotNone(provenance)
        self.assertEqual(provenance["source"], ev.DEFAULT_MODEL_SOURCE)
        self.assertEqual(provenance["revision"], ev.DEFAULT_MODEL_REVISION)
        self.assertEqual(provenance["license"], ev.DEFAULT_MODEL_LICENSE)
        self.assertIn(ev.DEFAULT_MODEL_REVISION, provenance["license_url"])
        self.assertIn(ev.DEFAULT_MODEL_REVISION, provenance["url"])
        self.assertIsNone(ev.default_model_provenance("00" * 32))

    def test_default_model_download_verifies_before_atomic_publication(self):
        expected_hash = (
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )

        def retrieve(url, path, reporthook):
            self.assertEqual(url, "https://example.invalid/model.gguf")
            Path(path).write_bytes(b"abc")
            reporthook(1, 3, 3)

        with tempfile.TemporaryDirectory() as root:
            models_dir = Path(root)
            with (
                mock.patch.object(ev, "MODELS_DIR", models_dir),
                mock.patch.object(ev, "DEFAULT_MODEL_SIZE", 3),
                mock.patch.object(ev, "DEFAULT_MODEL_SHA256", expected_hash),
                mock.patch.object(
                    ev, "DEFAULT_MODEL_URL", "https://example.invalid/model.gguf"
                ),
                mock.patch.object(ev.urllib.request, "urlretrieve", retrieve),
                mock.patch("builtins.print"),
            ):
                target = ev.download_default_model()

            self.assertEqual(target.read_bytes(), b"abc")
            self.assertFalse(target.with_suffix(".gguf.part").exists())

    def test_default_model_download_removes_mismatched_partial(self):
        expected_hash = (
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )

        def retrieve(_url, path, reporthook):
            del reporthook
            Path(path).write_bytes(b"abd")

        with tempfile.TemporaryDirectory() as root:
            models_dir = Path(root)
            target = models_dir / ev.DEFAULT_MODEL_FILENAME
            with (
                mock.patch.object(ev, "MODELS_DIR", models_dir),
                mock.patch.object(ev, "DEFAULT_MODEL_SIZE", 3),
                mock.patch.object(ev, "DEFAULT_MODEL_SHA256", expected_hash),
                mock.patch.object(ev.urllib.request, "urlretrieve", retrieve),
                mock.patch("builtins.print"),
            ):
                with self.assertRaisesRegex(RuntimeError, "SHA-256 mismatch"):
                    ev.download_default_model()

            self.assertFalse(target.exists())
            self.assertFalse(target.with_suffix(".gguf.part").exists())


class ServerWorkloadTest(unittest.TestCase):
    def test_identical_workload_uses_one_prompt_for_all_requests(self):
        prompts = ev.server_workload_prompts("identical", "base", 4)
        self.assertEqual(prompts, ("base",))

    def test_shared_prefix_workload_has_unique_suffixes(self):
        prompts = ev.server_workload_prompts("shared-prefix", "common base", 4)
        self.assertEqual(len(prompts), 4)
        self.assertEqual(len(set(prompts)), 4)
        self.assertTrue(all(prompt.startswith("common base\n\n") for prompt in prompts))

    def test_distinct_workload_has_one_unique_prompt_per_request(self):
        prompts = ev.server_workload_prompts("distinct", "unused", 12)
        self.assertEqual(len(prompts), 12)
        self.assertEqual(len(set(prompts)), 12)
        self.assertNotIn("unused", prompts)

    def test_unknown_workload_is_rejected(self):
        with self.assertRaises(ValueError):
            ev.server_workload_prompts("unknown", "base", 4)

    def test_mixed_length_workload_pairs_distinct_prompts_with_token_budgets(self):
        budgets = ev.server_workload_token_budgets("mixed-length", 64)
        prompts = ev.server_workload_prompts("mixed-length", "unused", len(budgets))

        self.assertEqual(budgets, (1, 16, 32, 64))
        self.assertEqual(len(prompts), len(budgets))
        self.assertEqual(len(set(prompts)), len(prompts))

    def test_prefix_cache_warmup_uses_one_prompt_and_one_budget(self):
        prompts = ev.server_workload_prompts("shared-prefix", "base", 4)
        budgets = ev.server_workload_token_budgets("shared-prefix", 64)

        measured = ev.server_client_command(
            Path("throughput"),
            8080,
            4,
            4,
            prompts,
            budgets,
            "shared-prefix",
            True,
        )
        warmup = ev.server_client_command(
            Path("throughput"),
            8080,
            1,
            1,
            prompts[:1],
            budgets[:1],
            "shared-prefix",
            True,
        )

        self.assertEqual(measured.count("--prompt"), 4)
        self.assertEqual(warmup.count("--prompt"), 1)
        self.assertEqual(warmup.count("--max-tokens"), 1)
        self.assertEqual(warmup[warmup.index("--requests") + 1], "1")
        self.assertEqual(warmup[warmup.index("--concurrency") + 1], "1")

    def test_per_prompt_traces_must_be_stable_and_match(self):
        default = {
            "streaming_prompt_token_id_traces": [[1, 2], [3]],
            "streaming_all_prompt_token_id_traces_stable": "true",
        }
        batched = {
            "streaming_prompt_token_id_traces": [[1, 2], [3]],
            "streaming_all_prompt_token_id_traces_stable": "true",
        }
        self.assertTrue(ev.server_token_traces_match(default, batched))

        batched["streaming_prompt_token_id_traces"] = [[1, 2], [5]]
        self.assertFalse(ev.server_token_traces_match(default, batched))
        batched["streaming_prompt_token_id_traces"] = [[1, 2], [3]]
        batched["streaming_all_prompt_token_id_traces_stable"] = "false"
        self.assertFalse(ev.server_token_traces_match(default, batched))

    def test_soak_rss_summary_accepts_bounded_tail_and_rejects_growth(self):
        accepted = ev.summarize_soak_rss([100, 104, 103, 105], tolerance_bytes=8)
        rejected = ev.summarize_soak_rss([100, 104, 120, 140], tolerance_bytes=8)

        self.assertTrue(accepted["soak_rss_stable"])
        self.assertFalse(rejected["soak_rss_stable"])
        self.assertEqual(rejected["soak_rss_growth_bytes"], 40)

    def test_macos_physical_footprint_parser_and_summary(self):
        output = """\
Auxiliary data:
    phys_footprint: 27886744 B
    phys_footprint_peak: 473597392 B
"""
        self.assertEqual(ev.parse_macos_phys_footprint_bytes(output), 27886744)
        accepted = ev.summarize_soak_physical_footprint(
            [100, 104, 103, 105], tolerance_bytes=8
        )
        rejected = ev.summarize_soak_physical_footprint(
            [100, 104, 120, 140], tolerance_bytes=8
        )
        self.assertTrue(accepted["soak_physical_footprint_stable"])
        self.assertFalse(rejected["soak_physical_footprint_stable"])
        with self.assertRaises(ValueError):
            ev.parse_macos_phys_footprint_bytes("phys_footprint: unknown")

    def test_server_soak_warms_up_before_collecting_idle_samples(self):
        cfg = mock.Mock(
            server_soak_rounds=3,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_bytes=8,
            requests=2,
        )
        client = mock.Mock(
            returncode=0,
            stderr="",
            stdout=(
                "streaming_prompt_token_id_traces=[[1,2]]\n"
                "streaming_all_prompt_token_id_traces_stable=true\n"
            ),
        )

        with (
            mock.patch.object(ev.subprocess, "run", return_value=client) as run,
            mock.patch.object(ev.platform, "system", return_value="Linux"),
            mock.patch.object(ev, "process_rss_bytes", side_effect=[100, 104, 105]),
            mock.patch.object(ev.time, "sleep") as sleep,
        ):
            result = ev.run_server_soak(
                ["throughput-client"], 123, cfg, expected_trace="[[1,2]]"
            )

        self.assertEqual(run.call_count, 4)
        self.assertEqual(sleep.call_count, 4)
        self.assertEqual(result["soak_warmup_requests"], 2)
        self.assertEqual(result["soak_requests"], 6)
        self.assertEqual(result["soak_rss_idle_bytes"], [100, 104, 105])
        self.assertTrue(result["soak_all_token_traces_match"])
        self.assertTrue(result["soak_memory_stable"])

    def test_server_trace_identity_rejects_empty_traces(self):
        self.assertEqual(
            ev.server_trace_identity(
                {"streaming_prompt_token_id_traces": "[[1,2],[3]]"}
            ),
            "[[1,2],[3]]",
        )
        with self.assertRaises(RuntimeError):
            ev.server_trace_identity({"streaming_prompt_token_id_traces": "[[]]"})


class AcceptanceSuiteTest(unittest.TestCase):
    @staticmethod
    def clean_snapshot():
        return {
            "load_per_core_1m": 0.1,
            "ferrite_processes": [],
            "process_observation_available": True,
            "thermal_status": None,
            "max_background_process_cpu_percent": 10.0,
            "top_background_cpu_processes": [],
        }

    @staticmethod
    def resume_case():
        config = suite.SuiteConfig(
            models=(Path("/models/model.gguf"),),
            prompt="prompt",
            generate_tokens=8,
            benchmark_runs=8,
            batch_streams=(4,),
            server_batch_streams=4,
            requests=4,
            workloads=("identical",),
            repetitions=3,
            tag_prefix="resume",
            server_soak_rounds=0,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_mib=8,
            server_prefix_cache=False,
            kernel_provider="auto",
        )
        return suite.build_cases(config)[0]

    @classmethod
    def clean_suite_attempt(cls, root, case, model_sha256):
        evals = root / "scripts" / "evals"
        evals.mkdir(parents=True)
        artifact = evals / "case.json"
        expected = suite.expected_eval_report_identity(case)
        artifact.write_text(
            json.dumps(
                {
                    "schema_version": 4,
                    "tag": expected["tag"],
                    "config": expected["config"],
                    "env": {"binary_build_mode": "prebuilt"},
                    "models": [
                        {
                            "model_path": expected["model_paths"][0],
                            "model_sha256": model_sha256,
                            "cli": {
                                "status": "ok",
                                "benchmark_token_ids": [1],
                                "decode_tokens_per_second_precise": 1.0,
                                "ttft_prefill_seconds": 1.0,
                                "batch_benchmarks": [
                                    {
                                        "streams": 4,
                                        "status": "ok",
                                        "stream_0_matches_single": True,
                                    }
                                ],
                            },
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )
        markdown = artifact.with_suffix(".md")
        markdown.write_text("# case\n", encoding="utf-8")
        return {
            **suite.case_identity(case, 0),
            "attempt": 1,
            "status": "clean",
            "preflight": cls.clean_snapshot(),
            "artifact": "scripts/evals/case.json",
            "artifact_sha256": suite.sha256_file(artifact),
            "artifact_markdown": "scripts/evals/case.md",
            "artifact_markdown_sha256": suite.sha256_file(markdown),
            "postflight": cls.clean_snapshot(),
            "postflight_rejection_reasons": [],
            "case_rejection_reasons": [],
        }

    def test_preflight_only_does_not_require_a_model(self):
        args = suite.parse_args(["--preflight-only"])

        self.assertTrue(args.preflight_only)
        self.assertIsNone(args.model)

    def test_preflight_report_retains_policy_snapshot_and_reasons(self):
        snapshot = {
            "load_per_core_1m": 0.5,
            "ferrite_processes": [],
            "process_observation_available": True,
            "thermal_status": None,
            "max_background_process_cpu_percent": 10.0,
        }

        report = suite.preflight_report(snapshot, 0.25, 50.0)

        self.assertEqual(report["status"], "rejected")
        self.assertEqual(report["snapshot"], snapshot)
        self.assertEqual(report["clean_host_policy"]["max_load_per_core"], 0.25)
        self.assertEqual(len(report["rejection_reasons"]), 1)

    def test_case_report_rejects_runtime_thread_count_drift(self):
        config = suite.SuiteConfig(
            models=(Path("/models/model.gguf"),),
            prompt="prompt",
            generate_tokens=8,
            benchmark_runs=8,
            batch_streams=(4,),
            server_batch_streams=4,
            requests=4,
            workloads=("identical",),
            repetitions=3,
            tag_prefix="threads",
            server_soak_rounds=0,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_mib=8,
            threads=2,
        )
        case = suite.build_cases(config)[0]
        expected = suite.expected_eval_report_identity(case)
        report = {
            "schema_version": ev.SCHEMA_VERSION,
            "tag": expected["tag"],
            "config": expected["config"],
            "env": {"binary_build_mode": "prebuilt"},
            "models": [
                {
                    "model_path": expected["model_paths"][0],
                    "model_sha256": "ab" * 32,
                    "cli": {"inference_threads": 3},
                }
            ],
        }

        with self.assertRaisesRegex(RuntimeError, "runtime thread count"):
            suite.validate_case_report(report, case, ["ab" * 32])

    def test_build_cases_balances_order_across_repetitions(self):
        config = suite.SuiteConfig(
            models=(Path("model.gguf"),),
            prompt="prompt",
            generate_tokens=8,
            benchmark_runs=8,
            batch_streams=(4, 8),
            server_batch_streams=4,
            requests=4,
            workloads=("identical", "shared-prefix", "distinct", "mixed-length"),
            repetitions=3,
            tag_prefix="gate",
            server_soak_rounds=3,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_mib=8,
            server_prefix_cache=True,
        )

        cases = suite.build_cases(config)

        self.assertEqual(len(cases), 15)
        self.assertEqual(
            [case["label"] for case in cases[:5]],
            [
                "cli",
                "server-identical",
                "server-shared-prefix",
                "server-distinct",
                "server-mixed-length",
            ],
        )
        self.assertEqual(cases[5]["label"], "server-identical")
        self.assertEqual(cases[10]["label"], "server-shared-prefix")
        self.assertIn("--server-workload", cases[6]["command"])
        self.assertIn("--server-soak-rounds", cases[1]["command"])
        self.assertNotIn("--server-soak-rounds", cases[2]["command"])
        self.assertNotIn("--server-prefix-cache", cases[0]["command"])
        self.assertIn("--server-prefix-cache", cases[1]["command"])
        self.assertTrue(
            all("--skip-build" in case["command"] for case in cases),
            "the suite must reuse the release binaries it built once",
        )

    def test_build_cases_propagates_portable_kernel_provider(self):
        config = suite.SuiteConfig(
            models=(Path("model.gguf"),),
            prompt="prompt",
            generate_tokens=8,
            benchmark_runs=8,
            batch_streams=(4,),
            server_batch_streams=4,
            requests=4,
            workloads=("identical",),
            repetitions=3,
            tag_prefix="portable",
            server_soak_rounds=0,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_mib=8,
            server_prefix_cache=False,
            kernel_provider="portable",
            threads=2,
        )

        for case in suite.build_cases(config):
            self.assertIn("--kernel-provider", case["command"])
            self.assertIn("portable", case["command"])
            self.assertIn("--threads", case["command"])
            thread_index = case["command"].index("--threads")
            self.assertEqual(case["command"][thread_index + 1], "2")
            identity = suite.expected_eval_report_identity(case)
            self.assertEqual(identity["config"]["threads"], 2)

    def test_build_cases_propagates_bounded_locus_only_to_server_cases(self):
        config = suite.SuiteConfig(
            models=(Path("model.gguf"),),
            prompt="prompt",
            generate_tokens=8,
            benchmark_runs=8,
            batch_streams=(4,),
            server_batch_streams=4,
            requests=4,
            workloads=("identical",),
            repetitions=3,
            tag_prefix="locus",
            server_soak_rounds=3,
            server_soak_idle_ms=250,
            server_soak_rss_tolerance_mib=8,
            server_prefix_cache=False,
            kernel_provider="auto",
            server_kv_backend="locus",
            server_kv_tokens_per_block=16,
            server_kv_max_tokens=128,
        )

        cases = suite.build_cases(config)
        cli = next(case for case in cases if case["kind"] == "cli")
        server = next(case for case in cases if case["kind"] == "server")
        self.assertNotIn("--server-kv-backend", cli["command"])
        self.assertIn("--server-kv-backend", server["command"])
        self.assertIn("locus", server["command"])
        self.assertIn("--server-kv-tokens-per-block", server["command"])
        self.assertIn("--server-kv-max-tokens", server["command"])
        identity = suite.expected_eval_report_identity(server)
        self.assertEqual(identity["config"]["server_kv_backend"], "locus")
        self.assertEqual(identity["config"]["server_kv_max_tokens"], 128)

    def test_suite_locus_configuration_requires_a_positive_capacity(self):
        with tempfile.TemporaryDirectory() as directory:
            model = Path(directory) / "model.gguf"
            model.write_bytes(b"model")
            missing = suite.parse_args(
                ["--model", str(model), "--server-kv-backend", "locus"]
            )
            with self.assertRaisesRegex(ValueError, "requires --server-kv-max-tokens"):
                suite.validated_config(missing)

            accepted = suite.parse_args(
                [
                    "--model",
                    str(model),
                    "--server-kv-backend",
                    "locus",
                    "--server-kv-max-tokens",
                    "128",
                ]
            )
            config = suite.validated_config(accepted)
            self.assertEqual(config.server_kv_tokens_per_block, 16)
            self.assertEqual(config.server_kv_max_tokens, 128)

    def test_host_rejection_reasons_cover_load_processes_and_thermal_pressure(self):
        snapshot = {
            "load_per_core_1m": 0.5,
            "ferrite_processes": [{"pid": 7, "command": "ferrite-server"}],
            "thermal_status": ["CPU_Speed_Limit = 80"],
        }

        reasons = suite.host_rejection_reasons(snapshot, max_load_per_core=0.25)

        self.assertEqual(len(reasons), 3)

    def test_host_rejection_reasons_cover_busy_background_process(self):
        snapshot = {
            "load_per_core_1m": 0.1,
            "ferrite_processes": [],
            "thermal_status": None,
            "max_background_process_cpu_percent": 120.0,
            "top_background_cpu_processes": [
                {"pid": 42, "cpu_percent": 120.0, "command": "vm-helper"}
            ],
        }

        reasons = suite.host_rejection_reasons(
            snapshot,
            max_load_per_core=0.25,
            max_background_process_cpu_percent=50.0,
        )

        self.assertEqual(len(reasons), 1)
        self.assertIn("vm-helper pid 42", reasons[0])

    def test_postflight_rejection_ignores_load_but_keeps_other_gates(self):
        snapshot = {
            "load_per_core_1m": 1.0,
            "ferrite_processes": [{"pid": 7, "command": "ferrite-server"}],
            "thermal_status": None,
            "max_background_process_cpu_percent": 120.0,
            "top_background_cpu_processes": [
                {"pid": 42, "cpu_percent": 120.0, "command": "vm-helper"}
            ],
        }

        reasons = suite.host_rejection_reasons(
            snapshot,
            max_load_per_core=0.25,
            max_background_process_cpu_percent=50.0,
            check_load=False,
        )

        self.assertEqual(len(reasons), 2)
        self.assertTrue(all("load per core" not in reason for reason in reasons))

    def test_host_rejection_fails_closed_without_process_observation(self):
        snapshot = {
            "load_per_core_1m": None,
            "ferrite_processes": None,
            "process_observation_available": False,
            "thermal_status": None,
            "max_background_process_cpu_percent": None,
        }

        reasons = suite.host_rejection_reasons(
            snapshot, max_load_per_core=0.25
        )

        self.assertEqual(reasons, ["host process observation is unavailable"])

    def test_clean_host_wait_fails_immediately_without_process_observation(self):
        snapshot = {
            "load_per_core_1m": None,
            "ferrite_processes": None,
            "process_observation_available": False,
            "thermal_status": None,
            "max_background_process_cpu_percent": None,
        }

        with mock.patch.object(suite, "host_snapshot", return_value=snapshot):
            with self.assertRaisesRegex(
                RuntimeError, "host process observation is unavailable"
            ):
                suite.wait_for_clean_host(0.25, 600, 15)

    def test_clean_host_wait_throttles_logs_without_throttling_samples(self):
        rejected = self.clean_snapshot()
        rejected["load_per_core_1m"] = 0.5
        clean = self.clean_snapshot()

        with (
            mock.patch.object(
                suite,
                "host_snapshot",
                side_effect=[rejected] * 5 + [clean],
            ) as snapshots,
            mock.patch.object(
                suite.time,
                "monotonic",
                side_effect=[0.0, 0.0, 15.0, 30.0, 45.0, 60.0],
            ),
            mock.patch.object(suite.time, "sleep") as sleep,
            mock.patch("builtins.print") as printed,
        ):
            observed = suite.wait_for_clean_host(0.25, 600, 15)

        self.assertEqual(observed, clean)
        self.assertEqual(snapshots.call_count, 6)
        self.assertEqual(sleep.call_count, 5)
        self.assertEqual(printed.call_count, 2)

    def test_summarizes_three_exact_server_repetitions(self):
        run_reports = []
        for repetition, throughput in enumerate((100.0, 120.0, 140.0), start=1):
            case = {
                "kind": "server",
                "label": "server-distinct",
                "repetition": repetition,
            }
            report = {
                "models": [
                    {
                        "model_sha256": "ab" * 32,
                        "server": {
                            "status": "ok",
                            "streaming_all_prompt_token_id_traces_stable": "true",
                        },
                        "batched_server": {
                            "status": "ok",
                            "streaming_all_prompt_token_id_traces_stable": "true",
                            "token_ids_match_default": True,
                            "streaming_prompt_token_id_traces": [[1, 2], [3]],
                            "aggregate_completion_tokens_per_second": throughput,
                            "streaming_time_to_first_token_p50_ms": "100",
                            "streaming_time_to_first_token_p95_ms": "150",
                            "server_rss_peak_bytes": 1024,
                        },
                    }
                ]
            }
            run_reports.append((case, report))

        summaries = suite.summarize_reports(run_reports)

        self.assertEqual(len(summaries), 1)
        self.assertTrue(summaries[0]["accepted"])
        self.assertEqual(
            summaries[0]["metrics"]["aggregate_completion_tokens_per_second"][
                "median"
            ],
            120.0,
        )

    def test_server_summary_uses_explicit_memory_gate_and_retains_rss(self):
        case = {"kind": "server", "label": "server-identical", "repetition": 1}
        model = {
            "server": {
                "status": "ok",
                "streaming_all_prompt_token_id_traces_stable": "true",
            },
            "batched_server": {
                "status": "ok",
                "streaming_all_prompt_token_id_traces_stable": "true",
                "token_ids_match_default": True,
                "streaming_prompt_token_id_traces": [[1, 2]],
                "aggregate_completion_tokens_per_second": 12.0,
                "streaming_time_to_first_token_p50_ms": "100",
                "streaming_time_to_first_token_p95_ms": "150",
                "soak_rounds": 3,
                "soak_all_token_traces_match": True,
                "soak_rss_stable": False,
                "soak_rss_growth_bytes": 400 << 20,
                "soak_rss_tail_range_bytes": 2 << 20,
                "soak_memory_stability_metric": "macos_phys_footprint",
                "soak_memory_stable": True,
                "soak_memory_growth_bytes": 1 << 20,
                "soak_memory_tail_range_bytes": 2 << 20,
            },
        }

        record = suite.case_model_record(case, model)

        self.assertTrue(record["parity"])
        self.assertTrue(record["required_metrics_present"])
        self.assertEqual(record["metrics"]["soak_rss_growth_bytes"], 400 << 20)
        self.assertEqual(record["metrics"]["soak_memory_growth_bytes"], 1 << 20)
        model["batched_server"]["soak_memory_stable"] = False
        self.assertFalse(suite.case_model_record(case, model)["parity"])

    def test_resume_retains_only_hashed_complete_clean_cases(self):
        case = self.resume_case()
        model_sha256 = "ab" * 32
        policy = {
            "max_load_per_core": 0.25,
            "max_background_process_cpu_percent": 50.0,
            "clean_timeout_seconds": 600,
            "clean_poll_seconds": 15,
        }
        identity = {"clean_host_policy": policy, "source_tree_sha256": "a" * 64}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            clean = self.clean_suite_attempt(root, case, model_sha256)
            previous = {
                "schema_version": 2,
                "status": "rejected",
                "started_utc": "2026-07-14T00:00:00Z",
                "resume_identity": identity,
                "resume_chain": ["first.json"],
                "attempts": [clean],
                "runs": [clean],
            }
            with (
                mock.patch.object(suite, "REPO_ROOT", root),
                mock.patch.object(suite, "EVALS_DIR", root / "scripts" / "evals"),
            ):
                attempts, runs, chain, reports = suite.validate_resume_manifest(
                    previous, identity, [case], [model_sha256]
                )

        self.assertEqual(attempts, [clean])
        self.assertEqual(runs, [clean])
        self.assertEqual(chain, ["first.json"])
        self.assertEqual(reports[0][1]["tag"], "resume-cli-r1")

    def test_resume_rejects_changed_raw_case_artifact(self):
        case = self.resume_case()
        model_sha256 = "ab" * 32
        policy = {
            "max_load_per_core": 0.25,
            "max_background_process_cpu_percent": 50.0,
        }
        identity = {"clean_host_policy": policy}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            clean = self.clean_suite_attempt(root, case, model_sha256)
            (root / clean["artifact"]).write_text("{}", encoding="utf-8")
            previous = {
                "schema_version": 2,
                "status": "rejected",
                "started_utc": "2026-07-14T00:00:00Z",
                "resume_identity": identity,
                "resume_chain": [],
                "attempts": [clean],
                "runs": [clean],
            }
            with (
                mock.patch.object(suite, "REPO_ROOT", root),
                mock.patch.object(suite, "EVALS_DIR", root / "scripts" / "evals"),
                self.assertRaisesRegex(ValueError, "artifact hash does not match"),
            ):
                suite.validate_resume_manifest(
                    previous, identity, [case], [model_sha256]
                )

    def test_resume_rejects_contaminated_selected_case(self):
        case = self.resume_case()
        model_sha256 = "ab" * 32
        policy = {
            "max_load_per_core": 0.25,
            "max_background_process_cpu_percent": 50.0,
        }
        identity = {"clean_host_policy": policy}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            run = self.clean_suite_attempt(root, case, model_sha256)
            run["postflight"]["max_background_process_cpu_percent"] = 75.0
            run["postflight"]["top_background_cpu_processes"] = [
                {"pid": 42, "cpu_percent": 75.0, "command": "vm-helper"}
            ]
            run["postflight_rejection_reasons"] = [
                "background process CPU 75.0% exceeds 50.0% (vm-helper pid 42)"
            ]
            previous = {
                "schema_version": 2,
                "status": "rejected",
                "started_utc": "2026-07-14T00:00:00Z",
                "resume_identity": identity,
                "resume_chain": [],
                "attempts": [run],
                "runs": [run],
            }
            with (
                mock.patch.object(suite, "REPO_ROOT", root),
                mock.patch.object(suite, "EVALS_DIR", root / "scripts" / "evals"),
                self.assertRaisesRegex(ValueError, "clean case is incomplete"),
            ):
                suite.validate_resume_manifest(
                    previous, identity, [case], [model_sha256]
                )

    def test_main_resume_reuses_clean_case_and_reruns_only_missing_case(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            evals = root / "scripts" / "evals"
            evals.mkdir(parents=True)
            model = root / "model.gguf"
            model.write_bytes(b"model")
            model_sha256 = suite.sha256_file(model)
            binaries = {
                "ferrite": root / "target" / "release" / "ferrite",
                "server": root / "target" / "release" / "ferrite-server",
                "throughput": (
                    root / "target" / "release" / "ferrite-openai-throughput"
                ),
            }
            for name, path in binaries.items():
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(name.encode())

            original_build_cases = suite.build_cases
            cases_by_command = {}

            def two_cases(config):
                cases = original_build_cases(config)[:2]
                cases_by_command.clear()
                cases_by_command.update(
                    {tuple(case["command"]): case for case in cases}
                )
                return cases

            invoked_tags = []
            lifecycle_events = []

            def fake_child(command, **_kwargs):
                case = cases_by_command[tuple(command)]
                expected = suite.expected_eval_report_identity(case)
                invoked_tags.append(expected["tag"])
                lifecycle_events.append("child")
                stem = f"raw-{len(invoked_tags)}"
                artifact = evals / f"{stem}.json"
                model_entry = {
                    "model_path": expected["model_paths"][0],
                    "model_sha256": model_sha256,
                }
                if case["kind"] == "cli":
                    model_entry["cli"] = {
                        "status": "ok",
                        "benchmark_token_ids": [1],
                        "decode_tokens_per_second_precise": 1.0,
                        "ttft_prefill_seconds": 1.0,
                        "batch_benchmarks": [
                            {
                                "streams": streams,
                                "status": "ok",
                                "stream_0_matches_single": True,
                            }
                            for streams in (4, 8)
                        ],
                    }
                else:
                    model_entry["server"] = {
                        "status": "ok",
                        "streaming_all_prompt_token_id_traces_stable": "true",
                    }
                    model_entry["batched_server"] = {
                        "status": "ok",
                        "streaming_all_prompt_token_id_traces_stable": "true",
                        "token_ids_match_default": True,
                        "streaming_prompt_token_id_traces": [[1]],
                        "aggregate_completion_tokens_per_second": 1.0,
                        "streaming_time_to_first_token_p50_ms": "1",
                        "streaming_time_to_first_token_p95_ms": "1",
                    }
                report = {
                    "schema_version": 4,
                    "tag": expected["tag"],
                    "config": expected["config"],
                    "env": {"binary_build_mode": "prebuilt"},
                    "models": [model_entry],
                }
                artifact.write_text(json.dumps(report), encoding="utf-8")
                artifact.with_suffix(".md").write_text(
                    "# raw\n", encoding="utf-8"
                )
                return mock.Mock(
                    returncode=0,
                    stdout=(
                        f"wrote {artifact}\n"
                        f"wrote {artifact.with_suffix('.md')}\n"
                    ),
                )

            clean = self.clean_snapshot()
            contaminated = self.clean_snapshot()
            contaminated["max_background_process_cpu_percent"] = 75.0
            contaminated["top_background_cpu_processes"] = [
                {"pid": 42, "cpu_percent": 75.0, "command": "vm-helper"}
            ]
            snapshots = iter([clean, contaminated, clean])

            def fake_host_snapshot():
                lifecycle_events.append("postflight")
                return next(snapshots)

            def fake_print(*args, **_kwargs):
                if (
                    args
                    and isinstance(args[0], str)
                    and args[0].startswith("wrote ")
                    and "\nwrote " in args[0]
                ):
                    lifecycle_events.append("report")

            argv = [
                "--model",
                str(model),
                "--workload",
                "identical",
                "--repetitions",
                "3",
                "--skip-build",
            ]
            with (
                mock.patch.object(suite, "REPO_ROOT", root),
                mock.patch.object(suite, "EVALS_DIR", evals),
                mock.patch.object(suite, "EVAL_SCRIPT", root / "scripts" / "eval.py"),
                mock.patch.object(suite, "build_cases", side_effect=two_cases),
                mock.patch.object(
                    suite.ferrite_eval,
                    "release_binaries",
                    return_value=binaries,
                ),
                mock.patch.object(suite, "source_tree_sha256", return_value="a" * 64),
                mock.patch.object(suite, "host_identity", return_value={"host": "test"}),
                mock.patch.object(suite.platform, "platform", return_value="test-platform"),
                mock.patch.object(
                    suite,
                    "wait_for_clean_host",
                    return_value=self.clean_snapshot(),
                ),
                mock.patch.object(
                    suite,
                    "host_snapshot",
                    side_effect=fake_host_snapshot,
                ),
                mock.patch.object(suite.subprocess, "run", side_effect=fake_child),
                mock.patch.object(
                    suite,
                    "summarize_reports",
                    return_value=[{"accepted": True}],
                ),
                mock.patch("builtins.print", side_effect=fake_print),
            ):
                self.assertEqual(suite.main(argv), 1)
                first_manifest = sorted(evals.glob("*-acceptance-suite.json"))[0]
                resumed_status = suite.main(
                    [*argv, "--resume-artifact", str(first_manifest)]
                )
                manifest_documents = [
                    (path, json.loads(path.read_text(encoding="utf-8")))
                    for path in evals.glob("*-acceptance-suite*.json")
                ]
                latest_manifest, latest_document = next(
                    item for item in manifest_documents if item[1].get("resumed_from")
                )
                self.assertEqual(
                    resumed_status,
                    0,
                    latest_document,
                )

            self.assertEqual(
                invoked_tags,
                [
                    "acceptance-cli-r1",
                    "acceptance-server-identical-r1",
                    "acceptance-server-identical-r1",
                ],
            )
            self.assertEqual(
                lifecycle_events,
                [
                    "child",
                    "postflight",
                    "report",
                    "child",
                    "postflight",
                    "report",
                    "child",
                    "postflight",
                    "report",
                ],
            )
            accepted = json.loads(latest_manifest.read_text(encoding="utf-8"))
            self.assertEqual(accepted["status"], "accepted")
            self.assertEqual(len(accepted["runs"]), 2)
            self.assertEqual(len(accepted["attempts"]), 3)


class ReferenceCompareTest(unittest.TestCase):
    @staticmethod
    def clean_snapshot():
        return {
            "load_per_core_1m": 0.1,
            "ferrite_processes": [],
            "process_observation_available": True,
            "thermal_status": None,
            "max_background_process_cpu_percent": 10.0,
            "top_background_cpu_processes": [],
        }

    @classmethod
    def reference_run(cls, repetition, contaminated_runtime=None):
        order = list(reference.runtime_order(repetition))
        results = {}
        for runtime in reference.RUNTIMES:
            postflight = cls.clean_snapshot()
            reasons = []
            if runtime == contaminated_runtime:
                postflight["max_background_process_cpu_percent"] = 75.0
                postflight["top_background_cpu_processes"] = [
                    {"pid": 42, "cpu_percent": 75.0, "command": "vm-helper"}
                ]
                reasons = [
                    "background process CPU 75.0% exceeds 50.0% "
                    "(vm-helper pid 42)"
                ]
            results[runtime] = {
                "preflight": cls.clean_snapshot(),
                "postflight": postflight,
                "postflight_rejection_reasons": reasons,
            }
        return {
            "repetition": repetition,
            "attempt": 1,
            "order": order,
            "results": results,
        }

    @staticmethod
    def resume_identity():
        return {
            "source_tree_sha256": "a" * 64,
            "config": {
                "repetitions": 3,
                "max_load_per_core": 0.25,
                "max_background_process_cpu_percent": 50.0,
            },
        }

    def test_pinned_revision_matches_full_or_short_version_output(self):
        revision = reference.PINNED_LLAMA_CPP_REVISION

        self.assertTrue(reference.llama_revision_matches(revision, revision))
        self.assertTrue(
            reference.llama_revision_matches(
                f"version: 9992 ({revision[:8]})", revision
            )
        )
        self.assertFalse(reference.llama_revision_matches("version: other", revision))

    def test_resume_retains_only_complete_clean_pairs(self):
        identity = self.resume_identity()
        clean = self.reference_run(1)
        contaminated = self.reference_run(2, contaminated_runtime="ferrite")
        previous = {
            "schema_version": 2,
            "status": "rejected",
            "started_utc": "2026-07-14T00:00:00Z",
            "resume_identity": identity,
            "resume_chain": ["first.json"],
            "attempts": [clean, contaminated],
            "runs": [clean],
        }

        attempts, runs, chain = reference.validate_resume_report(previous, identity)

        self.assertEqual(attempts, [clean, contaminated])
        self.assertEqual(runs, [clean])
        self.assertEqual(chain, ["first.json"])
        self.assertTrue(reference.run_is_clean(clean, identity["config"]))
        self.assertFalse(
            reference.run_is_clean(contaminated, identity["config"])
        )

    def test_resume_rejects_identity_drift(self):
        identity = self.resume_identity()
        previous = {
            "schema_version": 2,
            "status": "rejected",
            "started_utc": "2026-07-14T00:00:00Z",
            "resume_identity": identity,
            "resume_chain": [],
            "attempts": [],
            "runs": [],
        }
        drifted = json.loads(json.dumps(identity))
        drifted["source_tree_sha256"] = "b" * 64

        with self.assertRaisesRegex(ValueError, "identity does not match"):
            reference.validate_resume_report(previous, drifted)

    def test_resume_rejects_contaminated_selected_pair(self):
        identity = self.resume_identity()
        contaminated = self.reference_run(1, contaminated_runtime="llama_cpp")
        previous = {
            "schema_version": 2,
            "status": "rejected",
            "started_utc": "2026-07-14T00:00:00Z",
            "resume_identity": identity,
            "resume_chain": [],
            "attempts": [contaminated],
            "runs": [contaminated],
        }

        with self.assertRaisesRegex(ValueError, "incomplete or contaminated"):
            reference.validate_resume_report(previous, identity)

    def test_resume_rejects_attempt_number_gap(self):
        identity = self.resume_identity()
        attempt = self.reference_run(1)
        attempt["attempt"] = 2
        previous = {
            "schema_version": 2,
            "status": "rejected",
            "started_utc": "2026-07-14T00:00:00Z",
            "resume_identity": identity,
            "resume_chain": [],
            "attempts": [attempt],
            "runs": [],
        }

        with self.assertRaisesRegex(ValueError, "invalid attempt number"):
            reference.validate_resume_report(previous, identity)

    def test_unique_output_stem_never_reuses_either_artifact_name(self):
        with tempfile.TemporaryDirectory() as directory:
            output_dir = Path(directory)
            (output_dir / "comparison.json").write_text("{}", encoding="utf-8")
            (output_dir / "comparison-2.md").write_text("", encoding="utf-8")

            stem = reference.unique_output_stem("comparison", output_dir)

            self.assertEqual(stem, "comparison-3")

    def test_atomic_checkpoint_replaces_complete_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "comparison.json"
            path.write_text("old", encoding="utf-8")

            reference.atomic_write_text(path, "new\n")

            self.assertEqual(path.read_text(encoding="utf-8"), "new\n")
            self.assertEqual(list(path.parent.glob(f".{path.name}.*")), [])

    def test_reference_postflight_ignores_its_own_load_average(self):
        snapshot = {
            "load_per_core_1m": 1.0,
            "ferrite_processes": [],
            "thermal_status": ["Note: No thermal warning level has been recorded"],
        }

        self.assertEqual(reference.postflight_rejection_reasons(snapshot), [])

    def test_reference_postflight_rejects_new_background_cpu_spike(self):
        snapshot = {
            "load_per_core_1m": 1.0,
            "ferrite_processes": [],
            "thermal_status": None,
            "max_background_process_cpu_percent": 75.0,
            "top_background_cpu_processes": [
                {"pid": 42, "cpu_percent": 75.0, "command": "vm-helper"}
            ],
        }

        reasons = reference.postflight_rejection_reasons(
            snapshot, max_background_process_cpu_percent=50.0
        )

        self.assertEqual(len(reasons), 1)
        self.assertIn("vm-helper pid 42", reasons[0])

    def test_requests_use_equivalent_greedy_controls(self):
        ferrite = reference.ferrite_request("prompt", 8)
        llama = reference.llama_cpp_request("prompt", 8)

        self.assertEqual(ferrite["prompt"], llama["prompt"])
        self.assertEqual(ferrite["max_tokens"], llama["n_predict"])
        self.assertEqual(ferrite["temperature"], 0)
        self.assertEqual(llama["temperature"], 0)
        self.assertEqual(ferrite["top_p"], llama["top_p"])
        self.assertEqual(ferrite["seed"], llama["seed"])
        self.assertFalse(llama["cache_prompt"])
        self.assertTrue(llama["return_tokens"])

    def test_chat_requests_preserve_messages_and_use_rendered_llama_prompt(self):
        ferrite_endpoint, ferrite = reference.runtime_request(
            "ferrite", "hello", 8, request_mode="chat"
        )
        llama_endpoint, llama = reference.runtime_request(
            "llama_cpp",
            "hello",
            8,
            request_mode="chat",
            rendered_prompt="rendered prompt",
        )

        self.assertEqual(ferrite_endpoint, "/v1/chat/completions")
        self.assertEqual(ferrite["messages"][0]["content"], "hello")
        self.assertEqual(ferrite["max_completion_tokens"], 8)
        self.assertEqual(ferrite["temperature"], 0)
        self.assertTrue(ferrite["return_token_ids"])
        self.assertEqual(llama_endpoint, "/completion")
        self.assertEqual(llama["prompt"], "rendered prompt")
        self.assertTrue(llama["parse_special"])

    def test_chat_stream_summary_ignores_role_content(self):
        events = [
            {
                "choices": [
                    {
                        "delta": {"role": "assistant", "content": ""},
                        "finish_reason": None,
                    }
                ]
            },
            {
                "choices": [
                    {
                        "delta": {"content": "Hello"},
                        "token_ids": [7],
                        "finish_reason": None,
                    }
                ]
            },
            {
                "choices": [
                    {"delta": {}, "finish_reason": "length"}
                ]
            },
            {
                "choices": [],
                "usage": {"prompt_tokens": 3, "completion_tokens": 1},
            },
        ]

        result = reference.summarize_stream(
            "ferrite",
            events,
            10.0,
            10.2,
            [(10.1, [7])],
            request_mode="chat",
        )

        self.assertEqual(result["token_ids"], [7])
        self.assertEqual(result["content_bytes"], 5)
        self.assertEqual(result["finish_reason"], "length")
        self.assertEqual(result["usage"]["prompt_tokens"], 3)

    def test_llama_terminal_eos_is_recorded_but_not_content_associated(self):
        event = {
            "tokens": [99],
            "content": "",
            "stop": True,
            "stop_type": "eos",
        }

        self.assertEqual(reference.event_token_ids("llama_cpp", event), [99])
        self.assertEqual(reference.event_visible_token_ids("llama_cpp", event), [])

    def test_llama_split_terminal_eos_is_recorded_but_not_content_associated(self):
        timestamped_events = [
            (10.1, {"tokens": [99], "content": "", "stop": False}),
            (
                10.2,
                {
                    "tokens": [],
                    "content": "",
                    "stop": True,
                    "stop_type": "eos",
                },
            ),
        ]

        emitted, visible = reference.stream_token_chunks(
            "llama_cpp", timestamped_events
        )

        self.assertEqual(emitted, [(10.1, [99])])
        self.assertEqual(visible, [])

    def test_stream_summary_preserves_exact_token_ids_and_latency(self):
        events = [
            {"tokens": [7], "content": "a", "stop": False},
            {
                "tokens": [8],
                "content": "b",
                "stop": True,
                "timings": {"prompt_n": 3, "predicted_n": 2},
            },
        ]

        result = reference.summarize_stream(
            "llama_cpp", events, 10.0, 10.4, [(10.1, [7]), (10.3, [8])]
        )

        self.assertEqual(result["token_ids"], [7, 8])
        self.assertEqual(result["time_to_first_token_ms"], 100.0)
        self.assertEqual(result["inter_chunk_latency_ms_p50"], 200.0)
        self.assertEqual(result["usage"]["prompt_tokens"], 3)

    def test_three_stable_exact_pairs_are_accepted(self):
        runs = []
        for repetition in range(1, 4):
            record = {
                "token_ids": [1, 2, 3, 4],
                "token_count": 4,
                "usage": {"prompt_tokens": 6, "completion_tokens": 4},
                "time_to_first_token_ms": 100 + repetition,
                "post_first_chunk_tokens_per_second": 50.0,
            }
            runs.append(
                {
                    "results": {
                        "ferrite": dict(record),
                        "llama_cpp": dict(record),
                    }
                }
            )

        summary = reference.summarize_runs(runs, repetitions=3, max_tokens=4)

        self.assertTrue(summary["comparison"]["accepted"])
        self.assertTrue(summary["comparison"]["all_exact_token_id_pairs_match"])

    def test_mismatched_pair_is_rejected(self):
        runs = []
        for _ in range(3):
            runs.append(
                {
                    "results": {
                        "ferrite": {
                            "token_ids": [1, 2],
                            "token_count": 2,
                            "usage": {"prompt_tokens": 3},
                        },
                        "llama_cpp": {
                            "token_ids": [1, 9],
                            "token_count": 2,
                            "usage": {"prompt_tokens": 3},
                        },
                    }
                }
            )

        summary = reference.summarize_runs(runs, repetitions=3, max_tokens=2)

        self.assertFalse(summary["comparison"]["accepted"])

    def test_exact_recorded_near_tie_policy_accepts_only_recorded_traces(self):
        rendered_hash = reference.sha256_text("rendered")
        case = {
            "case_id": "near-tie",
            "expected_ferrite_token_ids": [1, 2],
            "expected_llama_cpp_token_ids": [1, 3],
            "prompt_tokens": 5,
            "rendered_prompt_sha256": rendered_hash,
            "expected_finish_reason": "stop",
        }
        runs = []
        for _ in range(3):
            ferrite = {
                "token_ids": [1, 2],
                "token_count": 2,
                "usage": {"prompt_tokens": 5, "completion_tokens": 2},
                "finish_reason": "stop",
            }
            llama = {
                "token_ids": [1, 3],
                "token_count": 2,
                "usage": {"prompt_tokens": 5, "completion_tokens": 2},
                "finish_reason": "stop",
                "rendered_prompt_sha256": rendered_hash,
            }
            runs.append({"results": {"ferrite": ferrite, "llama_cpp": llama}})

        summary = reference.summarize_runs(
            runs,
            repetitions=3,
            max_tokens=4,
            allow_early_stop=True,
            numerical_policy_case=case,
        )

        self.assertTrue(summary["comparison"]["accepted"])
        self.assertFalse(summary["comparison"]["all_exact_token_id_pairs_match"])
        self.assertEqual(
            summary["comparison"]["accepted_under"],
            "reviewed_numerical_policy",
        )
        runs[0]["results"]["ferrite"]["token_ids"] = [1, 4]
        rejected = reference.summarize_runs(
            runs,
            repetitions=3,
            max_tokens=4,
            allow_early_stop=True,
            numerical_policy_case=case,
        )
        self.assertFalse(rejected["comparison"]["accepted"])

    def test_policy_loader_requires_exact_static_identity(self):
        identity = {
            "model_sha256": "a" * 64,
            "prompt_sha256": "b" * 64,
            "request_mode": "chat",
            "max_tokens": 1,
            "llama_cpp_revision": reference.PINNED_LLAMA_CPP_REVISION,
        }
        case = {
            **identity,
            "case_id": "case",
            "expected_ferrite_token_ids": [1],
            "expected_llama_cpp_token_ids": [2],
            "first_divergence_index": 0,
            "prompt_tokens": 3,
            "rendered_prompt_sha256": "c" * 64,
            "recorded_absolute_logit_gap": 0.0005,
            "decision": "accept_exact_recorded_near_tie_trace",
        }
        policy = {
            "schema_version": 1,
            "policy_id": "test-policy",
            "review_status": "reviewed",
            "cases": [case],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "policy.json"
            path.write_text(json.dumps(policy), encoding="utf-8")
            loaded = reference.load_numerical_policy(path, identity)

        self.assertEqual(loaded["case"]["case_id"], "case")


class RenderMarkdownTest(unittest.TestCase):
    def test_report_exit_gate_rejects_failed_phase_or_route_mismatch(self):
        accepted = {
            "models": [
                {
                    "server": {"status": "ok"},
                    "batched_server": {
                        "status": "ok",
                        "token_ids_match_default": True,
                    },
                }
            ]
        }
        failed = json.loads(json.dumps(accepted))
        failed["models"][0]["server"]["status"] = "failed"
        mismatched = json.loads(json.dumps(accepted))
        mismatched["models"][0]["batched_server"][
            "token_ids_match_default"
        ] = False

        self.assertTrue(ev.report_succeeded(accepted))
        self.assertFalse(ev.report_succeeded(failed))
        self.assertFalse(ev.report_succeeded(mismatched))

    def test_report_renders_cli_and_server_sections(self):
        report = ev.build_report(
            env={"timestamp_utc": "2026-07-09T12:00:00Z", "hostname": "mac",
                 "cpu": "Apple M-test", "physical_cores": 8,
                 "ram_bytes": 16 << 30, "platform": "macOS", "python": "3.14",
                 "git_commit": "abc123", "git_branch": "main", "git_dirty": False,
                 "rustc_version": "rustc 1.x", "logical_cores": 8},
            cfg=ev.EvalConfig(
                "hi",
                64,
                64,
                2000,
                4,
                (2,),
                4,
                "identical",
                True,
                False,
                None,
                0,
                500,
                16 << 20,
            ),
            model_results=[{
                "model_path": "target/models/model.gguf",
                "model_sha256": "ab" * 32,
                "cli": {"status": "ok", "load_seconds": 1.0,
                        "inference_threads": 8,
                        "ttft_prefill_seconds": 0.5,
                        "decode_tokens_per_second_precise": 12.3,
                        "rss_peak_bytes": 1 << 30,
                        "batch_benchmarks": [{
                            "streams": 2,
                            "status": "ok",
                            "average_step_ns": 20_000_000,
                            "aggregate_tokens_per_second": 100.0,
                            "per_stream_tokens_per_second": 50.0,
                            "stream_0_matches_single": True,
                            "rss_peak_bytes": 2 << 30,
                        }]},
                "server": {"status": "ok",
                           "streaming_time_to_first_token_ms": "450",
                           "streaming_tokens_per_second": "11.5",
                           "streaming_all_token_id_traces_match": "true"},
                "batched_server": {
                    "status": "ok",
                    "concurrency": 4,
                    "aggregate_completion_tokens_per_second": 120.0,
                    "token_ids_match_default": True,
                },
            }],
            tag="unit-test",
        )
        self.assertEqual(report["schema_version"], ev.SCHEMA_VERSION)
        markdown = ev.render_markdown(report)
        self.assertIn("## model.gguf", markdown)
        self.assertIn(f"model SHA-256: `{'ab' * 32}`", markdown)
        self.assertIn("| load | 1.0 s |", markdown)
        self.assertIn("12.3", markdown)
        self.assertIn("| inference threads | 8 |", markdown)
        self.assertIn("| 2 | 100.0 | 50.0 | 20.00 ms | True |", markdown)
        self.assertIn("| first response TTFT | 450 ms |", markdown)
        self.assertIn("| all request token-ID traces match | true |", markdown)
        self.assertIn("| Continuous-batched server metric | value |", markdown)
        self.assertIn("| aggregate completion tok/s | 120.0 |", markdown)
        self.assertIn("| token IDs match default | True |", markdown)
        self.assertIn("tag: unit-test", markdown)


class CliExecutionFlagsTest(unittest.TestCase):
    def test_portable_kernel_provider_is_explicit(self):
        cfg = ev.EvalConfig(
            "hi",
            64,
            64,
            2000,
            4,
            (),
            None,
            "identical",
            False,
            False,
            None,
            0,
            500,
            16 << 20,
            False,
            "portable",
        )
        self.assertEqual(
            ev.cli_execution_flags(cfg), ["--kernel-provider", "portable"]
        )
        self.assertEqual(
            ev.cli_execution_flags(cfg._replace(threads=2)),
            ["--threads", "2", "--kernel-provider", "portable"],
        )

    def test_residual_activation_flag_is_explicit(self):
        enabled = ev.EvalConfig(
            "hi",
            64,
            64,
            2000,
            4,
            (),
            None,
            "identical",
            True,
            False,
            None,
            0,
            500,
            16 << 20,
        )
        disabled = enabled._replace(experimental_residual_q8_activation_matvec=False)
        self.assertEqual(
            ev.cli_execution_flags(enabled),
            ["--experimental-residual-q8-activation-matvec"],
        )
        self.assertEqual(ev.cli_execution_flags(disabled), [])

    def test_one_pass_policy_and_role_scope_are_explicit(self):
        cfg = ev.EvalConfig(
            "hi",
            64,
            64,
            2000,
            4,
            (),
            None,
            "identical",
            False,
            True,
            "q_proj,ffn_down",
            0,
            500,
            16 << 20,
        )
        self.assertEqual(
            ev.cli_execution_flags(cfg),
            [
                "--experimental-q8-k-activation-matvec",
                "--experimental-q8-k-activation-roles",
                "q_proj,ffn_down",
            ],
        )


if __name__ == "__main__":
    unittest.main()
