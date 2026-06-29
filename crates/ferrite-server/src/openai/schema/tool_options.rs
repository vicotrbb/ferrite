use serde_json::Value;

pub(super) fn is_empty_tools(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Array(items)) => items.is_empty(),
        Some(_) => false,
    }
}

pub(super) fn is_no_tool_choice(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(choice)) => choice == "none",
        Some(_) => false,
    }
}

pub(super) fn is_disabled_parallel_tool_calls(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Bool(enabled)) => !enabled,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_tool_options_are_no_tool_options() {
        assert!(is_empty_tools(&None));
        assert!(is_no_tool_choice(&None));
        assert!(is_disabled_parallel_tool_calls(&None));
    }

    #[test]
    fn explicit_no_tool_options_are_neutral() {
        assert!(is_empty_tools(&Some(json!([]))));
        assert!(is_no_tool_choice(&Some(json!("none"))));
        assert!(is_disabled_parallel_tool_calls(&Some(json!(false))));
    }

    #[test]
    fn enabled_tool_options_are_not_neutral() {
        assert!(!is_empty_tools(&Some(json!([
            {"type": "function", "function": {"name": "lookup"}}
        ]))));
        assert!(!is_no_tool_choice(&Some(json!("auto"))));
        assert!(!is_no_tool_choice(&Some(json!({"type": "function"}))));
        assert!(!is_disabled_parallel_tool_calls(&Some(json!(true))));
    }
}
