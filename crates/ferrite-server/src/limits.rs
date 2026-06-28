use std::error::Error;
use std::fmt;

pub const DEFAULT_MAX_TOKENS: usize = 16;
pub const HARD_MAX_TOKENS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenLimits {
    default_max_tokens: usize,
    hard_max_tokens: usize,
}

impl TokenLimits {
    pub fn new(default_max_tokens: usize, hard_max_tokens: usize) -> Result<Self, TokenLimitError> {
        if default_max_tokens == 0 {
            return Err(TokenLimitError::DefaultMustBePositive);
        }
        if hard_max_tokens == 0 {
            return Err(TokenLimitError::HardMustBePositive);
        }
        if default_max_tokens > hard_max_tokens {
            return Err(TokenLimitError::DefaultAboveHard);
        }
        Ok(Self {
            default_max_tokens,
            hard_max_tokens,
        })
    }

    pub fn default_max_tokens(&self) -> usize {
        self.default_max_tokens
    }

    pub fn hard_max_tokens(&self) -> usize {
        self.hard_max_tokens
    }

    pub fn normalize(&self, requested: Option<usize>) -> Result<usize, TokenLimitError> {
        let tokens = requested.unwrap_or(self.default_max_tokens);
        if tokens == 0 {
            return Err(TokenLimitError::RequestedMustBePositive);
        }
        if tokens > self.hard_max_tokens {
            return Err(TokenLimitError::RequestedAboveHard {
                hard_max_tokens: self.hard_max_tokens,
            });
        }
        Ok(tokens)
    }
}

impl Default for TokenLimits {
    fn default() -> Self {
        Self {
            default_max_tokens: DEFAULT_MAX_TOKENS,
            hard_max_tokens: HARD_MAX_TOKENS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenLimitError {
    DefaultMustBePositive,
    HardMustBePositive,
    DefaultAboveHard,
    RequestedMustBePositive,
    RequestedAboveHard { hard_max_tokens: usize },
}

impl fmt::Display for TokenLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultMustBePositive => {
                formatter.write_str("default max tokens must be greater than zero")
            }
            Self::HardMustBePositive => {
                formatter.write_str("hard max tokens must be greater than zero")
            }
            Self::DefaultAboveHard => formatter
                .write_str("default max tokens must be less than or equal to hard max tokens"),
            Self::RequestedMustBePositive => {
                formatter.write_str("max_tokens must be greater than zero")
            }
            Self::RequestedAboveHard { hard_max_tokens } => write!(
                formatter,
                "max_tokens must be less than or equal to {hard_max_tokens}"
            ),
        }
    }
}

impl Error for TokenLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_match_previous_route_defaults() {
        let limits = TokenLimits::default();

        assert_eq!(limits.default_max_tokens(), DEFAULT_MAX_TOKENS);
        assert_eq!(limits.hard_max_tokens(), HARD_MAX_TOKENS);
    }

    #[test]
    fn normalizes_missing_request_to_default() -> Result<(), Box<dyn Error>> {
        let limits = TokenLimits::new(4, 8)?;

        assert_eq!(limits.normalize(None)?, 4);
        Ok(())
    }

    #[test]
    fn rejects_requested_limit_above_hard_limit() -> Result<(), Box<dyn Error>> {
        let limits = TokenLimits::new(4, 8)?;

        let error = match limits.normalize(Some(9)) {
            Ok(_) => return Err("requested token limit above hard limit should be rejected".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "max_tokens must be less than or equal to 8"
        );
        Ok(())
    }
}
