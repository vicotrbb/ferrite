use crate::limits::TokenLimits;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    bind_addr: SocketAddr,
    model_id: String,
    model_path: Option<PathBuf>,
    api_key: Option<String>,
    token_limits: TokenLimits,
    inference_wait_timeout: Duration,
}

impl ServerConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut default_max_tokens = config.token_limits.default_max_tokens();
        let mut hard_max_tokens = config.token_limits.hard_max_tokens();
        let mut iter = args.into_iter();
        let _program = iter.next();

        while let Some(arg) = iter.next() {
            let flag = arg
                .to_str()
                .ok_or_else(|| ConfigError::new("arguments must be valid UTF-8"))?;
            match flag {
                "--bind" => {
                    let value = next_value(&mut iter, "--bind")?;
                    config.bind_addr = os_string_to_string(value)?
                        .parse()
                        .map_err(|error| ConfigError::new(format!("invalid --bind: {error}")))?;
                }
                "--model-id" => {
                    config.model_id = os_string_to_string(next_value(&mut iter, "--model-id")?)?;
                    if config.model_id.trim().is_empty() {
                        return Err(ConfigError::new("--model-id must not be empty"));
                    }
                }
                "--model" => {
                    config.model_path = Some(PathBuf::from(next_value(&mut iter, "--model")?));
                }
                "--api-key" => {
                    let api_key = os_string_to_string(next_value(&mut iter, "--api-key")?)?;
                    if api_key.trim().is_empty() {
                        return Err(ConfigError::new("--api-key must not be empty"));
                    }
                    config.api_key = Some(api_key);
                }
                "--default-max-tokens" => {
                    let value = next_value(&mut iter, "--default-max-tokens")?;
                    default_max_tokens = parse_token_limit(value, "--default-max-tokens")?;
                }
                "--hard-max-tokens" => {
                    let value = next_value(&mut iter, "--hard-max-tokens")?;
                    hard_max_tokens = parse_token_limit(value, "--hard-max-tokens")?;
                }
                "--inference-wait-ms" => {
                    let value = next_value(&mut iter, "--inference-wait-ms")?;
                    config.inference_wait_timeout =
                        Duration::from_millis(parse_millis(value, "--inference-wait-ms")?);
                }
                "--help" | "-h" => {
                    return Err(ConfigError::new(usage()));
                }
                other => {
                    return Err(ConfigError::new(format!(
                        "unknown argument {other}\n{}",
                        usage()
                    )));
                }
            }
        }

        config.token_limits = TokenLimits::new(default_max_tokens, hard_max_tokens)
            .map_err(|error| ConfigError::new(error.to_string()))?;
        Ok(config)
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_path(&self) -> Option<&Path> {
        self.model_path.as_deref()
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn token_limits(&self) -> TokenLimits {
        self.token_limits
    }

    pub fn inference_wait_timeout(&self) -> Duration {
        self.inference_wait_timeout
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
            model_id: "ferrite-local".to_owned(),
            model_path: None,
            api_key: None,
            token_limits: TokenLimits::default(),
            inference_wait_timeout: Duration::ZERO,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ConfigError {}

fn next_value(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<OsString, ConfigError> {
    iter.next()
        .ok_or_else(|| ConfigError::new(format!("missing value for {flag}")))
}

fn os_string_to_string(value: OsString) -> Result<String, ConfigError> {
    value
        .into_string()
        .map_err(|_| ConfigError::new("arguments must be valid UTF-8"))
}

fn parse_token_limit(value: OsString, flag: &str) -> Result<usize, ConfigError> {
    os_string_to_string(value)?
        .parse()
        .map_err(|error| ConfigError::new(format!("invalid {flag}: {error}")))
}

fn parse_millis(value: OsString, flag: &str) -> Result<u64, ConfigError> {
    os_string_to_string(value)?
        .parse()
        .map_err(|error| ConfigError::new(format!("invalid {flag}: {error}")))
}

fn usage() -> &'static str {
    "usage: ferrite-server [--bind 127.0.0.1:8080] [--model-id ferrite-local] [--model path/to/model.gguf] [--api-key local-secret] [--default-max-tokens 16] [--hard-max-tokens 256] [--inference-wait-ms 0]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bind_addr_and_model_id() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--bind"),
            OsString::from("127.0.0.1:18181"),
            OsString::from("--model-id"),
            OsString::from("smollm2"),
        ])?;

        assert_eq!(
            config.bind_addr(),
            SocketAddr::from(([127, 0, 0, 1], 18181))
        );
        assert_eq!(config.model_id(), "smollm2");
        Ok(())
    }

    #[test]
    fn parses_optional_api_key() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--api-key"),
            OsString::from("local-secret"),
        ])?;

        assert_eq!(config.api_key(), Some("local-secret"));
        Ok(())
    }

    #[test]
    fn rejects_empty_api_key() -> Result<(), Box<dyn Error>> {
        let result = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--api-key"),
            OsString::from("  "),
        ]);
        let error = match result {
            Ok(_) => return Err("empty api key should be rejected".into()),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "--api-key must not be empty");
        Ok(())
    }

    #[test]
    fn parses_token_limits() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--default-max-tokens"),
            OsString::from("4"),
            OsString::from("--hard-max-tokens"),
            OsString::from("8"),
        ])?;

        assert_eq!(config.token_limits().default_max_tokens(), 4);
        assert_eq!(config.token_limits().hard_max_tokens(), 8);
        Ok(())
    }

    #[test]
    fn parses_inference_wait_timeout() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--inference-wait-ms"),
            OsString::from("250"),
        ])?;

        assert_eq!(
            config.inference_wait_timeout(),
            std::time::Duration::from_millis(250)
        );
        Ok(())
    }

    #[test]
    fn rejects_default_limit_above_hard_limit() -> Result<(), Box<dyn Error>> {
        let result = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--default-max-tokens"),
            OsString::from("9"),
            OsString::from("--hard-max-tokens"),
            OsString::from("8"),
        ]);
        let error = match result {
            Ok(_) => return Err("default token limit above hard limit should be rejected".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "default max tokens must be less than or equal to hard max tokens"
        );
        Ok(())
    }

    #[test]
    fn parses_token_limits_in_any_order() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--default-max-tokens"),
            OsString::from("512"),
            OsString::from("--hard-max-tokens"),
            OsString::from("1024"),
        ])?;

        assert_eq!(config.token_limits().default_max_tokens(), 512);
        assert_eq!(config.token_limits().hard_max_tokens(), 1024);
        Ok(())
    }
}
