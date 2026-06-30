mod throughput;

use std::{error::Error, ffi::OsString, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatGateConfig {
    addr: String,
    api_key: String,
    models: Vec<String>,
    prompt: String,
    assistant_context: String,
    follow_up: String,
    token_lengths: Vec<usize>,
    turns: usize,
}

impl LongChatGateConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, LongChatGateError> {
        let mut config = Self::default();
        let mut iter = args.into_iter();
        let _program = iter.next();

        while let Some(arg) = iter.next() {
            let flag = arg
                .to_str()
                .ok_or_else(|| LongChatGateError::new("arguments must be valid UTF-8"))?;
            match flag {
                "--addr" => {
                    config.addr =
                        parse_non_empty_string(next_value(&mut iter, "--addr")?, "--addr")?;
                }
                "--api-key" => {
                    config.api_key =
                        parse_non_empty_string(next_value(&mut iter, "--api-key")?, "--api-key")?;
                }
                "--models" => {
                    config.models =
                        parse_non_empty_list(next_value(&mut iter, "--models")?, "--models")?;
                }
                "--prompt" => {
                    config.prompt =
                        parse_non_empty_string(next_value(&mut iter, "--prompt")?, "--prompt")?;
                }
                "--assistant-context" => {
                    config.assistant_context = parse_non_empty_string(
                        next_value(&mut iter, "--assistant-context")?,
                        "--assistant-context",
                    )?;
                }
                "--follow-up" => {
                    config.follow_up = parse_non_empty_string(
                        next_value(&mut iter, "--follow-up")?,
                        "--follow-up",
                    )?;
                }
                "--token-lengths" => {
                    config.token_lengths =
                        parse_token_lengths(next_value(&mut iter, "--token-lengths")?)?;
                }
                "--turns" => {
                    config.turns = parse_turns(next_value(&mut iter, "--turns")?)?;
                }
                "--help" | "-h" => return Err(LongChatGateError::new(usage())),
                other => {
                    return Err(LongChatGateError::new(format!(
                        "unknown argument {other}\n{}",
                        usage()
                    )));
                }
            }
        }

        Ok(config)
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn token_lengths(&self) -> &[usize] {
        &self.token_lengths
    }

    pub fn models(&self) -> &[String] {
        &self.models
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn assistant_context(&self) -> &str {
        &self.assistant_context
    }

    pub fn follow_up(&self) -> &str {
        &self.follow_up
    }

    pub fn turns(&self) -> usize {
        self.turns
    }

    pub fn planned_scenarios(&self) -> usize {
        self.models.len() * self.token_lengths.len() * self.turns
    }

    pub fn scenarios(&self) -> Vec<LongChatScenario<'_>> {
        let mut scenarios = Vec::with_capacity(self.planned_scenarios());
        for model in &self.models {
            for turn in 1..=self.turns {
                for token_length in &self.token_lengths {
                    scenarios.push(LongChatScenario::new(model, turn, *token_length));
                }
            }
        }
        scenarios
    }
}

impl Default for LongChatGateConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:8080".to_owned(),
            api_key: "local-secret".to_owned(),
            models: vec![
                "Qwen2.5-0.5B-Instruct-Q4_K_M".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q8_0".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q6_K".to_owned(),
                "SmolLM2-1.7B-Instruct-Q4_K_M".to_owned(),
            ],
            prompt: "Write a concise paragraph about CPU inference.".to_owned(),
            assistant_context: "CPU inference prioritizes memory locality, predictable scheduling, and efficient token streaming.".to_owned(),
            follow_up: "Continue with the operational risks for a long streaming chat.".to_owned(),
            token_lengths: vec![256, 512, 1024],
            turns: 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatGateError {
    message: String,
}

impl LongChatGateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LongChatGateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for LongChatGateError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongChatScenario<'a> {
    model: &'a str,
    turn: usize,
    token_length: usize,
}

impl<'a> LongChatScenario<'a> {
    fn new(model: &'a str, turn: usize, token_length: usize) -> Self {
        Self {
            model,
            turn,
            token_length,
        }
    }

    pub fn model(&self) -> &str {
        self.model
    }

    pub fn turn(&self) -> usize {
        self.turn
    }

    pub fn token_length(&self) -> usize {
        self.token_length
    }
}

fn next_value(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<OsString, LongChatGateError> {
    iter.next()
        .ok_or_else(|| LongChatGateError::new(format!("missing value for {flag}")))
}

fn parse_token_lengths(value: OsString) -> Result<Vec<usize>, LongChatGateError> {
    parse_non_empty_list(value, "--token-lengths")?
        .into_iter()
        .map(|part| parse_positive_usize(&part, "--token-lengths"))
        .collect()
}

fn parse_non_empty_list(value: OsString, flag: &str) -> Result<Vec<String>, LongChatGateError> {
    let value = parse_non_empty_string(value, flag)?;

    value
        .split(',')
        .map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return Err(LongChatGateError::new(format!(
                    "{flag} must not contain empty entries"
                )));
            }
            Ok(part.to_owned())
        })
        .collect()
}

fn parse_non_empty_string(value: OsString, flag: &str) -> Result<String, LongChatGateError> {
    let value = os_string_to_string(value)?;
    if value.trim().is_empty() {
        return Err(LongChatGateError::new(format!(
            "{flag} must contain at least one value"
        )));
    }
    Ok(value)
}

fn parse_turns(value: OsString) -> Result<usize, LongChatGateError> {
    let turns = parse_positive_usize(&os_string_to_string(value)?, "--turns")?;
    if turns < 4 {
        return Err(LongChatGateError::new("--turns must be at least 4"));
    }
    Ok(turns)
}

fn parse_positive_usize(value: &str, flag: &str) -> Result<usize, LongChatGateError> {
    let parsed: usize = value
        .parse()
        .map_err(|error| LongChatGateError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(LongChatGateError::new(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(parsed)
}

fn os_string_to_string(value: OsString) -> Result<String, LongChatGateError> {
    value
        .into_string()
        .map_err(|_| LongChatGateError::new("arguments must be valid UTF-8"))
}

fn usage() -> &'static str {
    "usage: ferrite-openai-long-chat-gate [--addr 127.0.0.1:8080] [--api-key local-secret] [--models MODEL[,MODEL...]] [--prompt TEXT] [--assistant-context TEXT] [--follow-up TEXT] [--token-lengths 256,512,1024] [--turns 4]"
}

pub fn format_plan(config: &LongChatGateConfig) -> String {
    let models = config.models().join(",");
    let token_lengths = config
        .token_lengths()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "long_chat_models={models}\nlong_chat_token_lengths={token_lengths}\nlong_chat_turns={}\nlong_chat_planned_scenarios={}",
        config.turns(),
        config.planned_scenarios()
    )
}

pub fn format_scenarios(config: &LongChatGateConfig) -> String {
    config
        .scenarios()
        .iter()
        .map(|scenario| {
            format!(
                "long_chat_scenario=model:{},turn:{},max_tokens:{}",
                scenario.model(),
                scenario.turn(),
                scenario.token_length()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_report(config: &LongChatGateConfig) -> String {
    format!("{}\n{}", format_plan(config), format_scenarios(config))
}
