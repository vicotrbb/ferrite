mod support;

use std::error::Error;
use std::process::Command;
use support::fixtures::{cli_binary, remove_fixture_model, write_q4_k_fixture_model};
use support::q8_k::{
    q8_k_compare_line_has_argmax_indexes_and_margins, q8_k_compare_role_summary_has_drift_fields,
};

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
