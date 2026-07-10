use std::process::Command;

#[test]
fn short_help_exits_successfully_without_starting_the_server(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_ferrite-server"))
        .arg("-h")
        .output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("usage: ferrite-server "));
    assert!(stdout.contains("--threads N"));
    assert!(stdout.contains("--max-concurrent-inferences 1"));
    Ok(())
}
