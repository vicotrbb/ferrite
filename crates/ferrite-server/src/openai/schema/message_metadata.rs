use serde_json::Value;

pub(super) fn is_optional_string(value: &Option<Value>) -> bool {
    matches!(value, None | Some(Value::String(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_metadata_is_valid() {
        assert!(is_optional_string(&None));
    }

    #[test]
    fn string_metadata_is_valid() {
        assert!(is_optional_string(&Some(json!("lookup"))));
        assert!(is_optional_string(&Some(json!(""))));
    }

    #[test]
    fn non_string_metadata_is_not_valid() {
        assert!(!is_optional_string(&Some(json!(123))));
        assert!(!is_optional_string(&Some(json!({"id": "lookup"}))));
    }
}
