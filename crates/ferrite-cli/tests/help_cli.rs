use std::process::Command;

#[test]
fn help_exits_successfully_without_required_model_arguments(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_ferrite"))
        .arg("--help")
        .output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("usage: ferrite "));
    assert!(stdout.contains("--threads <count>"));
    assert!(stdout.contains("--benchmark-batch-streams <count>"));
    Ok(())
}

#[test]
fn version_exits_successfully_without_required_model_arguments(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_ferrite"))
        .arg("--version")
        .output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!("ferrite {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}
