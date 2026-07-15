use super::{
    chat_content::ChatContent, function_options::is_no_function_call,
    message_metadata::is_optional_string, tool_options::ChatToolCall,
    unsupported::UnsupportedFields,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChatMessage {
    #[serde(default)]
    role: ChatRole,
    #[serde(default)]
    content: Option<ChatContent>,
    #[serde(default)]
    name: Option<Value>,
    #[serde(default)]
    tool_call_id: Option<Value>,
    #[serde(default)]
    tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(default)]
    function_call: Option<Value>,
    #[serde(default)]
    audio: Option<Value>,
    #[serde(default)]
    refusal: Option<Value>,
    #[serde(default, flatten)]
    extra_fields: BTreeMap<String, Value>,
}

impl ChatMessage {
    pub(super) fn from_request_value(value: Value) -> Self {
        match value {
            Value::Object(_) => serde_json::from_value(value).unwrap_or_else(|_| Self::malformed()),
            _ => Self::malformed(),
        }
    }

    fn malformed() -> Self {
        Self {
            role: ChatRole::Unknown,
            content: None,
            name: None,
            tool_call_id: None,
            tool_calls: None,
            function_call: None,
            audio: None,
            refusal: None,
            extra_fields: BTreeMap::new(),
        }
    }

    pub(crate) fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(ChatContent::from_text(content)),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            function_call: None,
            audio: None,
            refusal: None,
            extra_fields: BTreeMap::new(),
        }
    }

    pub fn role(&self) -> ChatRole {
        self.role
    }

    pub fn content(&self) -> &str {
        self.content.as_ref().map_or("", ChatContent::text)
    }

    pub(crate) fn tool_calls(&self) -> &[ChatToolCall] {
        self.tool_calls.as_deref().unwrap_or_default()
    }

    pub(crate) fn tool_call_id(&self) -> Option<&str> {
        self.tool_call_id.as_ref().and_then(Value::as_str)
    }

    pub(super) fn unsupported_fields(&self) -> Vec<String> {
        UnsupportedFields::new()
            .with_present("messages.role", self.role == ChatRole::Unknown)
            .with_present("messages.content", !self.content_matches_role())
            .with_present("messages.name", !self.name_matches_role())
            .with_present("messages.tool_call_id", !self.tool_call_id_matches_role())
            .with_present("messages.tool_calls", !self.tool_calls_match_role())
            .with_present(
                "messages.function_call",
                !is_no_function_call(&self.function_call),
            )
            .with_present("messages.audio", self.audio.is_some())
            .with_present("messages.refusal", self.refusal.is_some())
            .with_extra_keys_with_prefix("messages.", &self.extra_fields)
            .into_vec()
    }

    fn content_matches_role(&self) -> bool {
        match &self.content {
            Some(content) => {
                !content.has_unsupported_part()
                    && (self.role == ChatRole::Assistant || !content.has_refusal_part())
            }
            None => {
                self.role == ChatRole::Assistant
                    && (self.tool_calls.is_some() || self.function_call.is_some())
            }
        }
    }

    fn name_matches_role(&self) -> bool {
        match self.role {
            ChatRole::Function => self.name.as_ref().is_some_and(Value::is_string),
            _ => is_optional_string(&self.name),
        }
    }

    fn tool_message_missing_tool_call_id(&self) -> bool {
        self.role == ChatRole::Tool && self.tool_call_id.is_none()
    }

    fn tool_calls_match_role(&self) -> bool {
        match self.tool_calls.as_deref() {
            None | Some([]) => true,
            Some(calls) => {
                self.role == ChatRole::Assistant && calls.iter().all(ChatToolCall::is_valid)
            }
        }
    }

    fn tool_call_id_matches_role(&self) -> bool {
        match self.role {
            ChatRole::Tool => {
                is_optional_string(&self.tool_call_id) && !self.tool_message_missing_tool_call_id()
            }
            _ => self.tool_call_id.is_none(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatRole {
    Developer,
    System,
    User,
    Assistant,
    Tool,
    Function,
    #[default]
    Unknown,
}

impl<'de> Deserialize<'de> for ChatRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(match value.as_str() {
            Some("developer") => Self::Developer,
            Some("system") => Self::System,
            Some("user") => Self::User,
            Some("assistant") => Self::Assistant,
            Some("tool") => Self::Tool,
            Some("function") => Self::Function,
            _ => Self::Unknown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_unknown_message_role_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message: ChatMessage = serde_json::from_str(r#"{"role":"critic","content":"hello"}"#)?;

        assert_eq!(message.unsupported_fields(), ["messages.role"]);
        Ok(())
    }

    #[test]
    fn records_missing_message_role_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message: ChatMessage = serde_json::from_str(r#"{"content":"hello"}"#)?;

        assert_eq!(message.unsupported_fields(), ["messages.role"]);
        Ok(())
    }

    #[test]
    fn records_malformed_message_item_for_request_validation() {
        let message = ChatMessage::from_request_value(Value::Number(42.into()));

        assert_eq!(
            message.unsupported_fields(),
            ["messages.role", "messages.content"]
        );
    }
}
