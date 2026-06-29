use serde_json::Value;

const MAX_METADATA_PAIRS: usize = 16;
const MAX_METADATA_KEY_CHARS: usize = 64;
const MAX_METADATA_VALUE_CHARS: usize = 512;

pub(super) fn is_valid_metadata(value: &Option<Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let Some(metadata) = value.as_object() else {
        return false;
    };
    metadata.len() <= MAX_METADATA_PAIRS
        && metadata.iter().all(|(key, value)| {
            key.chars().count() <= MAX_METADATA_KEY_CHARS
                && value
                    .as_str()
                    .is_some_and(|value| value.chars().count() <= MAX_METADATA_VALUE_CHARS)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_metadata_is_valid() {
        assert!(is_valid_metadata(&None));
    }

    #[test]
    fn string_key_value_metadata_is_valid() {
        assert!(is_valid_metadata(&Some(json!({
            "trace_id": "local-123",
            "tenant": "dev"
        }))));
    }

    #[test]
    fn non_object_or_non_string_metadata_is_not_valid() {
        assert!(!is_valid_metadata(&Some(json!("trace"))));
        assert!(!is_valid_metadata(&Some(json!({"trace_id": 123}))));
    }

    #[test]
    fn metadata_size_limits_are_enforced() {
        let too_many_pairs = (0..=MAX_METADATA_PAIRS)
            .map(|index| (format!("key{index}"), json!("value")))
            .collect();
        assert!(!is_valid_metadata(&Some(Value::Object(too_many_pairs))));

        let too_long_key = "k".repeat(MAX_METADATA_KEY_CHARS + 1);
        assert!(!is_valid_metadata(&Some(json!({too_long_key: "value"}))));

        let too_long_value = "v".repeat(MAX_METADATA_VALUE_CHARS + 1);
        assert!(!is_valid_metadata(&Some(json!({"key": too_long_value}))));
    }
}
