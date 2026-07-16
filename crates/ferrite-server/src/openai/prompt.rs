use super::{
    error::OpenAiHttpError,
    schema::{ChatMessage, ChatRole},
};
use std::collections::BTreeSet;

const MAX_TEMPLATE_BYTES: usize = 64 * 1024;
const MAX_RENDERED_PROMPT_BYTES: usize = 16 * 1024 * 1024;
const QWEN_DEFAULT_SYSTEM: &str =
    "You are Qwen, created by Alibaba Cloud. You are a helpful assistant.";
const SMOLLM2_DEFAULT_SYSTEM: &str =
    "You are a helpful AI assistant named SmolLM, trained by Hugging Face";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChatTemplateFamily {
    ChatMl,
    Llama3,
    Llama2,
    Phi3,
    Fallback,
}

pub fn render_chat_prompt(messages: &[ChatMessage]) -> Result<String, OpenAiHttpError> {
    render_chat_prompt_with_model_template(None, None, messages)
}

pub(crate) fn validate_chat_messages(messages: &[ChatMessage]) -> Result<(), OpenAiHttpError> {
    if messages.is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "messages must contain at least one item",
            "messages",
        ));
    }
    let mut pending_tool_call_ids = BTreeSet::new();
    for message in messages {
        match message.role() {
            ChatRole::Assistant if !message.tool_calls().is_empty() => {
                if !pending_tool_call_ids.is_empty() {
                    return Err(OpenAiHttpError::invalid_request_with_param(
                        "assistant tool calls require responses before another assistant tool call",
                        "messages.tool_calls",
                    ));
                }
                for tool_call in message.tool_calls() {
                    if !pending_tool_call_ids.insert(tool_call.id().to_owned()) {
                        return Err(OpenAiHttpError::invalid_request_with_param(
                            "assistant tool call IDs must be unique",
                            "messages.tool_calls",
                        ));
                    }
                }
            }
            ChatRole::Tool => {
                let Some(tool_call_id) = message.tool_call_id() else {
                    continue;
                };
                if !pending_tool_call_ids.remove(tool_call_id) {
                    return Err(OpenAiHttpError::invalid_request_with_param(
                        format!("tool message references unknown tool call ID {tool_call_id:?}"),
                        "messages.tool_call_id",
                    ));
                }
            }
            _ if !pending_tool_call_ids.is_empty() => {
                return Err(OpenAiHttpError::invalid_request_with_param(
                    "every assistant tool call requires a matching tool response before the next message",
                    "messages.tool_call_id",
                ));
            }
            _ => {}
        }
    }
    if !pending_tool_call_ids.is_empty() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "every assistant tool call requires a matching tool response",
            "messages.tool_call_id",
        ));
    }
    Ok(())
}

pub(crate) fn render_chat_prompt_with_model_template(
    template: Option<&str>,
    bos_token: Option<&str>,
    messages: &[ChatMessage],
) -> Result<String, OpenAiHttpError> {
    render_chat_prompt_with_model_template_and_tools(template, bos_token, messages, None)
}

pub(crate) fn render_chat_prompt_with_model_template_and_tools(
    template: Option<&str>,
    bos_token: Option<&str>,
    messages: &[ChatMessage],
    tool_prompt_suffix: Option<&str>,
) -> Result<String, OpenAiHttpError> {
    validate_chat_messages(messages)?;

    let family = classify_template(template);
    let prompt = match family {
        ChatTemplateFamily::ChatMl => {
            render_chatml(template.unwrap_or_default(), messages, tool_prompt_suffix)?
        }
        ChatTemplateFamily::Llama3 => {
            reject_tools_for_template(tool_prompt_suffix, "Llama 3")?;
            render_llama3(template.unwrap_or_default(), bos_token, messages)?
        }
        ChatTemplateFamily::Llama2 => {
            reject_tools_for_template(tool_prompt_suffix, "Llama 2")?;
            render_llama2(template.unwrap_or_default(), bos_token, messages)?
        }
        ChatTemplateFamily::Phi3 => {
            reject_tools_for_template(tool_prompt_suffix, "Phi-3")?;
            render_phi3(template.unwrap_or_default(), bos_token, messages)?
        }
        ChatTemplateFamily::Fallback => {
            reject_tools_for_template(tool_prompt_suffix, "fallback")?;
            render_fallback(messages)?
        }
    };
    if prompt.len() > MAX_RENDERED_PROMPT_BYTES {
        return Err(OpenAiHttpError::invalid_request_with_param(
            format!("rendered chat prompt exceeds {MAX_RENDERED_PROMPT_BYTES} bytes"),
            "messages",
        ));
    }
    Ok(prompt)
}

