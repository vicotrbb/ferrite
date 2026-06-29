use super::chat::ChatMessage;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub(super) fn deserialize_chat_messages<'de, D>(
    deserializer: D,
) -> Result<Vec<ChatMessage>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Value::deserialize(deserializer)? {
        Value::Array(messages) => {
            serde_json::from_value(Value::Array(messages)).unwrap_or_default()
        }
        _ => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Request {
        #[serde(default, deserialize_with = "deserialize_chat_messages")]
        messages: Vec<ChatMessage>,
    }

    #[test]
    fn deserializes_valid_chat_messages() -> Result<(), Box<dyn std::error::Error>> {
        let request: Request =
            serde_json::from_str(r#"{"messages":[{"role":"user","content":"hello"}]}"#)?;

        assert_eq!(request.messages.len(), 1);
        Ok(())
    }

    #[test]
    fn records_null_messages_for_request_validation() -> Result<(), Box<dyn std::error::Error>> {
        let request: Request = serde_json::from_str(r#"{"messages":null}"#)?;

        assert!(request.messages.is_empty());
        Ok(())
    }

    #[test]
    fn records_non_array_messages_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let request: Request = serde_json::from_str(r#"{"messages":"hello"}"#)?;

        assert!(request.messages.is_empty());
        Ok(())
    }
}
