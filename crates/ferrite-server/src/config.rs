use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    bind_addr: SocketAddr,
    model_id: String,
    model_path: Option<PathBuf>,
}

impl ServerConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
            model_id: "ferrite-local".to_owned(),
            model_path: None,
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

fn usage() -> &'static str {
    "usage: ferrite-server [--bind 127.0.0.1:8080] [--model-id ferrite-local] [--model path/to/model.gguf]"
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
}
