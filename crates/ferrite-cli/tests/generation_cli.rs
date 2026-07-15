mod support;

use std::error::Error;
use std::process::Command;
use support::fixtures::{
    cli_binary, remove_fixture_model, write_fixture_model, write_fixture_model_with_eos_token_id,
    write_fixture_model_with_eot_token_id,
};

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
        .arg("--stream")
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
    assert!(stdout.contains("stream_token_id=2"));
    assert!(stdout.lines().any(|line| line == "stream_text="));
    assert!(stdout.lines().any(|line| line == "generated_text="));
    Ok(())
}

#[test]
fn cli_stops_generation_after_eot_token() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model_with_eot_token_id(2)?;
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
    assert!(stdout.lines().any(|line| line == "generated_text="));
    Ok(())
}

#[test]
fn cli_applies_sampling_logit_bias() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("2")
        .arg("--temperature")
        .arg("0")
        .arg("--logit-bias")
        .arg("1:100")
        .arg("--seed")
        .arg("42")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("sampling_fused_greedy_path=false"));
    assert!(stdout.contains("sampling_effective_seed=42"));
    assert!(stdout.contains("generated_token_ids=1,1"));
    assert!(stdout.contains("generated_text=hellohello"));
    Ok(())
}

#[test]
fn cli_stops_on_configured_stop_token() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt")
        .arg("hello")
        .arg("--generate-tokens")
        .arg("3")
        .arg("--stop-token-ids")
        .arg("2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(
        output.status.success(),
        "cli failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("generated_token_ids=2"));
    assert!(stdout.contains("generated_stopped_on_eos=false"));
    assert!(stdout.contains("generated_stopped_on_stop_token=true"));
    assert!(stdout.lines().any(|line| line == "generated_text="));
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
fn cli_rejects_tokenization_benchmark_without_text_prompt() -> Result<(), Box<dyn Error>> {
    let model_path = write_fixture_model()?;
    let binary = cli_binary()?;

    let output = Command::new(binary)
        .arg("--model")
        .arg(&model_path)
        .arg("--prompt-token-ids")
        .arg("1")
        .arg("--benchmark-tokenization-runs")
        .arg("2")
        .output()?;

    remove_fixture_model(&model_path)?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("use --benchmark-tokenization-runs with --prompt"));
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
