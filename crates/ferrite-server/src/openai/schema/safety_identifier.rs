use serde_json::Value;

const MAX_SAFETY_IDENTIFIER_CHARS: usize = 64;

pub(super) fn is_safety_identifier(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(value)) => value.chars().count() <= MAX_SAFETY_IDENTIFIER_CHARS,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_safety_identifier_is_valid() {
        assert!(is_safety_identifier(&None));
    }

    #[test]
    fn short_string_safety_identifier_is_valid() {
        assert!(is_safety_identifier(&Some(json!("hashed-local-user"))));
        assert!(is_safety_identifier(&Some(json!(""))));
    }

    #[test]
    fn long_safety_identifier_is_not_valid() {
        assert!(!is_safety_identifier(&Some(json!(
            "s".repeat(MAX_SAFETY_IDENTIFIER_CHARS + 1)
        ))));
    }

    #[test]
    fn non_string_safety_identifier_is_not_valid() {
        assert!(!is_safety_identifier(&Some(json!(123))));
        assert!(!is_safety_identifier(&Some(
            json!({"id": "hashed-local-user"})
        )));
    }
}
