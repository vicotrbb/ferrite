use super::{LongChatScenario, LongChatStateCapsulePlacement};
use std::{
    error::Error,
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_DISCONNECT_RECONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatGateConfig {
    addr: String,
    api_key: String,
    execute: bool,
    error_probe: bool,
    disconnect_probe: bool,
    queue_probe: bool,
    required_probes: Vec<LongChatRequiredProbe>,
    require_cached_follow_ups: bool,
    models: Vec<String>,
    required_models: Vec<String>,
    prompt: String,
    assistant_context: String,
    follow_up: String,
    prompt_cache_key: Option<String>,
    prompt_cache_keys: Vec<String>,
    prompt_cache_trace: bool,
    stop: Option<String>,
    expected_finish_reason: Option<String>,
    probe_max_tokens: Option<usize>,
    required_token_lengths: Vec<usize>,
    generated_context_max_chars: Option<usize>,
    generated_context_max_tokens: Option<usize>,
    generated_context_state_capsule: Option<String>,
    generated_context_state_capsule_placement: LongChatStateCapsulePlacement,
    required_generated_response_substrings: Vec<String>,
    disconnect_reconnect_timeout: Duration,
    rss_pid: Option<u32>,
    proof_log_path: Option<PathBuf>,
    proof_exit_code_path: Option<PathBuf>,
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
                "--queue-probe" => {
                    config.queue_probe = true;
                }
                "--require-probes" => {
                    config.required_probes =
                        parse_required_probes(next_value(&mut iter, "--require-probes")?)?;
                }
                "--require-cached-follow-ups" => {
                    config.require_cached_follow_ups = true;
                }
                "--models" => {
                    config.models =
                        parse_non_empty_list(next_value(&mut iter, "--models")?, "--models")?;
                }
                "--require-models" => {
                    config.required_models = parse_non_empty_list(
                        next_value(&mut iter, "--require-models")?,
                        "--require-models",
                    )?;
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
                "--prompt-cache-keys" => {
                    config.prompt_cache_keys = parse_non_empty_list(
                        next_value(&mut iter, "--prompt-cache-keys")?,
                        "--prompt-cache-keys",
                    )?;
                }
                "--prompt-cache-trace" => {
                    config.prompt_cache_trace = true;
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
                "--require-token-lengths" => {
                    config.required_token_lengths =
                        parse_token_lengths(next_value(&mut iter, "--require-token-lengths")?)?;
                }
                "--generated-context-max-chars" => {
                    config.generated_context_max_chars = Some(parse_positive_usize(
                        &os_string_to_string(next_value(
                            &mut iter,
                            "--generated-context-max-chars",
                        )?)?,
                        "--generated-context-max-chars",
                    )?);
                }
                "--generated-context-max-tokens" => {
                    config.generated_context_max_tokens = Some(parse_positive_usize(
                        &os_string_to_string(next_value(
                            &mut iter,
                            "--generated-context-max-tokens",
                        )?)?,
                        "--generated-context-max-tokens",
                    )?);
                }
                "--generated-context-state-capsule" => {
                    config.generated_context_state_capsule = Some(parse_non_empty_string(
                        next_value(&mut iter, "--generated-context-state-capsule")?,
                        "--generated-context-state-capsule",
                    )?);
                }
                "--generated-context-state-capsule-placement" => {
                    let value = parse_non_empty_string(
                        next_value(&mut iter, "--generated-context-state-capsule-placement")?,
                        "--generated-context-state-capsule-placement",
                    )?;
                    config.generated_context_state_capsule_placement =
                        LongChatStateCapsulePlacement::parse(&value)?;
                }
                "--require-generated-response-contains" => {
                    config
                        .required_generated_response_substrings
                        .push(parse_non_empty_string(
                            next_value(&mut iter, "--require-generated-response-contains")?,
                            "--require-generated-response-contains",
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
                "--proof-log" => {
                    config.proof_log_path = Some(parse_non_empty_path(
                        next_value(&mut iter, "--proof-log")?,
                        "--proof-log",
                    )?);
                }
                "--proof-exit-code" => {
                    config.proof_exit_code_path = Some(parse_non_empty_path(
                        next_value(&mut iter, "--proof-exit-code")?,
                        "--proof-exit-code",
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

        if config.prompt_cache_key.is_some() && !config.prompt_cache_keys.is_empty() {
            return Err(LongChatGateError::new(
                "--prompt-cache-key cannot be combined with --prompt-cache-keys",
            ));
        }
        if config.require_cached_follow_ups
            && config.prompt_cache_key.is_none()
            && config.prompt_cache_keys.is_empty()
        {
            return Err(LongChatGateError::new(
                "--require-cached-follow-ups requires --prompt-cache-key or --prompt-cache-keys",
            ));
        }
        if config.queue_probe && config.prompt_cache_keys.len() < 2 {
            return Err(LongChatGateError::new(
                "--queue-probe requires at least two --prompt-cache-keys",
            ));
        }
        if config.generated_context_max_chars.is_some()
            && config.generated_context_max_tokens.is_some()
        {
            return Err(LongChatGateError::new(
                "--generated-context-max-chars cannot be combined with --generated-context-max-tokens",
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

    pub fn queue_probe(&self) -> bool {
        self.queue_probe
    }

    pub fn required_probes(&self) -> &[LongChatRequiredProbe] {
        &self.required_probes
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

    pub fn required_models(&self) -> &[String] {
        &self.required_models
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

    pub fn prompt_cache_keys(&self) -> &[String] {
        &self.prompt_cache_keys
    }

    pub fn prompt_cache_trace(&self) -> bool {
        self.prompt_cache_trace
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

    pub fn required_token_lengths(&self) -> &[usize] {
        &self.required_token_lengths
    }

    pub fn generated_context_max_chars(&self) -> Option<usize> {
        self.generated_context_max_chars
    }

    pub fn generated_context_max_tokens(&self) -> Option<usize> {
        self.generated_context_max_tokens
    }

    pub fn generated_context_state_capsule(&self) -> Option<&str> {
        self.generated_context_state_capsule.as_deref()
    }

    pub fn generated_context_state_capsule_placement(&self) -> LongChatStateCapsulePlacement {
        self.generated_context_state_capsule_placement
    }

    pub fn required_generated_response_substrings(&self) -> &[String] {
        &self.required_generated_response_substrings
    }

    pub fn disconnect_reconnect_timeout(&self) -> Duration {
        self.disconnect_reconnect_timeout
    }

    pub fn rss_pid(&self) -> Option<u32> {
        self.rss_pid
    }

    pub fn proof_log_path(&self) -> Option<&Path> {
        self.proof_log_path.as_deref()
    }

    pub fn proof_exit_code_path(&self) -> Option<&Path> {
        self.proof_exit_code_path.as_deref()
    }

    pub fn turns(&self) -> usize {
        self.turns
    }

    pub fn planned_scenarios(&self) -> usize {
        self.models.len() * self.token_lengths.len() * self.turns * self.prompt_cache_lane_count()
    }

    pub fn scenarios(&self) -> Vec<LongChatScenario<'_>> {
        let mut scenarios = Vec::with_capacity(self.planned_scenarios());
        for model in &self.models {
            if self.prompt_cache_keys.is_empty() {
                for turn in 1..=self.turns {
                    for token_length in &self.token_lengths {
                        scenarios.push(LongChatScenario::new(model, turn, *token_length));
                    }
                }
            } else {
                for prompt_cache_key in &self.prompt_cache_keys {
                    for turn in 1..=self.turns {
                        for token_length in &self.token_lengths {
                            scenarios.push(LongChatScenario::new_with_prompt_cache_key(
                                model,
                                turn,
                                *token_length,
                                Some(prompt_cache_key),
                            ));
                        }
                    }
                }
            }
        }
        scenarios
    }

    fn prompt_cache_lane_count(&self) -> usize {
        self.prompt_cache_keys.len().max(1)
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
            queue_probe: false,
            required_probes: Vec::new(),
            require_cached_follow_ups: false,
            models: vec![
                "Qwen2.5-0.5B-Instruct-Q4_K_M".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q8_0".to_owned(),
                "Qwen2.5-1.5B-Instruct-Q6_K".to_owned(),
                "SmolLM2-1.7B-Instruct-Q4_K_M".to_owned(),
            ],
            required_models: Vec::new(),
            prompt: "Write a concise paragraph about CPU inference.".to_owned(),
            assistant_context: "CPU inference prioritizes memory locality, predictable scheduling, and efficient token streaming.".to_owned(),
            follow_up: "Continue with the operational risks for a long streaming chat.".to_owned(),
            prompt_cache_key: None,
            prompt_cache_keys: Vec::new(),
            prompt_cache_trace: false,
            stop: None,
            expected_finish_reason: None,
            probe_max_tokens: None,
            required_token_lengths: Vec::new(),
            generated_context_max_chars: None,
            generated_context_max_tokens: None,
            generated_context_state_capsule: None,
            generated_context_state_capsule_placement: LongChatStateCapsulePlacement::default(),
            required_generated_response_substrings: Vec::new(),
            disconnect_reconnect_timeout: DEFAULT_DISCONNECT_RECONNECT_TIMEOUT,
            rss_pid: None,
            proof_log_path: None,
            proof_exit_code_path: None,
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
    pub(super) fn new(message: impl Into<String>) -> Self {
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

fn parse_non_empty_path(value: OsString, flag: &str) -> Result<PathBuf, LongChatGateError> {
    if value.as_os_str().is_empty() {
        return Err(LongChatGateError::new(format!(
            "{flag} must contain at least one value"
        )));
    }
    Ok(PathBuf::from(value))
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
    "usage: ferrite-openai-long-chat-gate [--execute] [--error-probe] [--disconnect-probe] [--queue-probe] [--require-probes error,disconnect,queue] [--require-cached-follow-ups] [--addr 127.0.0.1:8080] [--api-key local-secret] [--models MODEL[,MODEL...]] [--require-models MODEL[,MODEL...]] [--prompt TEXT] [--assistant-context TEXT] [--follow-up TEXT] [--prompt-cache-key KEY] [--prompt-cache-keys KEY[,KEY...]] [--prompt-cache-trace] [--stop TEXT] [--expect-finish-reason REASON] [--probe-max-tokens TOKENS] [--require-token-lengths 256,512,1024] [--generated-context-max-chars CHARS] [--generated-context-max-tokens TOKENS] [--generated-context-state-capsule TEXT] [--generated-context-state-capsule-placement assistant-context|assistant-context-only|follow-up] [--require-generated-response-contains TEXT] [--disconnect-reconnect-timeout-ms 30000] [--rss-pid PID] [--proof-log PATH] [--proof-exit-code PATH] [--token-lengths 256,512,1024] [--turns 4]"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongChatRequiredProbe {
    Error,
    Disconnect,
    Queue,
}

impl LongChatRequiredProbe {
    pub fn parse(value: &str) -> Result<Self, LongChatGateError> {
        match value {
            "error" => Ok(Self::Error),
            "disconnect" => Ok(Self::Disconnect),
            "queue" => Ok(Self::Queue),
            other => Err(LongChatGateError::new(format!(
                "invalid --require-probes entry {other:?}; expected error, disconnect, or queue"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Disconnect => "disconnect",
            Self::Queue => "queue",
        }
    }
}

fn parse_required_probes(value: OsString) -> Result<Vec<LongChatRequiredProbe>, LongChatGateError> {
    parse_non_empty_list(value, "--require-probes")?
        .into_iter()
        .map(|part| LongChatRequiredProbe::parse(&part))
        .collect()
}
