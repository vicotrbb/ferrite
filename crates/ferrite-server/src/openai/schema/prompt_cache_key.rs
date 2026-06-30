use serde_json::Value;

pub(super) fn is_prompt_cache_key(value: &Option<Value>) -> bool {
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
    fn missing_prompt_cache_key_is_valid() {
        assert!(is_prompt_cache_key(&None));
    }

    #[test]
    fn string_prompt_cache_key_is_valid() {
        assert!(is_prompt_cache_key(&Some(json!("tenant-a:prompt-1"))));
        assert!(is_prompt_cache_key(&Some(json!(""))));
    }

    #[test]
    fn null_prompt_cache_key_is_valid() {
        assert!(is_prompt_cache_key(&Some(Value::Null)));
    }

    #[test]
    fn non_string_prompt_cache_key_is_not_valid() {
        assert!(!is_prompt_cache_key(&Some(json!(123))));
        assert!(!is_prompt_cache_key(&Some(json!({"key": "tenant-a"}))));
    }
}
