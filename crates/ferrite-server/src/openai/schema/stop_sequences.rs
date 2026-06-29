use serde_json::Value;

pub(super) fn is_supported_stop_sequences(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(items)) => {
            items.len() <= 4
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|value| !value.is_empty()))
        }
        Some(_) => false,
    }
}

pub(super) fn stop_sequences(value: &Option<Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(ToOwned::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_stop_sequences_are_supported() {
        assert!(is_supported_stop_sequences(&None));
    }

    #[test]
    fn empty_stop_array_is_supported() {
        assert!(is_supported_stop_sequences(&Some(json!([]))));
    }

    #[test]
    fn stop_strings_and_non_empty_string_arrays_are_supported() {
        assert!(is_supported_stop_sequences(&Some(json!("."))));
        assert!(is_supported_stop_sequences(&Some(json!([".", "\n"]))));
    }

    #[test]
    fn malformed_stop_sequences_are_not_supported() {
        assert!(!is_supported_stop_sequences(&Some(json!(""))));
        assert!(!is_supported_stop_sequences(&Some(json!([""]))));
        assert!(!is_supported_stop_sequences(&Some(json!([1]))));
        assert!(!is_supported_stop_sequences(&Some(json!([
            "a", "b", "c", "d", "e"
        ]))));
    }

    #[test]
    fn extracts_supported_stop_sequences() {
        assert_eq!(stop_sequences(&Some(json!("."))), ["."]);
        assert_eq!(stop_sequences(&Some(json!([".", "\n"]))), [".", "\n"]);
        assert!(stop_sequences(&Some(json!([]))).is_empty());
    }
}
