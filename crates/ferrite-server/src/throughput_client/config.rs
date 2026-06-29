use std::{error::Error, ffi::OsString, fmt, net::SocketAddr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThroughputClientConfig {
    addr: SocketAddr,
    model: String,
    prompt: String,
    requests: usize,
    concurrency: usize,
    max_tokens: usize,
    api_key: String,
}

impl ThroughputClientConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ClientConfigError> {
        let mut config = Self::default();
        let mut iter = args.into_iter();
        let _program = iter.next();

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
                "--prompt" => {
                    config.prompt = os_string_to_string(next_value(&mut iter, "--prompt")?)?;
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
                    config.max_tokens = parse_positive_usize(
                        next_value(&mut iter, "--max-tokens")?,
                        "--max-tokens",
                    )?;
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

        Ok(config)
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn requests(&self) -> usize {
        self.requests
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl Default for ThroughputClientConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
            model: "ferrite-local".to_owned(),
            prompt: "hello world".to_owned(),
            requests: 3,
            concurrency: 1,
            max_tokens: 1,
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
        .map_err(|_| ClientConfigError::new("arguments must be valid UTF-8"))
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

fn usage() -> &'static str {
    "usage: ferrite-openai-throughput [--addr 127.0.0.1:8080] [--model ferrite-local] [--prompt 'hello world'] [--requests 3] [--concurrency 1] [--max-tokens 1] [--api-key local-secret]"
}
