use super::config::LongChatGateError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LongChatStateCapsulePlacement {
    #[default]
    AssistantContext,
    FollowUp,
}

impl LongChatStateCapsulePlacement {
    pub fn parse(value: &str) -> Result<Self, LongChatGateError> {
        match value {
            "assistant-context" => Ok(Self::AssistantContext),
            "follow-up" => Ok(Self::FollowUp),
            _ => Err(LongChatGateError::new(
                "--generated-context-state-capsule-placement must be assistant-context or follow-up",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AssistantContext => "assistant-context",
            Self::FollowUp => "follow-up",
        }
    }

    pub fn decorates_assistant_context(self) -> bool {
        matches!(self, Self::AssistantContext)
    }

    pub fn decorates_follow_up(self) -> bool {
        matches!(self, Self::FollowUp)
    }
}

pub(super) fn format_state_capsule_context(capsule: &str, generated_context: &str) -> String {
    format!(
        "Ferrite state capsule:\n{capsule}\n\nGenerated assistant context:\n{generated_context}"
    )
}

pub(super) fn format_state_capsule_follow_up(capsule: &str, follow_up: &str) -> String {
    format!("Ferrite state capsule:\n{capsule}\n\nFollow-up instruction:\n{follow_up}")
}
