use serde_json::Value;

pub(super) fn is_text_only_modalities(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Array(items)) => {
            matches!(items.as_slice(), [Value::String(item)] if item == "text")
        }
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_modalities_are_text_only() {
        assert!(is_text_only_modalities(&None));
    }

    #[test]
    fn explicit_text_modalities_are_text_only() {
        assert!(is_text_only_modalities(&Some(json!(["text"]))));
    }

    #[test]
    fn audio_or_malformed_modalities_are_not_text_only() {
        assert!(!is_text_only_modalities(&Some(json!(["audio"]))));
        assert!(!is_text_only_modalities(&Some(json!(["text", "audio"]))));
        assert!(!is_text_only_modalities(&Some(json!("text"))));
        assert!(!is_text_only_modalities(&Some(json!([]))));
    }
}
