use ferrite_server::long_chat_gate::LongChatGateConfig;
use std::ffi::OsString;

#[test]
fn defaults_to_required_long_chat_token_lengths_and_turns() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([OsString::from("ferrite-openai-long-chat-gate")])?;

    assert_eq!(config.token_lengths(), &[256, 512, 1024]);
    assert_eq!(config.turns(), 4);
    Ok(())
}

#[test]
fn parses_custom_long_chat_token_lengths_and_turns() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("128,256"),
        OsString::from("--turns"),
        OsString::from("5"),
    ])?;

    assert_eq!(config.token_lengths(), &[128, 256]);
    assert_eq!(config.turns(), 5);
    Ok(())
}

#[test]
fn rejects_too_few_long_chat_turns() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--turns"),
        OsString::from("3"),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--turns"), "{error}");
    assert!(error.to_string().contains("at least 4"), "{error}");
    Ok(())
}

#[test]
fn rejects_empty_long_chat_token_lengths() -> Result<(), Box<dyn std::error::Error>> {
    let result = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from(""),
    ]);
    let error = match result {
        Ok(config) => return Err(format!("expected error, got config: {config:?}").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("--token-lengths"), "{error}");
    Ok(())
}
