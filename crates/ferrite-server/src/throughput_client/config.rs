use std::{error::Error, ffi::OsString, fmt, net::SocketAddr, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThroughputClientConfig {
    addr: SocketAddr,
    endpoint: OpenAiEndpoint,
    model: String,
    prompts: Vec<String>,
    assistant_context: Option<String>,
    follow_up: Option<String>,
    prompt_cache_key: Option<String>,
    prompt_cache_trace: bool,
    stop: Option<String>,
    requests: usize,
    concurrency: usize,
    max_token_budgets: Vec<usize>,
    stream: bool,
    stream_usage: bool,
    rss_pid: Option<u32>,
    rss_idle_delay: Duration,
    api_key: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAiEndpoint {
    Completions,
    ChatCompletions,
}

impl OpenAiEndpoint {
    pub fn path(self) -> &'static str {
        match self {
            Self::Completions => "/v1/completions",
            Self::ChatCompletions => "/v1/chat/completions",
        }
    }

    pub fn metric_name(self, stream: bool) -> &'static str {
        match (self, stream) {
            (Self::Completions, false) => "openai_http_completion_requests",
            (Self::ChatCompletions, false) => "openai_http_chat_completion_requests",
            (Self::Completions, true) => "openai_http_streaming_completion_requests",
            (Self::ChatCompletions, true) => "openai_http_streaming_chat_completion_requests",
        }
    }
}

impl ThroughputClientConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ClientConfigError> {
        let mut config = Self::default();
        let mut iter = args.into_iter();
        let _program = iter.next();
        let mut saw_prompt = false;
        let mut saw_max_tokens = false;

        while let Some(arg) = iter.next() {
            let flag = arg
                .to_str()
                .ok_or_else(|| ClientConfigError::new("arguments must be valid UTF-8"))?;
            match flag {
                "--addr" => {
                    config.addr = os_string_to_string(next_value(&mut iter, "--addr")?)?
                        .parse()
                        .map_err(|error| {
                            ClientConfigError::new(format!("invalid --addr: {error}"))
                        })?;
                }
                "--model" => {
                    config.model = os_string_to_string(next_value(&mut iter, "--model")?)?;
                    if config.model.trim().is_empty() {
                        return Err(ClientConfigError::new("--model must not be empty"));
                    }
                }
                "--endpoint" => {
                    config.endpoint = parse_endpoint(next_value(&mut iter, "--endpoint")?)?;
                }
                "--prompt" => {
                    let prompt = os_string_to_string(next_value(&mut iter, "--prompt")?)?;
                    if !saw_prompt {
                        config.prompts.clear();
                        saw_prompt = true;
                    }
                    config.prompts.push(prompt);
                }
                "--assistant-context" => {
                    let assistant_context =
                        os_string_to_string(next_value(&mut iter, "--assistant-context")?)?;
                    if assistant_context.is_empty() {
                        return Err(ClientConfigError::new(
                            "--assistant-context must not be empty",
                        ));
                    }
                    config.assistant_context = Some(assistant_context);
                }
                "--follow-up" => {
                    let follow_up = os_string_to_string(next_value(&mut iter, "--follow-up")?)?;
                    if follow_up.is_empty() {
                        return Err(ClientConfigError::new("--follow-up must not be empty"));
                    }
                    config.follow_up = Some(follow_up);
                }
                "--prompt-cache-key" => {
                    let prompt_cache_key =
                        os_string_to_string(next_value(&mut iter, "--prompt-cache-key")?)?;
                    if prompt_cache_key.is_empty() {
                        return Err(ClientConfigError::new(
                            "--prompt-cache-key must not be empty",
                        ));
                    }
                    config.prompt_cache_key = Some(prompt_cache_key);
                }
                "--prompt-cache-trace" => {
                    config.prompt_cache_trace = true;
                }
                "--stop" => {
                    let stop = os_string_to_string(next_value(&mut iter, "--stop")?)?;
                    if stop.is_empty() {
                        return Err(ClientConfigError::new("--stop must not be empty"));
                    }
                    config.stop = Some(stop);
                }
                "--requests" => {
                    config.requests =
                        parse_positive_usize(next_value(&mut iter, "--requests")?, "--requests")?;
                }
                "--concurrency" => {
                    config.concurrency = parse_positive_usize(
                        next_value(&mut iter, "--concurrency")?,
                        "--concurrency",
                    )?;
                }
                "--max-tokens" => {
                    let max_tokens = parse_positive_usize(
                        next_value(&mut iter, "--max-tokens")?,
                        "--max-tokens",
                    )?;
                    if !saw_max_tokens {
                        config.max_token_budgets.clear();
                        saw_max_tokens = true;
                    }
                    config.max_token_budgets.push(max_tokens);
                }
                "--stream" => {
                    config.stream = true;
                }
                "--stream-usage" => {
                    config.stream_usage = true;
                }
                "--rss-pid" => {
                    config.rss_pid = Some(parse_positive_u32(
                        next_value(&mut iter, "--rss-pid")?,
                        "--rss-pid",
                    )?);
                }
                "--rss-idle-ms" => {
                    let milliseconds = parse_positive_u64(
                        next_value(&mut iter, "--rss-idle-ms")?,
                        "--rss-idle-ms",
                    )?;
                    config.rss_idle_delay = Duration::from_millis(milliseconds);
                }
                "--api-key" => {
                    config.api_key = os_string_to_string(next_value(&mut iter, "--api-key")?)?;
                    if config.api_key.trim().is_empty() {
                        return Err(ClientConfigError::new("--api-key must not be empty"));
                    }
                }
                "--help" | "-h" => return Err(ClientConfigError::new(usage())),
                other => {
                    return Err(ClientConfigError::new(format!(
                        "unknown argument {other}\n{}",
                        usage()
                    )));
                }
            }
        }