fn reject_tools_for_template(
    tool_prompt_suffix: Option<&str>,
    family: &str,
) -> Result<(), OpenAiHttpError> {
    if tool_prompt_suffix.is_some() {
        return Err(OpenAiHttpError::invalid_request_with_param(
            format!("function tools are not supported by the loaded {family} chat template"),
            "tools",
        ));
    }
    Ok(())
}

fn classify_template(template: Option<&str>) -> ChatTemplateFamily {
    let Some(template) = template.filter(|template| template.len() <= MAX_TEMPLATE_BYTES) else {
        return ChatTemplateFamily::Fallback;
    };
    if template.contains("<|im_start|>") && template.contains("<|im_end|>") {
        ChatTemplateFamily::ChatMl
    } else if template.contains("<|start_header_id|>")
        && template.contains("<|end_header_id|>")
        && template.contains("<|eot_id|>")
    {
        ChatTemplateFamily::Llama3
    } else if template.contains("[INST]") && template.contains("[/INST]") {
        ChatTemplateFamily::Llama2
    } else if template.contains("<|user|>")
        && template.contains("<|assistant|>")
        && template.contains("<|end|>")
    {
        ChatTemplateFamily::Phi3
    } else {
        ChatTemplateFamily::Fallback
    }
}

fn render_chatml(
    template: &str,
    messages: &[ChatMessage],
    tool_prompt_suffix: Option<&str>,
) -> Result<String, OpenAiHttpError> {
    let qwen_family = template.contains(QWEN_DEFAULT_SYSTEM);
    let default_system = if qwen_family {
        Some(QWEN_DEFAULT_SYSTEM)
    } else if template.contains(SMOLLM2_DEFAULT_SYSTEM) {
        Some(SMOLLM2_DEFAULT_SYSTEM)
    } else {
        None
    };
    if tool_prompt_suffix.is_some() && !qwen_family {
        return Err(OpenAiHttpError::invalid_request_with_param(
            "function tools require a Qwen-compatible ChatML template",
            "tools",
        ));
    }
    let mut prompt = String::new();
    let mut first_message = 0;
    if let Some(default_system) = default_system {
        let system = messages
            .first()
            .filter(|message| message.role() == ChatRole::System)
            .map_or(default_system, ChatMessage::content);
        if let Some(tool_prompt_suffix) = tool_prompt_suffix {
            push_bounded(&mut prompt, "<|im_start|>system\n")?;
            push_bounded(&mut prompt, system)?;
            push_bounded(&mut prompt, tool_prompt_suffix)?;
            push_bounded(&mut prompt, "<|im_end|>\n")?;
        } else {
            push_chatml_message(&mut prompt, "system", system)?;
        }
        if messages
            .first()
            .is_some_and(|message| message.role() == ChatRole::System)
        {
            first_message = 1;
        }
    }

    let mut index = first_message;
    while index < messages.len() {
        let message = &messages[index];
        if message.role() == ChatRole::Assistant && !message.tool_calls().is_empty() && qwen_family
        {
            push_chatml_tool_call_message(&mut prompt, message)?;
            index += 1;
            continue;
        }
        if matches!(message.role(), ChatRole::Tool | ChatRole::Function) && qwen_family {
            push_bounded(&mut prompt, "<|im_start|>user")?;
            while index < messages.len()
                && matches!(messages[index].role(), ChatRole::Tool | ChatRole::Function)
            {
                push_bounded(&mut prompt, "\n<tool_response>\n")?;
                push_bounded(&mut prompt, messages[index].content())?;
                push_bounded(&mut prompt, "\n</tool_response>")?;
                index += 1;
            }
            push_bounded(&mut prompt, "<|im_end|>\n")?;
            continue;
        }
        push_chatml_message(&mut prompt, role_label(message.role()), message.content())?;
        index += 1;
    }
    push_bounded(&mut prompt, "<|im_start|>assistant\n")?;
    Ok(prompt)
}

