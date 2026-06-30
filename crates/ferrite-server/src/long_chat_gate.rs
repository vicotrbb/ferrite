use std::{error::Error, ffi::OsString, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatGateConfig {
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

    pub fn token_lengths(&self) -> &[usize] {
        &self.token_lengths
    }

    pub fn turns(&self) -> usize {
        self.turns
    }
}

impl Default for LongChatGateConfig {
    fn default() -> Self {
        Self {
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

fn next_value(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<OsString, LongChatGateError> {
    iter.next()
        .ok_or_else(|| LongChatGateError::new(format!("missing value for {flag}")))
}

fn parse_token_lengths(value: OsString) -> Result<Vec<usize>, LongChatGateError> {
    let value = os_string_to_string(value)?;
    if value.trim().is_empty() {
        return Err(LongChatGateError::new(
            "--token-lengths must contain at least one length",
        ));
    }

    value
        .split(',')
        .map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return Err(LongChatGateError::new(
                    "--token-lengths must not contain empty entries",
                ));
            }
            parse_positive_usize(part, "--token-lengths")
        })
        .collect()
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
    "usage: ferrite-openai-long-chat-gate [--token-lengths 256,512,1024] [--turns 4]"
}
