use serde_json::Value;

pub(super) fn is_seed(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Number(number)) => number.as_i64().is_some(),
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_seed_is_valid() {
        assert!(is_seed(&None));
    }

    #[test]
    fn integer_seed_is_valid() {
        assert!(is_seed(&Some(json!(0))));
        assert!(is_seed(&Some(json!(42))));
        assert!(is_seed(&Some(json!(-42))));
    }

    #[test]
    fn non_integer_seed_is_not_valid() {
        assert!(!is_seed(&Some(json!(4.2))));
        assert!(!is_seed(&Some(json!("42"))));
        assert!(!is_seed(&Some(json!({"seed": 42}))));
    }
}
