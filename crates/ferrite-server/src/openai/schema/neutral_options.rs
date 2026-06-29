use serde_json::Value;

pub(super) fn is_neutral_number(value: &Option<Value>, expected: f64) -> bool {
    value
        .as_ref()
        .is_none_or(|value| number_equals(value, expected))
}

fn number_equals(value: &Value, expected: f64) -> bool {
    value.as_f64().is_some_and(|actual| actual == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_value_is_neutral() {
        assert!(is_neutral_number(&None, 0.0));
    }

    #[test]
    fn matching_number_is_neutral() {
        assert!(is_neutral_number(&Some(json!(0)), 0.0));
        assert!(is_neutral_number(&Some(json!(1.0)), 1.0));
    }

    #[test]
    fn non_matching_or_non_number_value_is_not_neutral() {
        assert!(!is_neutral_number(&Some(json!(0.2)), 0.0));
        assert!(!is_neutral_number(&Some(json!("0")), 0.0));
    }
}
