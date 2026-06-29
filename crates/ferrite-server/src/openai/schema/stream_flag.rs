use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamFlag {
    value: bool,
    malformed: bool,
}

impl StreamFlag {
    pub fn value(&self) -> bool {
        self.value
    }

    pub fn is_malformed(&self) -> bool {
        self.malformed
    }
}

impl<'de> Deserialize<'de> for StreamFlag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_value(Value::deserialize(deserializer)?))
    }
}

impl StreamFlag {
    fn from_value(value: Value) -> Self {
        match value {
            Value::Bool(value) => Self {
                value,
                malformed: false,
            },
            Value::Null => Self::default(),
            _ => Self {
                value: false,
                malformed: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_true_stream_flag() -> Result<(), Box<dyn std::error::Error>> {
        let stream: StreamFlag = serde_json::from_str("true")?;

        assert!(stream.value());
        assert!(!stream.is_malformed());
        Ok(())
    }

    #[test]
    fn treats_null_stream_flag_as_missing() -> Result<(), Box<dyn std::error::Error>> {
        let stream: StreamFlag = serde_json::from_str("null")?;

        assert!(!stream.value());
        assert!(!stream.is_malformed());
        Ok(())
    }

    #[test]
    fn records_string_stream_flag_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let stream: StreamFlag = serde_json::from_str(r#""yes""#)?;

        assert!(!stream.value());
        assert!(stream.is_malformed());
        Ok(())
    }
}
