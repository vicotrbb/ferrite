use super::LongChatScenario;
use std::{error::Error, ffi::OsString, fmt, time::Duration};

const DEFAULT_DISCONNECT_RECONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatGateConfig {
    addr: String,
    api_key: String,
    execute: bool,
    error_probe: bool,
    disconnect_probe: bool,
    require_cached_follow_ups: bool,
    models: Vec<String>,
    prompt: String,
    assistant_context: String,
    follow_up: String,
    prompt_cache_key: Option<String>,
    stop: Option<String>,
    expected_finish_reason: Option<String>,
    probe_max_tokens: Option<usize>,
    disconnect_reconnect_timeout: Duration,
    rss_pid: Option<u32>,
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
                "--execute" => {
                    config.execute = true;
                }
                "--error-probe" => {
                    config.error_probe = true;
                }
                "--disconnect-probe" => {
                    config.disconnect_probe = true;
                }
                "--require-cached-follow-ups" => {
                    config.require_cached_follow_ups = true;
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
                "--prompt-cache-key" => {
                    config.prompt_cache_key = Some(parse_non_empty_string(
                        next_value(&mut iter, "--prompt-cache-key")?,
                        "--prompt-cache-key",
                    )?);
                }
                "--stop" => {
                    config.stop = Some(parse_non_empty_string(
                        next_value(&mut iter, "--stop")?,
                        "--stop",
                    )?);
                }
                "--expect-finish-reason" => {
                    config.expected_finish_reason = Some(parse_non_empty_string(
                        next_value(&mut iter, "--expect-finish-reason")?,
                        "--expect-finish-reason",
                    )?);
                }
                "--probe-max-tokens" => {
                    config.probe_max_tokens = Some(parse_positive_usize(
                        &os_string_to_string(next_value(&mut iter, "--probe-max-tokens")?)?,
                        "--probe-max-tokens",
                    )?);
                }
                "--disconnect-reconnect-timeout-ms" => {
                    config.disconnect_reconnect_timeout =
                        Duration::from_millis(parse_positive_u64(
                            next_value(&mut iter, "--disconnect-reconnect-timeout-ms")?,
                            "--disconnect-reconnect-timeout-ms",
                        )?);
                }
                "--rss-pid" => {
                    config.rss_pid = Some(parse_positive_u32(
                        next_value(&mut iter, "--rss-pid")?,
                        "--rss-pid",
                    )?);
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

        if config.require_cached_follow_ups && config.prompt_cache_key.is_none() {
            return Err(LongChatGateError::new(
                "--require-cached-follow-ups requires --prompt-cache-key",
            ));
        }

        Ok(config)
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn execute(&self) -> bool {
        self.execute
    }

    pub fn error_probe(&self) -> bool {
        self.error_probe
    }

    pub fn disconnect_probe(&self) -> bool {
        self.disconnect_probe
    }

    pub fn require_cached_follow_ups(&self) -> bool {
        self.require_cached_follow_ups
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

    pub fn prompt_cache_key(&self) -> Option<&str> {
        self.prompt_cache_key.as_deref()
    }

    pub fn stop(&self) -> Option<&str> {
        self.stop.as_deref()
    }

    pub fn expected_finish_reason(&self) -> Option<&str> {
        self.expected_finish_reason.as_deref()
    }

    pub fn probe_max_tokens(&self) -> Option<usize> {
        self.probe_max_tokens
    }

    pub fn disconnect_reconnect_timeout(&self) -> Duration {
        self.disconnect_reconnect_timeout
    }

    pub fn rss_pid(&self) -> Option<u32> {
        self.rss_pid
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
            execute: false,
            error_probe: false,
            disconnect_probe: false,
            require_cached_follow_ups: false,
            models: vec![
                "Qwen2.5-0.5B-Instruct-Q4_K_M".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q8_0".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q6_K".to_owned(),
                "SmolLM2-1.7B-Instruct-Q4_K_M".to_owned(),
            ],
            prompt: "Write a concise paragraph about CPU inference.".to_owned(),
            assistant_context: "CPU inference prioritizes memory locality, predictable scheduling, and efficient token streaming.".to_owned(),
            follow_up: "Continue with the operational risks for a long streaming chat.".to_owned(),
            prompt_cache_key: None,
            stop: None,
            expected_finish_reason: None,
            probe_max_tokens: None,
            disconnect_reconnect_timeout: DEFAULT_DISCONNECT_RECONNECT_TIMEOUT,
            rss_pid: None,
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
    parse_non_empty_list(value, "--token-lengths")?
        .into_iter()
        .map(|part| parse_positive_usize(&part, "--token-lengths"))
        .collect()
}

fn parse_positive_u32(value: OsString, flag: &str) -> Result<u32, LongChatGateError> {
    let parsed: u32 = parse_positive_u64(value, flag)?
        .try_into()
        .map_err(|error| LongChatGateError::new(format!("invalid {flag}: {error}")))?;
    Ok(parsed)
}

fn parse_positive_u64(value: OsString, flag: &str) -> Result<u64, LongChatGateError> {
    let parsed: u64 = os_string_to_string(value)?
        .parse()
        .map_err(|error| LongChatGateError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(LongChatGateError::new(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(parsed)
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
    "usage: ferrite-openai-long-chat-gate [--execute] [--error-probe] [--disconnect-probe] [--require-cached-follow-ups] [--addr 127.0.0.1:8080] [--api-key local-secret] [--models MODEL[,MODEL...]] [--prompt TEXT] [--assistant-context TEXT] [--follow-up TEXT] [--prompt-cache-key KEY] [--stop TEXT] [--expect-finish-reason REASON] [--probe-max-tokens TOKENS] [--disconnect-reconnect-timeout-ms 30000] [--rss-pid PID] [--token-lengths 256,512,1024] [--turns 4]"
}
