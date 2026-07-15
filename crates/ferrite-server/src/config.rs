use crate::limits::TokenLimits;
use ferrite_inference::scalar::KernelProvider;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_PREFIX_CACHE_MAX_ENTRIES: usize = 8;
const DEFAULT_PREFIX_CACHE_MAX_MIB: usize = 64;
const DEFAULT_KV_TOKENS_PER_BLOCK: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ServerKvBackend {
    #[default]
    Vec,
    Locus,
}

impl ServerKvBackend {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "vec" => Ok(Self::Vec),
            "locus" => Ok(Self::Locus),
            other => Err(ConfigError::new(format!(
                "invalid --kv-backend: must be one of vec, locus (got {other})"
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    bind_addr: SocketAddr,
    model_id: String,
    model_path: Option<PathBuf>,
    api_key: Option<String>,
    token_limits: TokenLimits,
    inference_wait_timeout: Duration,
    experimental_prefix_cache_enabled: bool,
    prefix_cache_max_entries: usize,
    prefix_cache_max_mib: usize,
    experimental_residual_q8_activation_matvec: bool,
    experimental_batched_decode_enabled: bool,
    max_batch_streams: Option<usize>,
    max_batch_queue: Option<usize>,
    kernel_provider: KernelProvider,
    inference_threads: Option<usize>,
    max_concurrent_inferences: usize,
    kv_backend: ServerKvBackend,
    kv_tokens_per_block: usize,
    kv_max_tokens: Option<usize>,
}

impl ServerConfig {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut default_max_tokens = config.token_limits.default_max_tokens();
        let mut hard_max_tokens = config.token_limits.hard_max_tokens();
        let mut prefix_cache_limits_configured = false;
        let mut kv_sizing_configured = false;
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
                "--experimental-prefix-cache" => {
                    config.experimental_prefix_cache_enabled = true;
                }
                "--prefix-cache-max-entries" => {
                    config.prefix_cache_max_entries = parse_positive_usize(
                        next_value(&mut iter, "--prefix-cache-max-entries")?,
                        "--prefix-cache-max-entries",
                    )?;
                    prefix_cache_limits_configured = true;
                }
                "--prefix-cache-max-mib" => {
                    config.prefix_cache_max_mib = parse_positive_usize(
                        next_value(&mut iter, "--prefix-cache-max-mib")?,
                        "--prefix-cache-max-mib",
                    )?;
                    prefix_cache_limits_configured = true;
                }
                "--experimental-residual-q8-activation-matvec" => {
                    config.experimental_residual_q8_activation_matvec = true;
                }
                "--experimental-batched-decode" => {
                    config.experimental_batched_decode_enabled = true;
                }
                "--max-batch-streams" => {
                    let value = os_string_to_string(next_value(&mut iter, "--max-batch-streams")?)?;
                    let streams = value.parse::<usize>().map_err(|error| {
                        ConfigError::new(format!("invalid --max-batch-streams: {error}"))
                    })?;
                    if streams == 0 {
                        return Err(ConfigError::new(
                            "--max-batch-streams must be greater than zero",
                        ));
                    }
                    config.max_batch_streams = Some(streams);
                }
                "--max-batch-queue" => {
                    config.max_batch_queue = Some(parse_positive_usize(
                        next_value(&mut iter, "--max-batch-queue")?,
                        "--max-batch-queue",
                    )?);
                }
                "--threads" => {
                    let value = os_string_to_string(next_value(&mut iter, "--threads")?)?;
                    let threads = value
                        .parse::<usize>()
                        .map_err(|error| ConfigError::new(format!("invalid --threads: {error}")))?;
                    if threads == 0 {
                        return Err(ConfigError::new("--threads must be greater than zero"));
                    }
                    config.inference_threads = Some(threads);
                }
                "--kernel-provider" => {
                    let value = os_string_to_string(next_value(&mut iter, "--kernel-provider")?)?;
                    config.kernel_provider = KernelProvider::parse(&value).map_err(|error| {
                        ConfigError::new(format!("invalid --kernel-provider: {error}"))
                    })?;
                }
                "--max-concurrent-inferences" => {
                    let value =
                        os_string_to_string(next_value(&mut iter, "--max-concurrent-inferences")?)?;
                    let permits = value.parse::<usize>().map_err(|error| {
                        ConfigError::new(format!("invalid --max-concurrent-inferences: {error}"))
                    })?;
                    if permits == 0 {
                        return Err(ConfigError::new(
                            "--max-concurrent-inferences must be greater than zero",
                        ));
                    }
                    config.max_concurrent_inferences = permits;
                }
                "--kv-backend" => {
                    let value = os_string_to_string(next_value(&mut iter, "--kv-backend")?)?;
                    config.kv_backend = ServerKvBackend::parse(&value)?;
                }
                "--kv-tokens-per-block" => {
                    config.kv_tokens_per_block = parse_positive_usize(
                        next_value(&mut iter, "--kv-tokens-per-block")?,
                        "--kv-tokens-per-block",
                    )?;
                    kv_sizing_configured = true;
                }
                "--kv-max-tokens" => {
                    config.kv_max_tokens = Some(parse_positive_usize(
                        next_value(&mut iter, "--kv-max-tokens")?,
                        "--kv-max-tokens",
                    )?);
                    kv_sizing_configured = true;
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
        match (
            config.experimental_batched_decode_enabled,
            config.max_batch_streams,
        ) {
            (true, None) => {
                return Err(ConfigError::new(
                    "--experimental-batched-decode requires --max-batch-streams N",
                ));
            }
            (false, Some(_)) => {
                return Err(ConfigError::new(
                    "--max-batch-streams requires --experimental-batched-decode",
                ));
            }
            _ => {}
        }
        if config.max_batch_queue.is_some() && !config.experimental_batched_decode_enabled {
            return Err(ConfigError::new(
                "--max-batch-queue requires --experimental-batched-decode",
            ));
        }
        if config.experimental_residual_q8_activation_matvec
            && config.experimental_batched_decode_enabled
        {
            return Err(ConfigError::new(
                "--experimental-residual-q8-activation-matvec cannot be combined with --experimental-batched-decode",
            ));
        }
        if prefix_cache_limits_configured && !config.experimental_prefix_cache_enabled {
            return Err(ConfigError::new(
                "prefix cache limits require --experimental-prefix-cache",
            ));
        }
        match (config.kv_backend, config.kv_max_tokens) {
            (ServerKvBackend::Locus, None) => {
                return Err(ConfigError::new(
                    "--kv-backend locus requires --kv-max-tokens N",
                ));
            }
            (ServerKvBackend::Vec, Some(_)) => {
                return Err(ConfigError::new(
                    "--kv-max-tokens requires --kv-backend locus",
                ));
            }
            _ => {}
        }
        if kv_sizing_configured && config.kv_backend == ServerKvBackend::Vec {
            return Err(ConfigError::new(
                "KV block sizing requires --kv-backend locus",
            ));
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

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn token_limits(&self) -> TokenLimits {
        self.token_limits
    }

    pub fn inference_wait_timeout(&self) -> Duration {
        self.inference_wait_timeout
    }

    pub fn experimental_prefix_cache_enabled(&self) -> bool {
        self.experimental_prefix_cache_enabled
    }

    pub fn prefix_cache_max_entries(&self) -> usize {
        self.prefix_cache_max_entries
    }

    pub fn prefix_cache_max_bytes(&self) -> u128 {
        self.prefix_cache_max_mib as u128 * 1024 * 1024
    }

    pub fn experimental_residual_q8_activation_matvec(&self) -> bool {
        self.experimental_residual_q8_activation_matvec
    }

    pub fn experimental_batched_decode_max_streams(&self) -> Option<usize> {
        self.experimental_batched_decode_enabled
            .then_some(self.max_batch_streams)
            .flatten()
    }

    pub fn experimental_batched_decode_max_queue(&self) -> Option<usize> {
        self.experimental_batched_decode_max_streams()
            .map(|streams| self.max_batch_queue.unwrap_or(streams))
    }

    pub fn inference_threads(&self) -> Option<usize> {
        self.inference_threads
    }

    pub fn kernel_provider(&self) -> KernelProvider {
        self.kernel_provider
    }

    pub fn max_concurrent_inferences(&self) -> usize {
        self.max_concurrent_inferences
    }

    pub fn kv_backend(&self) -> ServerKvBackend {
        self.kv_backend
    }

    pub fn kv_tokens_per_block(&self) -> usize {
        self.kv_tokens_per_block
    }

    pub fn kv_max_tokens(&self) -> Option<usize> {
        self.kv_max_tokens
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
            experimental_prefix_cache_enabled: false,
            prefix_cache_max_entries: DEFAULT_PREFIX_CACHE_MAX_ENTRIES,
            prefix_cache_max_mib: DEFAULT_PREFIX_CACHE_MAX_MIB,
            experimental_residual_q8_activation_matvec: false,
            experimental_batched_decode_enabled: false,
            max_batch_streams: None,
            max_batch_queue: None,
            kernel_provider: KernelProvider::Auto,
            inference_threads: None,
            max_concurrent_inferences: 1,
            kv_backend: ServerKvBackend::Vec,
            kv_tokens_per_block: DEFAULT_KV_TOKENS_PER_BLOCK,
            kv_max_tokens: None,
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
        .map_err(|_error| ConfigError::new("arguments must be valid UTF-8"))
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

fn parse_positive_usize(value: OsString, flag: &str) -> Result<usize, ConfigError> {
    let parsed = os_string_to_string(value)?
        .parse::<usize>()
        .map_err(|error| ConfigError::new(format!("invalid {flag}: {error}")))?;
    if parsed == 0 {
        return Err(ConfigError::new(format!(
            "{flag} must be greater than zero"
        )));
    }
    Ok(parsed)
}

pub fn usage() -> &'static str {
    "usage: ferrite-server [--bind 127.0.0.1:8080] [--model-id ferrite-local] [--model path/to/model.gguf] [--api-key local-secret] [--default-max-tokens 16] [--hard-max-tokens 256] [--inference-wait-ms 0] [--experimental-prefix-cache [--prefix-cache-max-entries 8] [--prefix-cache-max-mib 64]] [--experimental-residual-q8-activation-matvec] [--experimental-batched-decode --max-batch-streams N [--max-batch-queue N]] [--threads N] [--kernel-provider <auto|portable>] [--max-concurrent-inferences 1] [--kv-backend <vec|locus> [--kv-tokens-per-block 16] --kv-max-tokens N]"
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
        assert_eq!(config.kernel_provider(), KernelProvider::Auto);
        Ok(())
    }

    #[test]
    fn parses_and_validates_kernel_provider() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--kernel-provider"),
            OsString::from("portable"),
        ])?;
        assert_eq!(config.kernel_provider(), KernelProvider::Portable);

        let result = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--kernel-provider"),
            OsString::from("unsafe-native"),
        ]);
        let error = match result {
            Ok(_) => return Err("unknown kernel provider must fail".into()),
            Err(error) => error,
        };
        assert!(error.to_string().contains("auto, portable"));
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
    fn parses_experimental_prefix_cache_flag() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-prefix-cache"),
        ])?;

        assert!(config.experimental_prefix_cache_enabled());
        assert_eq!(config.prefix_cache_max_entries(), 8);
        assert_eq!(config.prefix_cache_max_bytes(), 64 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn parses_bounded_prefix_cache_limits() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-prefix-cache"),
            OsString::from("--prefix-cache-max-entries"),
            OsString::from("3"),
            OsString::from("--prefix-cache-max-mib"),
            OsString::from("12"),
        ])?;

        assert_eq!(config.prefix_cache_max_entries(), 3);
        assert_eq!(config.prefix_cache_max_bytes(), 12 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn prefix_cache_limits_require_opt_in_and_positive_values() -> Result<(), Box<dyn Error>> {
        let missing_opt_in = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--prefix-cache-max-entries"),
            OsString::from("3"),
        ]) {
            Ok(_) => return Err("cache limits without the cache opt-in should fail".into()),
            Err(error) => error,
        };
        assert_eq!(
            missing_opt_in.to_string(),
            "prefix cache limits require --experimental-prefix-cache"
        );

        let zero = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-prefix-cache"),
            OsString::from("--prefix-cache-max-mib"),
            OsString::from("0"),
        ]) {
            Ok(_) => return Err("a zero cache budget should fail".into()),
            Err(error) => error,
        };
        assert_eq!(
            zero.to_string(),
            "--prefix-cache-max-mib must be greater than zero"
        );
        Ok(())
    }

    #[test]
    fn parses_experimental_activation_matvec_flag() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-residual-q8-activation-matvec"),
        ])?;

        assert!(config.experimental_residual_q8_activation_matvec());
        Ok(())
    }

    #[test]
    fn parses_experimental_batched_decode_flags() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-batched-decode"),
            OsString::from("--max-batch-streams"),
            OsString::from("8"),
        ])?;

        assert_eq!(config.experimental_batched_decode_max_streams(), Some(8));
        assert_eq!(config.experimental_batched_decode_max_queue(), Some(8));
        Ok(())
    }

    #[test]
    fn parses_explicit_batched_decode_queue_limit() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-batched-decode"),
            OsString::from("--max-batch-streams"),
            OsString::from("4"),
            OsString::from("--max-batch-queue"),
            OsString::from("2"),
        ])?;

        assert_eq!(config.experimental_batched_decode_max_streams(), Some(4));
        assert_eq!(config.experimental_batched_decode_max_queue(), Some(2));
        Ok(())
    }

    #[test]
    fn parses_bounded_locus_kv_backend() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--kv-backend"),
            OsString::from("locus"),
            OsString::from("--kv-tokens-per-block"),
            OsString::from("8"),
            OsString::from("--kv-max-tokens"),
            OsString::from("4096"),
        ])?;

        assert_eq!(config.kv_backend(), ServerKvBackend::Locus);
        assert_eq!(config.kv_tokens_per_block(), 8);
        assert_eq!(config.kv_max_tokens(), Some(4096));
        Ok(())
    }

    #[test]
    fn locus_kv_backend_requires_explicit_capacity() -> Result<(), Box<dyn Error>> {
        let error = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--kv-backend"),
            OsString::from("locus"),
        ]) {
            Ok(_) => return Err("locus without a capacity should fail".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "--kv-backend locus requires --kv-max-tokens N"
        );
        Ok(())
    }

    #[test]
    fn vec_kv_backend_rejects_block_sizing() -> Result<(), Box<dyn Error>> {
        let error = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--kv-tokens-per-block"),
            OsString::from("8"),
        ]) {
            Ok(_) => return Err("vec with block sizing should fail".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "KV block sizing requires --kv-backend locus"
        );
        Ok(())
    }

    #[test]
    fn batched_decode_requires_both_opt_in_flags() -> Result<(), Box<dyn Error>> {
        let missing_limit = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-batched-decode"),
        ]) {
            Ok(_) => return Err("missing batch limit should be rejected".into()),
            Err(error) => error,
        };
        assert_eq!(
            missing_limit.to_string(),
            "--experimental-batched-decode requires --max-batch-streams N"
        );

        let missing_opt_in = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--max-batch-streams"),
            OsString::from("8"),
        ]) {
            Ok(_) => return Err("missing experimental opt-in should be rejected".into()),
            Err(error) => error,
        };
        assert_eq!(
            missing_opt_in.to_string(),
            "--max-batch-streams requires --experimental-batched-decode"
        );

        let queue_without_opt_in = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--max-batch-queue"),
            OsString::from("2"),
        ]) {
            Ok(_) => return Err("batch queue without opt-in should be rejected".into()),
            Err(error) => error,
        };
        assert_eq!(
            queue_without_opt_in.to_string(),
            "--max-batch-queue requires --experimental-batched-decode"
        );
        Ok(())
    }

    #[test]
    fn activation_matvec_rejects_batched_decode_combination() -> Result<(), Box<dyn Error>> {
        let error = match ServerConfig::parse([
            OsString::from("ferrite-server"),
            OsString::from("--experimental-residual-q8-activation-matvec"),
            OsString::from("--experimental-batched-decode"),
            OsString::from("--max-batch-streams"),
            OsString::from("4"),
        ]) {
            Ok(_) => return Err("incompatible experimental modes should be rejected".into()),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            "--experimental-residual-q8-activation-matvec cannot be combined with --experimental-batched-decode"
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
