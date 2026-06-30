use ferrite_server::long_chat_gate::{
    format_plan, format_report, format_scenarios, LongChatGateConfig,
};
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

#[test]
fn formats_long_chat_gate_plan_summary() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512,1024"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    assert_eq!(
        format_plan(&config),
        "long_chat_token_lengths=256,512,1024\nlong_chat_turns=4\nlong_chat_planned_scenarios=12"
    );
    Ok(())
}

#[test]
fn expands_ordered_long_chat_gate_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    let scenarios = config.scenarios();

    assert_eq!(scenarios.len(), 8);
    assert_eq!(scenarios[0].turn(), 1);
    assert_eq!(scenarios[0].token_length(), 256);
    assert_eq!(scenarios[1].turn(), 1);
    assert_eq!(scenarios[1].token_length(), 512);
    assert_eq!(scenarios[6].turn(), 4);
    assert_eq!(scenarios[6].token_length(), 256);
    assert_eq!(scenarios[7].turn(), 4);
    assert_eq!(scenarios[7].token_length(), 512);
    Ok(())
}

#[test]
fn formats_long_chat_gate_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    assert_eq!(
        format_scenarios(&config),
        "long_chat_scenario=turn:1,max_tokens:256\nlong_chat_scenario=turn:1,max_tokens:512\nlong_chat_scenario=turn:2,max_tokens:256\nlong_chat_scenario=turn:2,max_tokens:512\nlong_chat_scenario=turn:3,max_tokens:256\nlong_chat_scenario=turn:3,max_tokens:512\nlong_chat_scenario=turn:4,max_tokens:256\nlong_chat_scenario=turn:4,max_tokens:512"
    );
    Ok(())
}

#[test]
fn formats_long_chat_gate_report_with_plan_and_scenarios() -> Result<(), Box<dyn std::error::Error>>
{
    let config = LongChatGateConfig::parse([
        OsString::from("ferrite-openai-long-chat-gate"),
        OsString::from("--token-lengths"),
        OsString::from("256,512"),
        OsString::from("--turns"),
        OsString::from("4"),
    ])?;

    assert_eq!(
        format_report(&config),
        "long_chat_token_lengths=256,512\nlong_chat_turns=4\nlong_chat_planned_scenarios=8\nlong_chat_scenario=turn:1,max_tokens:256\nlong_chat_scenario=turn:1,max_tokens:512\nlong_chat_scenario=turn:2,max_tokens:256\nlong_chat_scenario=turn:2,max_tokens:512\nlong_chat_scenario=turn:3,max_tokens:256\nlong_chat_scenario=turn:3,max_tokens:512\nlong_chat_scenario=turn:4,max_tokens:256\nlong_chat_scenario=turn:4,max_tokens:512"
    );
    Ok(())
}
