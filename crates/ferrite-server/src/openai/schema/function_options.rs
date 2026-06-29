use serde_json::Value;

pub(super) fn is_empty_functions(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Array(functions)) => functions.is_empty(),
        Some(_) => false,
    }
}

pub(super) fn is_no_function_call(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(choice)) => choice == "none",
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_function_options_are_no_function_options() {
        assert!(is_empty_functions(&None));
        assert!(is_no_function_call(&None));
    }

    #[test]
    fn explicit_no_function_options_are_neutral() {
        assert!(is_empty_functions(&Some(json!([]))));
        assert!(is_no_function_call(&Some(json!("none"))));
    }

    #[test]
    fn enabled_function_options_are_not_neutral() {
        assert!(!is_empty_functions(&Some(json!([
            {"name": "lookup", "parameters": {"type": "object"}}
        ]))));
        assert!(!is_empty_functions(&Some(json!("none"))));
        assert!(!is_no_function_call(&Some(json!("auto"))));
        assert!(!is_no_function_call(&Some(json!({"name": "lookup"}))));
    }
}
