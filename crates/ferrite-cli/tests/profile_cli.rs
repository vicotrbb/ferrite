mod support;

use std::error::Error;
use std::process::Command;
use support::fixtures::{
    cli_binary, remove_fixture_model, write_fixture_model, write_q4_k_fixture_model,
};
use support::q8_k::q8_k_compare_role_summary_has_drift_fields;

#[test]
fn cli_benchmarks_repeated_next_token_runs_after_loading_once() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--benchmark-runs")
        .arg("3")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("prompt_token_ids=1"));
    assert!(stdout.contains("next_token_id=2"));
    assert!(stdout.contains("benchmark_runs=3"));
    assert!(stdout.contains("benchmark_cached_tokens=4"));
    let benchmark_token_ids = stdout
        .lines()
        .find_map(|line| line.strip_prefix("benchmark_token_ids="))
        .ok_or("missing benchmark_token_ids")?;
    assert_eq!(benchmark_token_ids.split(',').count(), 3);
    assert!(stdout.contains("benchmark_total_ns="));
    assert!(stdout.contains("benchmark_avg_ns="));
    assert!(stdout.contains("model_file_bytes="));
    assert!(stdout.contains("model_file_retained_bytes=0"));
    assert!(stdout.contains("scalar_weight_bytes=184"));
    assert!(stdout.contains("kv_cache_bytes=64"));
    Ok(())
}

#[test]
fn cli_can_pause_after_model_load_for_memory_sampling() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--sleep-after-load-ms")
        .arg("1")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("sleep_after_load_ms=1"));
    assert!(stdout.contains("next_token_id=2"));
    Ok(())
}

#[test]
fn cli_benchmarks_tokenization_without_generation() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--benchmark-tokenization-runs")
        .arg("3")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("tokenization_benchmark_runs=3"));
    assert!(stdout.contains("tokenization_benchmark_prompt_bytes=5"));
    assert!(stdout.contains("tokenization_benchmark_token_count=1"));
    assert!(stdout.contains("tokenization_benchmark_total_ns="));
    assert!(stdout.contains("tokenization_benchmark_avg_ns="));
    assert!(stdout.contains("model_file_bytes="));
    assert!(stdout.contains("model_file_retained_bytes=0"));
    assert!(!stdout.contains("next_token_id="));
    Ok(())
}

#[test]
fn cli_profiles_benchmark_token_id_decode() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--benchmark-runs")
        .arg("2")
        .arg("--profile-benchmark-token")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let profile_total_ns = stdout
        .lines()
        .find_map(|line| line.strip_prefix("profile_benchmark_token_total_ns="))
        .ok_or("missing profile_benchmark_token_total_ns")?;
    assert!(profile_total_ns.parse::<u128>()? > 0);
    assert!(stdout.contains("benchmark_runs=2"));
    assert!(stdout.contains("profile_benchmark_token_input_id=2"));
    assert!(stdout.contains("profile_benchmark_token_id=2"));
    assert!(stdout.contains("profile_benchmark_token_op=output:"));
    assert!(stdout.contains("profile_benchmark_token_matrix=output:F32:3:2:24"));
    assert!(stdout.contains("profile_benchmark_token_role=output:F32:3:2:24:"));
    Ok(())
}

#[test]
fn cli_compares_q8_k_activation_matvec_for_benchmark_token_profile() -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--benchmark-runs")
        .arg("2")
        .arg("--profile-benchmark-token")
        .arg("--compare-q8-k-activation-matvec")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("compare_q8_k_activation_matvec=true"));
    assert!(stdout.contains("profile_benchmark_token_q8_k_compare=layer.0.q_proj:"));
    assert!(q8_k_compare_role_summary_has_drift_fields(
        &stdout,
        "profile_benchmark_token_q8_k_compare_role=q_proj:"
    ));
    Ok(())
}

#[test]
fn cli_profiles_next_token_scalar_operations() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--profile-next-token")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let profile_total_ns = stdout
        .lines()
        .find_map(|line| line.strip_prefix("profile_next_token_total_ns="))
        .ok_or("missing profile_next_token_total_ns")?;
    assert!(profile_total_ns.parse::<u128>()? > 0);
    assert!(stdout.contains("profile_next_token_op=layer.0.q_proj:"));
    assert!(stdout.contains("profile_next_token_op=layer.0.ffn_down:"));
    assert!(stdout.contains("profile_next_token_op=output:"));
    assert!(stdout.contains("profile_next_token_matrix=layer.0.q_proj:F32:2:2:16"));
    assert!(stdout.contains("profile_next_token_matrix=layer.0.ffn_down:F32:2:2:16"));
    assert!(stdout.contains("profile_next_token_matrix=output:F32:3:2:24"));
    assert!(stdout.contains("profile_next_token_role=q_proj:F32:2:2:16:"));
    assert!(stdout.contains("profile_next_token_role=ffn_down:F32:2:2:16:"));
    assert!(stdout.contains("profile_next_token_role=output:F32:3:2:24:"));
    Ok(())
}
