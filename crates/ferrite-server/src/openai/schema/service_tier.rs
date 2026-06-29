use serde_json::Value;

const LOCAL_SERVICE_TIER: &str = "default";

pub(super) fn is_local_service_tier(value: &Option<Value>) -> bool {
    match value {
        None => true,
        Some(Value::String(tier)) => tier == "auto" || tier == LOCAL_SERVICE_TIER,
        Some(_) => false,
    }
}

pub(super) fn response_service_tier(value: &Option<Value>) -> Option<&'static str> {
    match value {
        Some(Value::String(tier)) if tier == "auto" || tier == LOCAL_SERVICE_TIER => {
            Some(LOCAL_SERVICE_TIER)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_service_tier_is_neutral_without_response_value() {
        assert!(is_local_service_tier(&None));
        assert_eq!(response_service_tier(&None), None);
    }

    #[test]
    fn auto_and_default_service_tiers_resolve_to_local_default() {
        assert!(is_local_service_tier(&Some(json!("auto"))));
        assert!(is_local_service_tier(&Some(json!("default"))));
        assert_eq!(response_service_tier(&Some(json!("auto"))), Some("default"));
        assert_eq!(
            response_service_tier(&Some(json!("default"))),
            Some("default")
        );
    }

    #[test]
    fn alternative_or_malformed_service_tiers_are_not_local() {
        assert!(!is_local_service_tier(&Some(json!("flex"))));
        assert!(!is_local_service_tier(&Some(json!("scale"))));
        assert!(!is_local_service_tier(&Some(json!("priority"))));
        assert!(!is_local_service_tier(&Some(json!(0))));
        assert_eq!(response_service_tier(&Some(json!("flex"))), None);
    }
}