        if config.stream_usage && !config.stream {
            return Err(ClientConfigError::new("--stream-usage requires --stream"));
        }
        if config.assistant_context.is_some() != config.follow_up.is_some() {
            return Err(ClientConfigError::new(
                "--assistant-context and --follow-up must be provided together",
            ));
        }
        if config.assistant_context.is_some() && config.endpoint != OpenAiEndpoint::ChatCompletions
        {
            return Err(ClientConfigError::new(
                "--assistant-context and --follow-up require --endpoint chat-completions",
            ));
        }
        if config.prompt_cache_trace && config.endpoint != OpenAiEndpoint::ChatCompletions {
            return Err(ClientConfigError::new(
                "--prompt-cache-trace requires --endpoint chat-completions",
            ));
        }
        let configured_cases = config.prompts.len().max(config.max_token_budgets.len());
        if config.requests < configured_cases {
            return Err(ClientConfigError::new(format!(
                "--requests must be at least the number of configured prompt or token-budget values ({configured_cases})"
            )));
        }
        if config.prompts.len() > 1
            && config.max_token_budgets.len() > 1
            && config.prompts.len() != config.max_token_budgets.len()
        {
            return Err(ClientConfigError::new(
                "repeatable --prompt and --max-tokens values must have equal counts when both are repeated",
            ));
        }

