mod support;

use std::error::Error;
use std::process::Command;
use support::fixtures::{cli_binary, remove_fixture_model, write_fixture_model};

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
