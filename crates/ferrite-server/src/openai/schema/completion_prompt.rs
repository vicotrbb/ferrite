use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionPrompt {
    prompts: Vec<String>,
}

impl CompletionPrompt {
    pub fn prompts(&self) -> &[String] {
        &self.prompts
    }

    pub fn single_prompt(&self) -> Option<&str> {
        (self.prompts.len() == 1).then(|| self.prompts[0].as_str())
    }
}

impl<'de> Deserialize<'de> for CompletionPrompt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CompletionPromptWire::deserialize(deserializer)?;
        Ok(Self {
            prompts: wire.into_prompts(),
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CompletionPromptWire {
    Text(String),
    TextArray(Vec<String>),
}

impl CompletionPromptWire {
    fn into_prompts(self) -> Vec<String> {
        match self {
            Self::Text(prompt) => vec![prompt],
            Self::TextArray(prompts) => prompts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_string_prompt() -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str(r#""hello""#)?;

        assert_eq!(prompt.prompts(), ["hello"]);
        Ok(())
    }

    #[test]
    fn deserializes_array_of_string_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str(r#"["hello","world"]"#)?;

        assert_eq!(prompt.prompts(), ["hello", "world"]);
        Ok(())
    }

    #[test]
    fn rejects_token_prompt_forms() {
        assert!(serde_json::from_str::<CompletionPrompt>(r#"[1,2,3]"#).is_err());
        assert!(serde_json::from_str::<CompletionPrompt>(r#"[[1,2,3]]"#).is_err());
    }
}
