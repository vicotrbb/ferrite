mod support;

use std::error::Error;
use std::process::Command;
use support::fixtures::{
    cli_binary, remove_fixture_model, write_fixture_model, write_fixture_model_with_eos_token_id,
    write_q4_k_fixture_model,
};
use support::q8_k::{
    q8_k_compare_line_has_argmax_indexes_and_margins, q8_k_compare_role_summary_has_drift_fields,
};

#[test]
fn cli_loads_gguf_and_prints_text_prompt_next_token() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("q8_k_activation_matvec_policy=default_only"));
    assert!(stdout.contains("next_token_id=2"));
    assert!(stdout.contains("next_token=winner"));
    Ok(())
}

#[test]
fn cli_loads_gguf_and_prints_token_id_prompt_next_token() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("1")
        .arg("--expect-token-id")
        .arg("2")
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
    assert!(stdout.contains("next_token=winner"));
    assert!(stdout.contains("match=true"));
    Ok(())
}

#[test]
fn cli_enables_experimental_q8_k_activation_matvec() -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--expect-token-id")
        .arg("1")
        .arg("--experimental-q8-k-activation-matvec")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("experimental_q8_k_activation_matvec=true"));
    assert!(stdout.contains("q8_k_activation_matvec_policy=experimental_parity_scoped"));
    assert!(stdout.contains("next_token_id=1"));
    assert!(stdout.contains("match=true"));
    Ok(())
}

#[test]
fn cli_compares_q8_k_activation_matvec_without_changing_execution_policy(
) -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--profile-next-token")
        .arg("--compare-q8-k-activation-matvec")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("experimental_q8_k_activation_matvec=false"));
    assert!(stdout.contains("compare_q8_k_activation_matvec=true"));
    assert!(stdout.contains("q8_k_activation_matvec_policy=default_only"));
    assert!(stdout.contains("profile_next_token_q8_k_compare=layer.0.q_proj:"));
    assert!(q8_k_compare_line_has_argmax_indexes_and_margins(
        &stdout,
        "profile_next_token_q8_k_compare=layer.0.q_proj:"
    ));
    assert!(q8_k_compare_role_summary_has_drift_fields(
        &stdout,
        "profile_next_token_q8_k_compare_role=q_proj:"
    ));
    Ok(())
}

#[test]
fn cli_scopes_experimental_q8_k_activation_matvec_roles() -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--profile-next-token")
        .arg("--experimental-q8-k-activation-matvec")
        .arg("--compare-q8-k-activation-matvec")
        .arg("--experimental-q8-k-activation-roles")
        .arg("ffn_down")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("experimental_q8_k_activation_matvec=true"));
    assert!(stdout.contains("q8_k_activation_matvec_policy=experimental_parity_scoped"));
    assert!(stdout.contains("q8_k_activation_matvec_roles=ffn_down"));
    assert!(stdout.contains("profile_next_token_q8_k_compare=layer.0.ffn_down:"));
    assert!(!stdout.contains("profile_next_token_q8_k_compare=layer.0.q_proj:"));
    Ok(())
}

#[test]
fn cli_scopes_q8_k_comparison_roles_without_changing_execution_policy() -> Result<(), Box<dyn Error>>
{
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--profile-next-token")
        .arg("--compare-q8-k-activation-matvec")
        .arg("--experimental-q8-k-activation-roles")
        .arg("ffn_down")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("experimental_q8_k_activation_matvec=false"));
    assert!(stdout.contains("q8_k_activation_matvec_policy=default_only"));
    assert!(stdout.contains("q8_k_activation_matvec_roles=ffn_down"));
    assert!(stdout.contains("profile_next_token_q8_k_compare=layer.0.ffn_down:"));
    assert!(!stdout.contains("profile_next_token_q8_k_compare=layer.0.q_proj:"));
    Ok(())
}

