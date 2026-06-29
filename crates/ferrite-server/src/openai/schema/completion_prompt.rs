use serde::{Deserialize, Deserializer};
use serde_json::Value;

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
        Ok(CompletionPromptWire::from_value(Value::deserialize(deserializer)?).into_prompt())
    }
}

enum CompletionPromptWire {
    Text(String),
    TextArray(Vec<String>),
    TokenArray(Vec<i64>),
    TokenArrayBatch(Vec<Vec<i64>>),
    Unsupported,
}

impl CompletionPromptWire {
    fn from_value(value: Value) -> Self {
        match value {
            Value::String(prompt) => Self::Text(prompt),
            Value::Array(values) => Self::from_array(values),
            _ => Self::Unsupported,
        }
    }

    fn from_array(values: Vec<Value>) -> Self {
        if values.iter().all(Value::is_string) {
            return Self::TextArray(
                values
                    .into_iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect(),
            );
        }
        if values.iter().all(Value::is_i64) {
            return Self::TokenArray(
                values
                    .into_iter()
                    .filter_map(|value| value.as_i64())
                    .collect(),
            );
        }
        if values.iter().all(is_token_array) {
            return Self::TokenArrayBatch(
                values
                    .into_iter()
                    .filter_map(|value| match value {
                        Value::Array(tokens) => Some(
                            tokens
                                .into_iter()
                                .filter_map(|token| token.as_i64())
                                .collect(),
                        ),
                        _ => None,
                    })
                    .collect(),
            );
        }
        Self::Unsupported
    }

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
            Self::Unsupported => CompletionPrompt {
                prompts: Vec::new(),
                has_unsupported_form: true,
            },
        }
    }
}

fn is_token_array(value: &Value) -> bool {
    match value {
        Value::Array(tokens) => tokens.iter().all(Value::is_i64),
        _ => false,
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

    #[test]
    fn records_null_prompt_for_request_validation() -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str("null")?;

        assert!(prompt.prompts().is_empty());
        assert!(prompt.has_unsupported_form());
        Ok(())
    }

    #[test]
    fn records_object_prompt_for_request_validation() -> Result<(), Box<dyn std::error::Error>> {
        let prompt: CompletionPrompt = serde_json::from_str(r#"{"text":"hello"}"#)?;

        assert!(prompt.prompts().is_empty());
        assert!(prompt.has_unsupported_form());
        Ok(())
    }
}
