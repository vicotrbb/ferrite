use super::unsupported::UnsupportedFields;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct StreamOptions {
    #[serde(default)]
    include_usage: Option<bool>,
    #[serde(default, flatten)]
    extra_fields: BTreeMap<String, Value>,
}

impl StreamOptions {
    pub fn include_usage(&self) -> bool {
        self.include_usage.unwrap_or(false)
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        UnsupportedFields::new()
            .with_extra_keys(&self.extra_fields)
            .into_vec()
    }
}
