use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResponseFormatKind {
    Text,
    JsonObject,
    Unsupported,
}

pub(super) fn response_format_kind(value: &Option<Value>) -> ResponseFormatKind {
    match value {
        None | Some(Value::Null) => ResponseFormatKind::Text,
        Some(Value::Object(fields)) if fields.len() == 1 => {
            match fields.get("type").and_then(Value::as_str) {
                Some("text") => ResponseFormatKind::Text,
                Some("json_object") => ResponseFormatKind::JsonObject,
                Some(_) | None => ResponseFormatKind::Unsupported,
            }
        }
        Some(_) => ResponseFormatKind::Unsupported,
    }
}

pub(super) fn is_supported_response_format(value: &Option<Value>) -> bool {
    response_format_kind(value) != ResponseFormatKind::Unsupported
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_response_format_is_neutral() {
        assert_eq!(response_format_kind(&None), ResponseFormatKind::Text);
    }

    #[test]
    fn text_response_format_is_neutral() {
        assert_eq!(
            response_format_kind(&Some(json!({"type": "text"}))),
            ResponseFormatKind::Text
        );
    }

    #[test]
    fn null_response_format_is_neutral() {
        assert_eq!(
            response_format_kind(&Some(Value::Null)),
            ResponseFormatKind::Text
        );
    }

    #[test]
    fn json_object_is_supported_but_schema_and_malformed_formats_are_not() {
        assert_eq!(
            response_format_kind(&Some(json!({"type": "json_object"}))),
            ResponseFormatKind::JsonObject
        );
        assert!(!is_supported_response_format(&Some(
            json!({"type": "json_schema", "json_schema": {}})
        )));
        assert!(!is_supported_response_format(&Some(json!("text"))));
    }
}
