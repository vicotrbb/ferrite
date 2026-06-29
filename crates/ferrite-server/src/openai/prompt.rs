use super::{
    error::OpenAiHttpError,
    schema::{ChatMessage, ChatRole},
};

pub fn render_chat_prompt(messages: &[ChatMessage]) -> Result<String, OpenAiHttpError> {
    if messages.is_empty() {
        return Err(OpenAiHttpError::invalid_request(
            "messages must contain at least one item",
        ));
    }

    let mut prompt = String::new();
    for message in messages {
        push_message(&mut prompt, message);
    }
    prompt.push_str("assistant: ");
    Ok(prompt)
}

fn push_message(prompt: &mut String, message: &ChatMessage) {
    prompt.push_str(role_label(message.role()));
    prompt.push_str(": ");
    prompt.push_str(message.content());
    prompt.push('\n');
}

fn role_label(role: ChatRole) -> &'static str {
    match role {
        ChatRole::Developer => "developer",
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
        ChatRole::Function => "function",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_messages_into_stable_prompt() -> Result<(), Box<dyn std::error::Error>> {
        let messages = [
            ChatMessage::new(ChatRole::System, "Be brief"),
            ChatMessage::new(ChatRole::User, "Hello"),
        ];

        assert_eq!(
            render_chat_prompt(&messages)?,
            "system: Be brief\nuser: Hello\nassistant: "
        );
        Ok(())
    }
}