        Ok(config)
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn endpoint(&self) -> OpenAiEndpoint {
        self.endpoint
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn prompt(&self) -> &str {
        &self.prompts[0]
    }

    pub fn prompts(&self) -> &[String] {
        &self.prompts
    }

    pub fn prompt_for_request(&self, request_index: usize) -> &str {
        &self.prompts[request_index % self.prompts.len()]
    }

    pub fn distinct_prompt_count(&self) -> usize {
        let mut prompts = self.prompts.iter().map(String::as_str).collect::<Vec<_>>();
        prompts.sort_unstable();
        prompts.dedup();
        prompts.len()
    }

    pub fn assistant_context(&self) -> Option<&str> {
        self.assistant_context.as_deref()
    }

    pub fn follow_up(&self) -> Option<&str> {
        self.follow_up.as_deref()
    }

    pub fn prompt_cache_key(&self) -> Option<&str> {
        self.prompt_cache_key.as_deref()
    }

    pub fn prompt_cache_trace(&self) -> bool {
        self.prompt_cache_trace
    }

    pub fn stop(&self) -> Option<&str> {
        self.stop.as_deref()
    }

    pub fn requests(&self) -> usize {
        self.requests
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    pub fn max_tokens(&self) -> usize {
        self.max_token_budgets[0]
    }

    pub fn max_token_budgets(&self) -> &[usize] {
        &self.max_token_budgets
    }

    pub fn max_tokens_for_request(&self, request_index: usize) -> usize {
        self.max_token_budgets[request_index % self.max_token_budgets.len()]
    }

    pub fn distinct_max_token_budget_count(&self) -> usize {
        let mut budgets = self.max_token_budgets.clone();
        budgets.sort_unstable();
        budgets.dedup();
        budgets.len()
    }

    pub fn stream(&self) -> bool {
        self.stream
    }

    pub fn stream_usage(&self) -> bool {
        self.stream_usage
    }

    pub fn rss_pid(&self) -> Option<u32> {
        self.rss_pid
    }

    pub fn rss_idle_delay(&self) -> Duration {
        self.rss_idle_delay
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl Default for ThroughputClientConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
            endpoint: OpenAiEndpoint::Completions,
            model: "ferrite-local".to_owned(),
            prompts: vec!["hello world".to_owned()],
            assistant_context: None,
            follow_up: None,
            prompt_cache_key: None,
            prompt_cache_trace: false,
            stop: None,
            requests: 3,
            concurrency: 1,
            max_token_budgets: vec![1],
            stream: false,
            stream_usage: false,
            rss_pid: None,
            rss_idle_delay: Duration::from_secs(2),
            api_key: "local-secret".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientConfigError {
    message: String,
}

impl ClientConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClientConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ClientConfigError {}

fn next_value(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<OsString, ClientConfigError> {
    iter.next()
        .ok_or_else(|| ClientConfigError::new(format!("missing value for {flag}")))
}

fn os_string_to_string(value: OsString) -> Result<String, ClientConfigError> {
    value
        .into_string()
        .map_err(|_error| ClientConfigError::new("arguments must be valid UTF-8"))
}

fn parse_positive_usize(value: OsString, flag: &str) -> Result<usize, ClientConfigError> {
    let parsed: usize = os_string_to_string(value)?
        .parse()
        .map_err(|error| ClientConfigError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(ClientConfigError::new(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(parsed)
}

fn parse_positive_u32(value: OsString, flag: &str) -> Result<u32, ClientConfigError> {
    let parsed: u32 = os_string_to_string(value)?
        .parse()
        .map_err(|error| ClientConfigError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(ClientConfigError::new(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(parsed)
}

fn parse_positive_u64(value: OsString, flag: &str) -> Result<u64, ClientConfigError> {
    let parsed: u64 = os_string_to_string(value)?
        .parse()
        .map_err(|error| ClientConfigError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(ClientConfigError::new(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(parsed)
}

fn parse_endpoint(value: OsString) -> Result<OpenAiEndpoint, ClientConfigError> {
    match os_string_to_string(value)?.as_str() {
        "completions" => Ok(OpenAiEndpoint::Completions),
        "chat-completions" => Ok(OpenAiEndpoint::ChatCompletions),
        other => Err(ClientConfigError::new(format!(
            "invalid --endpoint: {other}; expected completions or chat-completions"
        ))),
    }
}

/// Returns the command-line usage string for the throughput client.
pub fn usage() -> &'static str {
    "usage: ferrite-openai-throughput [--addr 127.0.0.1:8080] [--endpoint completions|chat-completions] [--model ferrite-local] [--prompt 'hello world']... [--assistant-context TEXT --follow-up TEXT] [--prompt-cache-key KEY] [--prompt-cache-trace] [--stop STOP] [--requests 3] [--concurrency 1] [--max-tokens 1]... [--stream] [--stream-usage] [--rss-pid PID] [--rss-idle-ms 2000] [--api-key local-secret]"
}
