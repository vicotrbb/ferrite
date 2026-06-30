use serde_json::Value;

pub(super) fn is_no_reasoning_effort(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(effort)) => effort == "none",
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_reasoning_effort_is_neutral() {
        assert!(is_no_reasoning_effort(&None));
    }

    #[test]
    fn none_reasoning_effort_is_neutral() {
        assert!(is_no_reasoning_effort(&Some(json!("none"))));
    }

    #[test]
    fn null_reasoning_effort_is_neutral() {
        assert!(is_no_reasoning_effort(&Some(Value::Null)));
    }

    #[test]
    fn enabled_or_malformed_reasoning_effort_is_not_neutral() {
        assert!(!is_no_reasoning_effort(&Some(json!("minimal"))));
        assert!(!is_no_reasoning_effort(&Some(json!("low"))));
        assert!(!is_no_reasoning_effort(&Some(json!("medium"))));
        assert!(!is_no_reasoning_effort(&Some(json!(0))));
    }
}
