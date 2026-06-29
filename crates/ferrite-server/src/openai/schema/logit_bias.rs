use serde_json::Value;

pub fn is_neutral_logit_bias(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Object(map)) => map.is_empty(),
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_missing_null_and_empty_object() {
        assert!(is_neutral_logit_bias(&None));
        assert!(is_neutral_logit_bias(&Some(Value::Null)));
        assert!(is_neutral_logit_bias(&Some(json!({}))));
    }

    #[test]
    fn rejects_non_empty_or_malformed_values() {
        assert!(!is_neutral_logit_bias(&Some(json!({"42": -100}))));
        assert!(!is_neutral_logit_bias(&Some(json!([]))));
        assert!(!is_neutral_logit_bias(&Some(json!(false))));
    }
}
