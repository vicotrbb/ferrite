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

pub(super) fn is_neutral_function_call(value: &Option<Value>, functions: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(choice)) if choice == "none" => true,
        Some(Value::String(choice)) if choice == "auto" => is_empty_functions(functions),
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
        assert!(is_neutral_function_call(&None, &None));
    }

    #[test]
    fn explicit_no_function_options_are_neutral() {
        assert!(is_empty_functions(&Some(json!([]))));
        assert!(is_no_function_call(&Some(json!("none"))));
        assert!(is_neutral_function_call(&Some(json!("none")), &None));
        assert!(is_neutral_function_call(&Some(json!("auto")), &None));
        assert!(is_neutral_function_call(
            &Some(json!("auto")),
            &Some(json!([]))
        ));
    }

    #[test]
    fn enabled_function_options_are_not_neutral() {
        let functions = Some(json!([
            {"name": "lookup", "parameters": {"type": "object"}}
        ]));
        assert!(!is_empty_functions(&functions));
        assert!(!is_empty_functions(&Some(json!("none"))));
        assert!(!is_no_function_call(&Some(json!("auto"))));
        assert!(!is_neutral_function_call(&Some(json!("auto")), &functions));
        assert!(!is_neutral_function_call(
            &Some(json!({"name": "lookup"})),
            &None
        ));
    }
}
