use serde_json::Value;

pub(super) fn is_user_identifier(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(_)) => true,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_user_identifier_is_valid() {
        assert!(is_user_identifier(&None));
    }

    #[test]
    fn string_user_identifier_is_valid() {
        assert!(is_user_identifier(&Some(json!("local-user-1"))));
        assert!(is_user_identifier(&Some(json!(""))));
    }

    #[test]
    fn null_user_identifier_is_valid() {
        assert!(is_user_identifier(&Some(Value::Null)));
    }

    #[test]
    fn non_string_user_identifier_is_not_valid() {
        assert!(!is_user_identifier(&Some(json!(123))));
        assert!(!is_user_identifier(&Some(json!({"id": "local-user-1"}))));
    }
}