#[test]
fn cli_accepts_all_q8_k_activation_role_scope() -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--profile-next-token")
        .arg("--compare-q8-k-activation-matvec")
        .arg("--experimental-q8-k-activation-roles")
        .arg("all")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("q8_k_activation_matvec_roles=all"));
    assert!(stdout.contains("profile_next_token_q8_k_compare=layer.0.q_proj:"));
    assert!(stdout.contains("profile_next_token_q8_k_compare=layer.0.ffn_down:"));
    Ok(())
}

#[test]
fn cli_rejects_q8_k_role_scope_without_comparison_or_experimental_dispatch(
) -> Result<(), Box<dyn Error>> {
    let model_path = write_q4_k_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("0")
        .arg("--experimental-q8-k-activation-roles")
        .arg("ffn_down")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains(
        "use --experimental-q8-k-activation-roles with --experimental-q8-k-activation-matvec or --compare-q8-k-activation-matvec"
    ));
    Ok(())
}

#[test]
fn cli_rejects_mixed_text_and_token_id_prompts() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--prompt-token-ids")
        .arg("1")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("use either --prompt or --prompt-token-ids"));
    Ok(())
}

#[test]
fn cli_succeeds_when_next_token_matches_expected_id() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--expect-token-id")
        .arg("2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("expected_token_id=2"));
    assert!(stdout.contains("match=true"));
    Ok(())
}

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

#[test]
fn cli_prints_top_next_token_logits() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--top-logits")
        .arg("2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let top_logits = stdout
        .lines()
        .find_map(|line| line.strip_prefix("top_logits="))
        .ok_or("missing top_logits")?;
    let entries = top_logits.split(',').collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].starts_with("2:"));
    Ok(())
}

#[test]
fn cli_generates_token_ids_and_decoded_text() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
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
    assert!(stdout.contains("generated_cached_tokens=3"));
    assert!(stdout.contains("generated_token_ids=2,2"));
    assert!(stdout.contains("generated_text=winnerwinner"));
    Ok(())
}

#[test]
fn cli_stops_generation_after_eos_token() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model_with_eos_token_id(2)?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("3")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("generated_cached_tokens=1"));
    assert!(stdout.contains("generated_token_ids=2"));
    assert!(stdout.contains("generated_stopped_on_eos=true"));
    assert!(stdout.contains("generated_text=winner"));
    Ok(())
}

#[test]
fn cli_succeeds_when_generated_tokens_match_expected_ids() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
        .arg("--expect-generated-token-ids")
        .arg("2,2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("expected_generated_token_ids=2,2"));
    assert!(stdout.contains("generated_match=true"));
    Ok(())
}

#[test]
fn cli_fails_when_generated_tokens_do_not_match_expected_ids() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
        .arg("--expect-generated-token-ids")
        .arg("2,1")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("expected_generated_token_ids=2,1"));
    assert!(stdout.contains("generated_match=false"));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("generated token ids 2,2 did not match expected token ids 2,1"));
    Ok(())
}

#[test]
fn cli_rejects_expected_generated_tokens_without_generation_count() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--expect-generated-token-ids")
        .arg("2,2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("use --expect-generated-token-ids with --generate-tokens"));
    Ok(())
}

#[test]
fn cli_rejects_generation_and_benchmark_together() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
        .arg("--benchmark-runs")
        .arg("2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("use either --generate-tokens or --benchmark-runs, not both"));
    Ok(())
}

#[test]
fn cli_streams_generated_token_chunks() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
        .arg("--stream")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.matches("stream_token_id=2").count(), 2);
    assert_eq!(stdout.matches("stream_text=winner").count(), 2);
    assert!(stdout.contains("generated_token_ids=2,2"));
    assert!(stdout.contains("generated_text=winnerwinner"));
    Ok(())
}

#[test]
fn cli_rejects_stream_without_generation_count() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--stream")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("use --stream with --generate-tokens"));
    Ok(())
}

#[test]
fn cli_fails_when_next_token_does_not_match_expected_id() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--expect-token-id")
        .arg("1")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("expected_token_id=1"));
    assert!(stdout.contains("match=false"));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("did not match expected token id 1"));
    Ok(())
}