fn push_chatml_tool_call_message(
    prompt: &mut String,
    message: &ChatMessage,
) -> Result<(), OpenAiHttpError> {
    push_bounded(prompt, "<|im_start|>assistant")?;
    if !message.content().is_empty() {
        push_bounded(prompt, "\n")?;
        push_bounded(prompt, message.content())?;
    }
    for tool_call in message.tool_calls() {
        let name = serde_json::to_string(tool_call.name()).map_err(|error| {
            OpenAiHttpError::invalid_request_with_param(
                format!("failed to serialize tool call name: {error}"),
                "messages.tool_calls",
            )
        })?;
        let arguments = tool_call.arguments_value().map_err(|error| {
            OpenAiHttpError::invalid_request_with_param(error.to_string(), "messages.tool_calls")
        })?;
        let arguments = serde_json::to_string(&arguments).map_err(|error| {
            OpenAiHttpError::invalid_request_with_param(
                format!("failed to serialize tool call arguments: {error}"),
                "messages.tool_calls",
            )
        })?;
        push_bounded(prompt, "\n<tool_call>\n{\"name\": ")?;
        push_bounded(prompt, &name)?;
        push_bounded(prompt, ", \"arguments\": ")?;
        push_bounded(prompt, &arguments)?;
        push_bounded(prompt, "}\n</tool_call>")?;
    }
    push_bounded(prompt, "<|im_end|>\n")
}

fn push_chatml_message(
    prompt: &mut String,
    role: &str,
    content: &str,
) -> Result<(), OpenAiHttpError> {
    push_bounded(prompt, "<|im_start|>")?;
    push_bounded(prompt, role)?;
    push_bounded(prompt, "\n")?;
    push_bounded(prompt, content)?;
    push_bounded(prompt, "<|im_end|>\n")
}

fn render_llama3(
    template: &str,
    bos_token: Option<&str>,
    messages: &[ChatMessage],
) -> Result<String, OpenAiHttpError> {
    let mut prompt = String::new();
    if template.contains("bos_token") || template.contains("<|begin_of_text|>") {
        push_bounded(&mut prompt, bos_token.unwrap_or("<|begin_of_text|>"))?;
    }
    for message in messages {
        push_bounded(&mut prompt, "<|start_header_id|>")?;
        push_bounded(&mut prompt, role_label(message.role()))?;
        push_bounded(&mut prompt, "<|end_header_id|>\n\n")?;
        push_bounded(&mut prompt, message.content())?;
        push_bounded(&mut prompt, "<|eot_id|>")?;
    }
    push_bounded(
        &mut prompt,
        "<|start_header_id|>assistant<|end_header_id|>\n\n",
    )?;
    Ok(prompt)
}

