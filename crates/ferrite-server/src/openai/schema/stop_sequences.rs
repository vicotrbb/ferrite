use serde_json::Value;

pub(super) fn is_neutral_stop_sequences(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::Array(items)) => items.is_empty(),
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_stop_sequences_are_neutral() {
        assert!(is_neutral_stop_sequences(&None));
    }

    #[test]
    fn empty_stop_array_is_neutral() {
        assert!(is_neutral_stop_sequences(&Some(json!([]))));
    }

    #[test]
    fn stop_strings_and_non_empty_arrays_are_not_neutral() {
        assert!(!is_neutral_stop_sequences(&Some(json!("."))));
        assert!(!is_neutral_stop_sequences(&Some(json!(["."]))));
    }
}
