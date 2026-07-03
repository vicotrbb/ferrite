use super::config::LongChatGateError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LongChatStateCapsulePlacement {
    #[default]
    AssistantContext,
    AssistantContextOnly,
    FollowUp,
}

impl LongChatStateCapsulePlacement {
    pub fn parse(value: &str) -> Result<Self, LongChatGateError> {
        match value {
            "assistant-context" => Ok(Self::AssistantContext),
            "assistant-context-only" => Ok(Self::AssistantContextOnly),
            "follow-up" => Ok(Self::FollowUp),
            _ => Err(LongChatGateError::new(
                "--generated-context-state-capsule-placement must be assistant-context, assistant-context-only, or follow-up",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AssistantContext => "assistant-context",
            Self::AssistantContextOnly => "assistant-context-only",
            Self::FollowUp => "follow-up",
        }
    }

    pub fn decorates_assistant_context(self) -> bool {
        matches!(self, Self::AssistantContext)
    }

    pub fn decorates_follow_up(self) -> bool {
        matches!(self, Self::FollowUp)
    }

    pub fn replaces_assistant_context(self) -> bool {
        matches!(self, Self::AssistantContextOnly)
    }
}

pub(super) fn format_state_capsule_context(capsule: &str, generated_context: &str) -> String {
    format!(
        "Ferrite state capsule:\n{capsule}\n\nGenerated assistant context:\n{generated_context}"
    )
}

pub(super) fn format_state_capsule_only(capsule: &str) -> String {
    format!("Ferrite state capsule:\n{capsule}")
}

pub(super) fn format_state_capsule_follow_up(capsule: &str, follow_up: &str) -> String {
    format!("Ferrite state capsule:\n{capsule}\n\nFollow-up instruction:\n{follow_up}")
}
