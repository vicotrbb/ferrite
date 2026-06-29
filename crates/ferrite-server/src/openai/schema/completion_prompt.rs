use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletionPrompt {
    prompts: Vec<String>,
    has_unsupported_form: bool,
}

impl CompletionPrompt {
    pub fn prompts(&self) -> &[String] {
        &self.prompts
    }

    pub fn has_unsupported_form(&self) -> bool {
        self.has_unsupported_form
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
        Ok(wire.into_prompt())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CompletionPromptWire {
    Text(String),
    TextArray(Vec<String>),
    TokenArray(Vec<i64>),
    TokenArrayBatch(Vec<Vec<i64>>),
}

impl CompletionPromptWire {
    fn into_prompt(self) -> CompletionPrompt {
        match self {
            Self::Text(prompt) => CompletionPrompt {
                prompts: vec![prompt],
                has_unsupported_form: false,
            },
            Self::TextArray(prompts) => CompletionPrompt {
                prompts,
                has_unsupported_form: false,
            },
            Self::TokenArray(tokens) => {
                drop(tokens);
                CompletionPrompt {
                    prompts: Vec::new(),
                    has_unsupported_form: true,
                }
            }
            Self::TokenArrayBatch(token_batches) => {
                drop(token_batches);
                CompletionPrompt {
                    prompts: Vec::new(),
                    has_unsupported_form: true,
                }
            }
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
        assert!(!prompt.has_unsupported_form());
        Ok(())
    }

    #[test]
    fn deserializes_array_of_string_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str(r#"["hello","world"]"#)?;

        assert_eq!(prompt.prompts(), ["hello", "world"]);
        assert!(!prompt.has_unsupported_form());
        Ok(())
    }

    #[test]
    fn records_token_prompt_forms_for_request_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let prompt: CompletionPrompt = serde_json::from_str(r#"[1,2,3]"#)?;

        assert!(prompt.prompts().is_empty());
        assert!(prompt.has_unsupported_form());
        Ok(())
    }

    #[test]
    fn records_token_prompt_batches_for_request_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str(r#"[[1,2,3]]"#)?;

        assert!(prompt.prompts().is_empty());
        assert!(prompt.has_unsupported_form());
        Ok(())
    }
}
