use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatContent {
    text: String,
    has_refusal_part: bool,
    has_unsupported_part: bool,
}

impl ChatContent {
    pub(crate) fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            has_refusal_part: false,
            has_unsupported_part: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn has_refusal_part(&self) -> bool {
        self.has_refusal_part
    }

    pub fn has_unsupported_part(&self) -> bool {
        self.has_unsupported_part
    }
}

impl<'de> Deserialize<'de> for ChatContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ChatContentWire::from_value(Value::deserialize(deserializer)?).into_content())
    }
}

enum ChatContentWire {
    Text(String),
    Parts(Vec<ContentPart>),
    Unsupported,
}

impl ChatContentWire {
    fn from_value(value: Value) -> Self {
        match value {
            Value::String(text) => Self::Text(text),
            Value::Array(parts) => Self::Parts(
                parts
                    .into_iter()
                    .map(ContentPart::from_value)
                    .collect::<Vec<_>>(),
            ),
            _ => Self::Unsupported,
        }
    }
}

impl ChatContentWire {
    fn into_content(self) -> ChatContent {
        match self {
            Self::Text(text) => ChatContent {
                text,
                has_refusal_part: false,
                has_unsupported_part: false,
            },
            Self::Parts(parts) => {
                let has_refusal_part = parts.iter().any(ContentPart::is_refusal);
                let has_unsupported_part = parts.iter().any(ContentPart::is_unsupported);
                ChatContent {
                    text: parts.into_iter().map(ContentPart::into_text).collect(),
                    has_refusal_part,
                    has_unsupported_part,
                }
            }
            Self::Unsupported => ChatContent {
                text: String::new(),
                has_refusal_part: false,
                has_unsupported_part: true,
            },
        }
    }
}

enum ContentPart {
    Text { text: String },
    Refusal { refusal: String },
    Unsupported,
}

impl ContentPart {
    fn from_value(value: Value) -> Self {
        match value.get("type").and_then(Value::as_str) {
            Some("text") => value
                .get("text")
                .and_then(Value::as_str)
                .map(|text| Self::Text {
                    text: text.to_owned(),
                })
                .unwrap_or(Self::Unsupported),
            Some("refusal") => value
                .get("refusal")
                .and_then(Value::as_str)
                .map(|refusal| Self::Refusal {
                    refusal: refusal.to_owned(),
                })
                .unwrap_or(Self::Unsupported),
            _ => Self::Unsupported,
        }
    }

    fn is_refusal(&self) -> bool {
        matches!(self, Self::Refusal { .. })
    }

    fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported)
    }

    fn into_text(self) -> String {
        match self {
            Self::Text { text } => text,
            Self::Refusal { refusal } => refusal,
            Self::Unsupported => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_string_content() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent = serde_json::from_str(r#""hello""#)?;

        assert_eq!(content.text(), "hello");
        assert!(!content.has_refusal_part());
        assert!(!content.has_unsupported_part());
        Ok(())
    }

    #[test]
    fn deserializes_text_content_parts() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent =
            serde_json::from_str(r#"[{"type":"text","text":"he"},{"type":"text","text":"llo"}]"#)?;

        assert_eq!(content.text(), "hello");
        assert!(!content.has_refusal_part());
        assert!(!content.has_unsupported_part());
        Ok(())
    }

    #[test]
    fn deserializes_refusal_content_parts() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent =
            serde_json::from_str(r#"[{"type":"refusal","refusal":"hello"}]"#)?;

        assert_eq!(content.text(), "hello");
        assert!(content.has_refusal_part());
        assert!(!content.has_unsupported_part());
        Ok(())
    }

    #[test]
    fn records_non_text_content_parts_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent = serde_json::from_str(
            r#"[{"type":"image_url","image_url":{"url":"https://example.test/image.png"}}]"#,
        )?;

        assert_eq!(content.text(), "");
        assert!(!content.has_refusal_part());
        assert!(content.has_unsupported_part());
        Ok(())
    }

    #[test]
    fn records_malformed_text_content_parts_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent = serde_json::from_str(r#"[{"type":"text"}]"#)?;

        assert_eq!(content.text(), "");
        assert!(!content.has_refusal_part());
        assert!(content.has_unsupported_part());
        Ok(())
    }

    #[test]
    fn records_scalar_content_for_request_validation() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent = serde_json::from_str("42")?;

        assert_eq!(content.text(), "");
        assert!(!content.has_refusal_part());
        assert!(content.has_unsupported_part());
        Ok(())
    }
}
