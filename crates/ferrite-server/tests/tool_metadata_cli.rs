use std::process::Command;

fn assert_metadata_commands(
    binary: &str,
    expected_name: &str,
    usage_prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let help = Command::new(binary).arg("--help").output()?;
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    assert!(String::from_utf8(help.stdout)?.starts_with(usage_prefix));

    let version = Command::new(binary).arg("--version").output()?;
    assert!(version.status.success());
    assert!(version.stderr.is_empty());
    assert_eq!(
        String::from_utf8(version.stdout)?,
        format!("{expected_name} {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn throughput_tool_supports_help_and_version() -> Result<(), Box<dyn std::error::Error>> {
    assert_metadata_commands(
        env!("CARGO_BIN_EXE_ferrite-openai-throughput"),
        "ferrite-openai-throughput",
        "usage: ferrite-openai-throughput ",
    )
}

#[test]
fn long_chat_gate_supports_help_and_version() -> Result<(), Box<dyn std::error::Error>> {
    assert_metadata_commands(
        env!("CARGO_BIN_EXE_ferrite-openai-long-chat-gate"),
        "ferrite-openai-long-chat-gate",
        "usage: ferrite-openai-long-chat-gate ",
    )
}
