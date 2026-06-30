use serde_json::Value;

pub(super) fn is_neutral_response_format(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Object(fields)) => fields
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "text"),
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_response_format_is_neutral() {
        assert!(is_neutral_response_format(&None));
    }

    #[test]
    fn text_response_format_is_neutral() {
        assert!(is_neutral_response_format(&Some(json!({"type": "text"}))));
    }

    #[test]
    fn null_response_format_is_neutral() {
        assert!(is_neutral_response_format(&Some(Value::Null)));
    }

    #[test]
    fn json_and_non_object_response_formats_are_not_neutral() {
        assert!(!is_neutral_response_format(&Some(
            json!({"type": "json_object"})
        )));
        assert!(!is_neutral_response_format(&Some(
            json!({"type": "json_schema", "json_schema": {}})
        )));
        assert!(!is_neutral_response_format(&Some(json!("text"))));
    }
}