fn render_llama2(
    template: &str,
    bos_token: Option<&str>,
    messages: &[ChatMessage],
) -> Result<String, OpenAiHttpError> {
    let bos_token = if template.contains("bos_token") || template.contains("<s>") {
        bos_token.unwrap_or("<s>")
    } else {
        ""
    };
    let eos_token = if template.contains("eos_token") || template.contains("</s>") {
        "</s>"
    } else {
        ""
    };
    let mut prompt = String::new();
    let mut pending_instruction = String::new();
    let mut index = 0;
    if let Some(system) = messages
        .first()
        .filter(|message| message.role() == ChatRole::System)
    {
        push_bounded(&mut pending_instruction, "<<SYS>>\n")?;
        push_bounded(&mut pending_instruction, system.content())?;
        push_bounded(&mut pending_instruction, "\n<</SYS>>\n\n")?;
        index = 1;
    }

    for message in &messages[index..] {
        if message.role() == ChatRole::Assistant {
            if !pending_instruction.is_empty() {
                push_llama2_instruction(&mut prompt, bos_token, &pending_instruction)?;
                pending_instruction.clear();
            }
            push_bounded(&mut prompt, " ")?;
            push_bounded(&mut prompt, message.content())?;
            push_bounded(&mut prompt, " ")?;
            push_bounded(&mut prompt, eos_token)?;
        } else {
            if !pending_instruction.is_empty() && !pending_instruction.ends_with("\n\n") {
                push_bounded(&mut pending_instruction, "\n")?;
            }
            if !matches!(message.role(), ChatRole::User | ChatRole::System) {
                push_bounded(&mut pending_instruction, role_label(message.role()))?;
                push_bounded(&mut pending_instruction, ": ")?;
            }
            push_bounded(&mut pending_instruction, message.content())?;
        }
    }
    if !pending_instruction.is_empty() {
        push_llama2_instruction(&mut prompt, bos_token, &pending_instruction)?;
    }
    Ok(prompt)
}

fn render_phi3(
    template: &str,
    bos_token: Option<&str>,
    messages: &[ChatMessage],
) -> Result<String, OpenAiHttpError> {
    let mut prompt = String::new();
    if template.contains("bos_token") || template.contains("<s>") {
        push_bounded(&mut prompt, bos_token.unwrap_or("<s>"))?;
    }
    for message in messages {
        match message.role() {
            ChatRole::User => {
                push_bounded(&mut prompt, "<|user|>\n")?;
                push_bounded(&mut prompt, message.content())?;
                push_bounded(&mut prompt, "<|end|>\n<|assistant|>\n")?;
            }
            ChatRole::Assistant => {
                push_bounded(&mut prompt, message.content())?;
                push_bounded(&mut prompt, "<|end|>\n")?;
            }
            role => {
                return Err(OpenAiHttpError::invalid_request_with_param(
                    format!(
                        "Phi-3 model chat template does not support {} messages",
                        role_label(role)
                    ),
                    "messages",
                ));
            }
        }
    }
    Ok(prompt)
}

fn push_llama2_instruction(
    prompt: &mut String,
    bos_token: &str,
    instruction: &str,
) -> Result<(), OpenAiHttpError> {
    push_bounded(prompt, bos_token)?;
    push_bounded(prompt, "[INST] ")?;
    push_bounded(prompt, instruction)?;
    push_bounded(prompt, " [/INST]")
}

fn render_fallback(messages: &[ChatMessage]) -> Result<String, OpenAiHttpError> {
    let mut prompt = String::new();
    for message in messages {
        push_bounded(&mut prompt, role_label(message.role()))?;
        push_bounded(&mut prompt, ": ")?;
        push_bounded(&mut prompt, message.content())?;
        push_bounded(&mut prompt, "\n")?;
    }
    push_bounded(&mut prompt, "assistant: ")?;
    Ok(prompt)
}

fn push_bounded(output: &mut String, value: &str) -> Result<(), OpenAiHttpError> {
    let next_len = output.len().checked_add(value.len()).ok_or_else(|| {
        OpenAiHttpError::invalid_request_with_param("rendered chat prompt is too large", "messages")
    })?;
    if next_len > MAX_RENDERED_PROMPT_BYTES {
        return Err(OpenAiHttpError::invalid_request_with_param(
            format!("rendered chat prompt exceeds {MAX_RENDERED_PROMPT_BYTES} bytes"),
            "messages",
        ));
    }
    output.try_reserve(value.len()).map_err(|error| {
        OpenAiHttpError::invalid_request_with_param(
            format!("failed to reserve rendered chat prompt capacity: {error}"),
            "messages",
        )
    })?;
    output.push_str(value);
    Ok(())
}

