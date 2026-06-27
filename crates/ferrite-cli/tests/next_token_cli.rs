use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn cli_binary() -> Result<OsString, Box<dyn Error>> {
    std::env::var_os("CARGO_BIN_EXE_ferrite").ok_or_else(|| "missing CARGO_BIN_EXE_ferrite".into())
}

fn write_fixture_model() -> Result<PathBuf, Box<dyn Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-cli-fixture-{}-{}.gguf",
        std::process::id(),
        unique_suffix()
    ));
    fs::write(&path, scalar_llama_f32_gguf_fixture())?;
    Ok(path)
}

fn remove_fixture_model(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn unique_suffix() -> u128 {
    u128::from(FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed))
}
