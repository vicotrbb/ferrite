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

pub(super) fn is_neutral_parallel_tool_calls(value: &Option<Value>, tools: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Bool(_)) => is_empty_tools(tools),
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
        assert!(is_neutral_parallel_tool_calls(&None, &None));
    }

    #[test]
    fn explicit_no_tool_options_are_neutral() {
        assert!(is_empty_tools(&Some(json!([]))));
        assert!(is_no_tool_choice(&Some(json!("none"))));
        assert!(is_neutral_parallel_tool_calls(
            &Some(json!(false)),
            &Some(json!([]))
        ));
        assert!(is_neutral_parallel_tool_calls(
            &Some(json!(true)),
            &Some(json!([]))
        ));
    }

    #[test]
    fn enabled_tool_options_are_not_neutral() {
        let tools = Some(json!([
            {"type": "function", "function": {"name": "lookup"}}
        ]));
        assert!(!is_empty_tools(&tools));
        assert!(!is_no_tool_choice(&Some(json!("auto"))));
        assert!(!is_no_tool_choice(&Some(json!({"type": "function"}))));
        assert!(!is_neutral_parallel_tool_calls(&Some(json!(true)), &tools));
        assert!(!is_neutral_parallel_tool_calls(&Some(json!("true")), &None));
    }
}