fn role_label(role: ChatRole) -> &'static str {
    match role {
        ChatRole::Developer => "developer",
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
        ChatRole::Function => "function",
        ChatRole::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_fixtures::{
        LLAMA2_INSTRUCT_CHAT_TEMPLATE, LLAMA3_INSTRUCT_CHAT_TEMPLATE, PHI3_INSTRUCT_CHAT_TEMPLATE,
        QWEN2_5_INSTRUCT_CHAT_TEMPLATE, SMOLLM2_INSTRUCT_CHAT_TEMPLATE,
    };

    #[test]
    fn renders_messages_into_stable_fallback_prompt() -> Result<(), Box<dyn std::error::Error>> {
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

    #[test]
    fn renders_qwen_chatml_special_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let messages = [
            ChatMessage::new(ChatRole::System, "Be brief"),
            ChatMessage::new(ChatRole::User, "Hello"),
        ];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(QWEN2_5_INSTRUCT_CHAT_TEMPLATE),
                None,
                &messages,
            )?,
            "<|im_start|>system\nBe brief<|im_end|>\n<|im_start|>user\nHello<|im_end|>\n<|im_start|>assistant\n"
        );
        Ok(())
    }

    #[test]
    fn renders_qwen_tool_definitions_inside_the_system_message()
    -> Result<(), Box<dyn std::error::Error>> {
        let messages = [ChatMessage::new(ChatRole::User, "Weather in Paris?")];
        let suffix = "\n\n# Tools\n<tools>\n{\"type\":\"function\"}\n</tools>";

        let prompt = render_chat_prompt_with_model_template_and_tools(
            Some(QWEN2_5_INSTRUCT_CHAT_TEMPLATE),
            None,
            &messages,
            Some(suffix),
        )?;

        assert!(prompt.starts_with(
            "<|im_start|>system\nYou are Qwen, created by Alibaba Cloud. You are a helpful assistant.\n\n# Tools"
        ));
        assert!(prompt.contains("<tools>\n{\"type\":\"function\"}\n</tools><|im_end|>"));
        assert!(prompt.ends_with("<|im_start|>assistant\n"));
        Ok(())
    }

    #[test]
    fn renders_qwen_tool_call_and_tool_response_history() -> Result<(), Box<dyn std::error::Error>>
    {
        let messages: Vec<ChatMessage> = serde_json::from_str(
            r#"[
                {"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"lookup","arguments":"{\"q\":\"rust\"}"}}]},
                {"role":"tool","tool_call_id":"call_1","content":"{\"result\":\"ok\"}"},
                {"role":"user","content":"summarize"}
            ]"#,
        )?;
        validate_chat_messages(&messages)?;

        let prompt = render_chat_prompt_with_model_template(
            Some(QWEN2_5_INSTRUCT_CHAT_TEMPLATE),
            None,
            &messages,
        )?;

        assert!(prompt.contains(
            "<|im_start|>assistant\n<tool_call>\n{\"name\": \"lookup\", \"arguments\": {\"q\":\"rust\"}}\n</tool_call><|im_end|>"
        ));
        assert!(prompt.contains(
            "<|im_start|>user\n<tool_response>\n{\"result\":\"ok\"}\n</tool_response><|im_end|>"
        ));
        Ok(())
    }

    #[test]
    fn qwen_template_supplies_its_model_default_system_message()
    -> Result<(), Box<dyn std::error::Error>> {
        let messages = [ChatMessage::new(ChatRole::User, "Hello")];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(QWEN2_5_INSTRUCT_CHAT_TEMPLATE),
                None,
                &messages,
            )?,
            format!(
                "<|im_start|>system\n{QWEN_DEFAULT_SYSTEM}<|im_end|>\n<|im_start|>user\nHello<|im_end|>\n<|im_start|>assistant\n"
            )
        );
        Ok(())
    }

    #[test]
    fn smollm2_template_supplies_its_model_default_system_message()
    -> Result<(), Box<dyn std::error::Error>> {
        let messages = [ChatMessage::new(ChatRole::User, "hello world")];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(SMOLLM2_INSTRUCT_CHAT_TEMPLATE),
                None,
                &messages,
            )?,
            format!(
                "<|im_start|>system\n{SMOLLM2_DEFAULT_SYSTEM}<|im_end|>\n<|im_start|>user\nhello world<|im_end|>\n<|im_start|>assistant\n"
            )
        );
        Ok(())
    }

    #[test]
    fn renders_llama3_headers_with_model_bos_token() -> Result<(), Box<dyn std::error::Error>> {
        let messages = [ChatMessage::new(ChatRole::User, "Hello")];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(LLAMA3_INSTRUCT_CHAT_TEMPLATE),
                Some("<|begin_of_text|>"),
                &messages,
            )?,
            "<|begin_of_text|><|start_header_id|>user<|end_header_id|>\n\nHello<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
        );
        Ok(())
    }

    #[test]
    fn renders_llama2_instruction_turns() -> Result<(), Box<dyn std::error::Error>> {
        let messages = [
            ChatMessage::new(ChatRole::System, "Be brief"),
            ChatMessage::new(ChatRole::User, "Hello"),
        ];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(LLAMA2_INSTRUCT_CHAT_TEMPLATE),
                Some("<s>"),
                &messages,
            )?,
            "<s>[INST] <<SYS>>\nBe brief\n<</SYS>>\n\nHello [/INST]"
        );
        Ok(())
    }

    #[test]
    fn renders_phi3_fused_turn_template() -> Result<(), Box<dyn std::error::Error>> {
        let messages = [
            ChatMessage::new(ChatRole::User, "Hello"),
            ChatMessage::new(ChatRole::Assistant, "Hi"),
            ChatMessage::new(ChatRole::User, "Be brief"),
        ];

        assert_eq!(
            render_chat_prompt_with_model_template(
                Some(PHI3_INSTRUCT_CHAT_TEMPLATE),
                Some("<s>"),
                &messages,
            )?,
            "<s><|user|>\nHello<|end|>\n<|assistant|>\nHi<|end|>\n<|user|>\nBe brief<|end|>\n<|assistant|>\n"
        );
        Ok(())
    }

    #[test]
    fn phi3_template_rejects_roles_it_would_otherwise_drop()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = render_chat_prompt_with_model_template(
            Some(PHI3_INSTRUCT_CHAT_TEMPLATE),
            Some("<s>"),
            &[ChatMessage::new(ChatRole::System, "Be brief")],
        );
        let Err(error) = result else {
            return Err("Phi-3 system message should be rejected explicitly".into());
        };
        assert!(
            error
                .to_string()
                .contains("does not support system messages")
        );
        Ok(())
    }

    #[test]
    fn unrecognized_model_template_uses_documented_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let messages = [ChatMessage::new(ChatRole::User, "Hello")];

        assert_eq!(
            render_chat_prompt_with_model_template(Some("{{ unsupported }}"), None, &messages)?,
            "user: Hello\nassistant: "
        );
        Ok(())
    }

    #[test]
    fn oversized_model_template_uses_bounded_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let template = "x".repeat(MAX_TEMPLATE_BYTES + 1);
        let messages = [ChatMessage::new(ChatRole::User, "Hello")];

        assert_eq!(
            render_chat_prompt_with_model_template(Some(&template), None, &messages)?,
            "user: Hello\nassistant: "
        );
        Ok(())
    }

    #[test]
    fn oversized_rendered_prompt_is_rejected_before_copying_message()
    -> Result<(), Box<dyn std::error::Error>> {
        let content = "x".repeat(MAX_RENDERED_PROMPT_BYTES);
        let messages = [ChatMessage::new(ChatRole::User, content)];

        let error = match render_chat_prompt(&messages) {
            Ok(_) => return Err("rendered prompt beyond the byte limit should be rejected".into()),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            format!("rendered chat prompt exceeds {MAX_RENDERED_PROMPT_BYTES} bytes")
        );
        Ok(())
    }
}
