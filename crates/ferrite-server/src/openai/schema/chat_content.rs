use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatContent {
    text: String,
}

impl ChatContent {
    #[cfg(test)]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl<'de> Deserialize<'de> for ChatContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ChatContentWire::deserialize(deserializer)?;
        Ok(Self {
            text: wire.into_text(),
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChatContentWire {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl ChatContentWire {
    fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Parts(parts) => parts.into_iter().map(ContentPart::into_text).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum ContentPart {
    Text { text: String },
    Refusal { refusal: String },
}

impl ContentPart {
    fn into_text(self) -> String {
        match self {
            Self::Text { text } => text,
            Self::Refusal { refusal } => refusal,
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
        Ok(())
    }

    #[test]
    fn deserializes_text_content_parts() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent =
            serde_json::from_str(r#"[{"type":"text","text":"he"},{"type":"text","text":"llo"}]"#)?;

        assert_eq!(content.text(), "hello");
        Ok(())
    }

    #[test]
    fn deserializes_refusal_content_parts() -> Result<(), Box<dyn std::error::Error>> {
        let content: ChatContent =
            serde_json::from_str(r#"[{"type":"refusal","refusal":"hello"}]"#)?;

        assert_eq!(content.text(), "hello");
        Ok(())
    }

    #[test]
    fn rejects_non_text_content_parts() {
        let result = serde_json::from_str::<ChatContent>(
            r#"[{"type":"image_url","image_url":{"url":"https://example.test/image.png"}}]"#,
        );

        assert!(result.is_err(), "image content parts are not supported");
        if let Err(error) = result {
            assert!(error.is_data() || error.is_syntax());
        }
    }
}
