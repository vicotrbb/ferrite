use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestTokenLimit {
    value: Option<usize>,
    malformed: bool,
}

impl RequestTokenLimit {
    pub fn value(&self) -> Option<usize> {
        self.value
    }

    pub fn is_malformed(&self) -> bool {
        self.malformed
    }
}

impl<'de> Deserialize<'de> for RequestTokenLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_value(Value::deserialize(deserializer)?))
    }
}

impl RequestTokenLimit {
    fn from_value(value: Value) -> Self {
        match value {
            Value::Null => Self::default(),
            Value::Number(number) => {
                match number.as_u64().and_then(|value| value.try_into().ok()) {
                    Some(value) => Self {
                        value: Some(value),
                        malformed: false,
                    },
                    None => Self {
                        value: None,
                        malformed: true,
                    },
                }
            }
            _ => Self {
                value: None,
                malformed: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_integer_token_limit() -> Result<(), Box<dyn std::error::Error>> {
        let limit: RequestTokenLimit = serde_json::from_str("16")?;

        assert_eq!(limit.value(), Some(16));
        assert!(!limit.is_malformed());
        Ok(())
    }

    #[test]
    fn records_string_token_limit_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let limit: RequestTokenLimit = serde_json::from_str(r#""16""#)?;

        assert_eq!(limit.value(), None);
        assert!(limit.is_malformed());
        Ok(())
    }

    #[test]
    fn records_negative_token_limit_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let limit: RequestTokenLimit = serde_json::from_str("-1")?;

        assert_eq!(limit.value(), None);
        assert!(limit.is_malformed());
        Ok(())
    }

    #[test]
    fn treats_null_token_limit_as_missing() -> Result<(), Box<dyn std::error::Error>> {
        let limit: RequestTokenLimit = serde_json::from_str("null")?;

        assert_eq!(limit.value(), None);
        assert!(!limit.is_malformed());
        Ok(())
    }
}
