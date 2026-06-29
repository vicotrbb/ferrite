use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub(super) fn deserialize_model_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Value::deserialize(deserializer)? {
        Value::String(model) => model,
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Request {
        #[serde(default, deserialize_with = "deserialize_model_id")]
        model: String,
    }

    #[test]
    fn deserializes_string_model_id() -> Result<(), Box<dyn std::error::Error>> {
        let request: Request = serde_json::from_str(r#"{"model":"fixture-model"}"#)?;

        assert_eq!(request.model, "fixture-model");
        Ok(())
    }

    #[test]
    fn records_non_string_model_id_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let request: Request = serde_json::from_str(r#"{"model":42}"#)?;

        assert!(request.model.is_empty());
        Ok(())
    }

    #[test]
    fn records_null_model_id_for_request_validation() -> Result<(), Box<dyn std::error::Error>> {
        let request: Request = serde_json::from_str(r#"{"model":null}"#)?;

        assert!(request.model.is_empty());
        Ok(())
    }
}
