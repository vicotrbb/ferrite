use super::unsupported::UnsupportedFields;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamOptions {
    include_usage: Option<bool>,
    malformed_include_usage: bool,
    include_obfuscation: Option<bool>,
    malformed_include_obfuscation: bool,
    malformed_options: bool,
    extra_fields: BTreeMap<String, Value>,
}

impl StreamOptions {
    pub fn include_usage(&self) -> bool {
        self.include_usage.unwrap_or(false)
    }

    pub fn unsupported_request_fields(&self) -> Vec<String> {
        UnsupportedFields::new()
            .with_present("stream_options", self.malformed_options)
            .with_present("include_usage", self.malformed_include_usage)
            .with_present(
                "include_obfuscation",
                self.malformed_include_obfuscation || self.include_obfuscation.unwrap_or(false),
            )
            .with_extra_keys(&self.extra_fields)
            .into_vec()
            .into_iter()
            .map(|field| {
                if field == "stream_options" {
                    field
                } else {
                    format!("stream_options.{field}")
                }
            })
            .collect()
    }
}

impl<'de> Deserialize<'de> for StreamOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_value(Value::deserialize(deserializer)?))
    }
}

impl StreamOptions {
    fn from_value(value: Value) -> Self {
        let Value::Object(options) = value else {
            return Self {
                include_usage: None,
                malformed_include_usage: false,
                include_obfuscation: None,
                malformed_include_obfuscation: false,
                malformed_options: true,
                extra_fields: BTreeMap::new(),
            };
        };

        let mut options = options.into_iter().collect::<BTreeMap<_, _>>();
        let (include_usage, malformed_include_usage) =
            take_optional_bool(&mut options, "include_usage");
        let (include_obfuscation, malformed_include_obfuscation) =
            take_optional_bool(&mut options, "include_obfuscation");

        Self {
            include_usage,
            malformed_include_usage,
            include_obfuscation,
            malformed_include_obfuscation,
            malformed_options: false,
            extra_fields: options,
        }
    }
}

fn take_optional_bool(options: &mut BTreeMap<String, Value>, key: &str) -> (Option<bool>, bool) {
    match options.remove(key) {
        Some(Value::Bool(value)) => (Some(value), false),
        Some(Value::Null) | None => (None, false),
        Some(_) => (None, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_usage_stream_option() -> Result<(), Box<dyn std::error::Error>> {
        let options: StreamOptions = serde_json::from_str(r#"{"include_usage":true}"#)?;

        assert!(options.include_usage());
        assert!(options.unsupported_request_fields().is_empty());
        Ok(())
    }

    #[test]
    fn records_malformed_include_usage_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options: StreamOptions = serde_json::from_str(r#"{"include_usage":"yes"}"#)?;

        assert!(!options.include_usage());
        assert_eq!(
            options.unsupported_request_fields(),
            ["stream_options.include_usage"]
        );
        Ok(())
    }

    #[test]
    fn records_non_object_options_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let options: StreamOptions = serde_json::from_str("true")?;

        assert_eq!(options.unsupported_request_fields(), ["stream_options"]);
        Ok(())
    }
}
