use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

const MAX_TOOLS: usize = 64;
const MAX_TOOL_NAME_BYTES: usize = 64;
const MAX_TOOL_DESCRIPTION_BYTES: usize = 4 * 1024;
const MAX_TOOL_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_TOOL_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_TOOL_JSON_DEPTH: usize = 32;
const MAX_TOOL_JSON_NODES: usize = 4_096;
const MAX_PARSED_TOOL_CALLS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolConfiguration {
    definitions: Vec<ToolDefinition>,
    choice: ToolChoice,
    parallel: bool,
}

impl ToolConfiguration {
    pub(crate) fn from_request(
        tools: &Option<Value>,
        tool_choice: &Option<Value>,
        parallel_tool_calls: &Option<Value>,
    ) -> Result<Self, ToolOptionError> {
        let definitions = parse_definitions(tools)?;
        let choice = parse_choice(tool_choice, definitions.is_empty())?;
        let parallel = match parallel_tool_calls {
            None => true,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                return Err(ToolOptionError::new(
                    "parallel_tool_calls must be a boolean",
                    "parallel_tool_calls",
                ));
            }
        };

        if let ToolChoice::Specific(name) = &choice {
            if !definitions
                .iter()
                .any(|definition| definition.function.name == *name)
            {
                return Err(ToolOptionError::new(
                    format!("tool_choice names unknown function {name:?}"),
                    "tool_choice",
                ));
            }
        }

        Ok(Self {
            definitions,
            choice,
            parallel,
        })
    }

    pub(crate) fn enabled(&self) -> bool {
        !self.definitions.is_empty() && self.choice != ToolChoice::None
    }

    pub(crate) fn prompt_suffix(&self) -> Result<Option<String>, ToolOptionError> {
        if !self.enabled() {
            return Ok(None);
        }
        let definitions = match &self.choice {
            ToolChoice::Specific(name) => self
                .definitions
                .iter()
                .filter(|definition| definition.function.name == *name)
                .collect::<Vec<_>>(),
            _ => self.definitions.iter().collect::<Vec<_>>(),
        };
        let mut output = String::from(
            "\n\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>",
        );
        for definition in definitions {
            output.push('\n');
            output.push_str(&serde_json::to_string(definition).map_err(|error| {
                ToolOptionError::new(
                    format!("failed to serialize tool definition: {error}"),
                    "tools",
                )
            })?);
        }
        output.push_str(
            "\n</tools>\n\nFor each function call, return a JSON object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>",
        );
        match &self.choice {
            ToolChoice::Required => output.push_str(
                "\nYou must call at least one provided function instead of answering directly.",
            ),
            ToolChoice::Specific(name) => output.push_str(&format!(
                "\nYou must call the {name} function instead of answering directly."
            )),
            ToolChoice::None | ToolChoice::Auto => {}
        }
        Ok(Some(output))
    }

    pub(crate) fn parse_output(
        &self,
        output: &str,
    ) -> Result<ParsedAssistantOutput, ToolParseError> {
        if !self.enabled() {
            return Ok(ParsedAssistantOutput::text(output.to_owned()));
        }
        let parsed = parse_tool_call_tags(output, self)?;
        if parsed.tool_calls.is_empty()
            && matches!(self.choice, ToolChoice::Required | ToolChoice::Specific(_))
        {
            return Err(ToolParseError::new(
                "model did not return the tool call required by tool_choice",
            ));
        }
        Ok(parsed)
    }

    fn allows_name(&self, name: &str) -> bool {
        let defined = self
            .definitions
            .iter()
            .any(|definition| definition.function.name == name);
        defined
            && match &self.choice {
                ToolChoice::Specific(required) => required == name,
                ToolChoice::None => false,
                ToolChoice::Auto | ToolChoice::Required => true,
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ToolChoice {
    None,
    Auto,
    Required,
    Specific(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolDefinition {
    #[serde(rename = "type")]
    kind: String,
    function: FunctionDefinition,
}

impl ToolDefinition {
    fn validate(&self) -> Result<(), ToolOptionError> {
        if self.kind != "function" {
            return Err(ToolOptionError::new(
                "tools currently support only type function",
                "tools",
            ));
        }
        self.function.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FunctionDefinition {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    strict: Option<bool>,
}

impl FunctionDefinition {
    fn validate(&self) -> Result<(), ToolOptionError> {
        validate_name(&self.name, "tools")?;
        if self
            .description
            .as_ref()
            .is_some_and(|description| description.len() > MAX_TOOL_DESCRIPTION_BYTES)
        {
            return Err(ToolOptionError::new(
                format!("tool description exceeds {MAX_TOOL_DESCRIPTION_BYTES} bytes"),
                "tools",
            ));
        }
        if !self.parameters.is_object() {
            return Err(ToolOptionError::new(
                "tool function parameters must be a JSON Schema object",
                "tools",
            ));
        }
        validate_bounded_json(
            &self.parameters,
            MAX_TOOL_SCHEMA_BYTES,
            "tool function parameters",
            "tools",
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecificToolChoice {
    #[serde(rename = "type")]
    kind: String,
    function: SpecificFunctionChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecificFunctionChoice {
    name: String,
}

fn parse_definitions(value: &Option<Value>) -> Result<Vec<ToolDefinition>, ToolOptionError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or_else(|| {
        ToolOptionError::new("tools must be an array of function definitions", "tools")
    })?;
    if values.len() > MAX_TOOLS {
        return Err(ToolOptionError::new(
            format!("tools contains more than {MAX_TOOLS} definitions"),
            "tools",
        ));
    }

    let mut names = BTreeSet::new();
    let mut definitions = Vec::with_capacity(values.len());
    for value in values {
        let definition: ToolDefinition =
            serde_json::from_value(value.clone()).map_err(|error| {
                ToolOptionError::new(format!("invalid tool definition: {error}"), "tools")
            })?;
        definition.validate()?;
        if !names.insert(definition.function.name.clone()) {
            return Err(ToolOptionError::new(
                format!(
                    "tools contains duplicate function name {:?}",
                    definition.function.name
                ),
                "tools",
            ));
        }
        definitions.push(definition);
    }
    Ok(definitions)
}

fn parse_choice(value: &Option<Value>, no_tools: bool) -> Result<ToolChoice, ToolOptionError> {
    match value {
        None if no_tools => Ok(ToolChoice::None),
        None => Ok(ToolChoice::Auto),
        Some(Value::String(choice)) if choice == "none" => Ok(ToolChoice::None),
        Some(Value::String(choice)) if choice == "auto" => {
            if no_tools {
                Ok(ToolChoice::None)
            } else {
                Ok(ToolChoice::Auto)
            }
        }
        Some(Value::String(choice)) if choice == "required" && !no_tools => {
            Ok(ToolChoice::Required)
        }
        Some(Value::Object(_)) if !no_tools => {
            let choice: SpecificToolChoice = serde_json::from_value(
                value
                    .clone()
                    .ok_or_else(|| ToolOptionError::new("missing tool_choice", "tool_choice"))?,
            )
            .map_err(|error| {
                ToolOptionError::new(format!("invalid tool_choice: {error}"), "tool_choice")
            })?;
            if choice.kind != "function" {
                return Err(ToolOptionError::new(
                    "tool_choice currently supports only type function",
                    "tool_choice",
                ));
            }
            validate_name(&choice.function.name, "tool_choice")?;
            Ok(ToolChoice::Specific(choice.function.name))
        }
        Some(_) if no_tools => Err(ToolOptionError::new(
            "tool_choice requires at least one tool definition",
            "tool_choice",
        )),
        Some(_) => Err(ToolOptionError::new(
            "tool_choice must be none, auto, required, or a named function choice",
            "tool_choice",
        )),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChatToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: ChatToolCallFunction,
}

impl ChatToolCall {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && self.id.len() <= 128
            && self.kind == "function"
            && validate_name(&self.function.name, "messages.tool_calls").is_ok()
            && parse_arguments(&self.function.arguments, "messages.tool_calls").is_ok()
    }

    pub(crate) fn name(&self) -> &str {
        &self.function.name
    }

    pub(crate) fn arguments_value(&self) -> Result<Value, ToolOptionError> {
        parse_arguments(&self.function.arguments, "messages.tool_calls")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedAssistantOutput {
    pub(crate) content: Option<String>,
    pub(crate) tool_calls: Vec<ParsedToolCall>,
}

impl ParsedAssistantOutput {
    fn text(content: String) -> Self {
        Self {
            content: Some(content),
            tool_calls: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedToolCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneratedToolCall {
    name: String,
    arguments: Value,
}

fn parse_tool_call_tags(
    output: &str,
    configuration: &ToolConfiguration,
) -> Result<ParsedAssistantOutput, ToolParseError> {
    const OPEN: &str = "<tool_call>";
    const CLOSE: &str = "</tool_call>";

    if !output.contains(OPEN) && !output.contains(CLOSE) {
        return Ok(ParsedAssistantOutput::text(output.to_owned()));
    }
    let mut content = String::new();
    let mut calls = Vec::new();
    let mut cursor = 0usize;
    while cursor < output.len() {
        let suffix = &output[cursor..];
        let Some(open_offset) = suffix.find(OPEN) else {
            if suffix.contains(CLOSE) {
                return Err(ToolParseError::new(
                    "tool output contains an unmatched closing tag",
                ));
            }
            content.push_str(suffix);
            break;
        };
        let before = &suffix[..open_offset];
        if before.contains(CLOSE) {
            return Err(ToolParseError::new(
                "tool output contains an unmatched closing tag",
            ));
        }
        content.push_str(before);
        let body_start = cursor + open_offset + OPEN.len();
        let remainder = &output[body_start..];
        let close_offset = remainder.find(CLOSE).ok_or_else(|| {
            ToolParseError::new("tool output contains an unterminated tool_call tag")
        })?;
        let body = remainder[..close_offset].trim();
        if body.len() > MAX_TOOL_ARGUMENT_BYTES {
            return Err(ToolParseError::new(format!(
                "tool call exceeds {MAX_TOOL_ARGUMENT_BYTES} bytes"
            )));
        }
        let raw: RawGeneratedToolCall = serde_json::from_str(body).map_err(|error| {
            ToolParseError::new(format!("tool call is not valid JSON: {error}"))
        })?;
        validate_name(&raw.name, "tool output")
            .map_err(|error| ToolParseError::new(error.to_string()))?;
        if !configuration.allows_name(&raw.name) {
            return Err(ToolParseError::new(format!(
                "model called undefined or disallowed function {:?}",
                raw.name
            )));
        }
        let arguments = normalize_generated_arguments(raw.arguments)?;
        calls.push(ParsedToolCall {
            name: raw.name,
            arguments,
        });
        if calls.len() > MAX_PARSED_TOOL_CALLS {
            return Err(ToolParseError::new(format!(
                "model returned more than {MAX_PARSED_TOOL_CALLS} tool calls"
            )));
        }
        if calls.len() > 1 && !configuration.parallel {
            return Err(ToolParseError::new(
                "model returned parallel tool calls when parallel_tool_calls is false",
            ));
        }
        cursor = body_start + close_offset + CLOSE.len();
    }

    let content = content.trim().to_owned();
    Ok(ParsedAssistantOutput {
        content: (!content.is_empty()).then_some(content),
        tool_calls: calls,
    })
}

fn normalize_generated_arguments(value: Value) -> Result<String, ToolParseError> {
    let value = match value {
        Value::String(arguments) => serde_json::from_str::<Value>(&arguments).map_err(|error| {
            ToolParseError::new(format!("tool arguments string is not valid JSON: {error}"))
        })?,
        value => value,
    };
    if !value.is_object() {
        return Err(ToolParseError::new("tool arguments must be a JSON object"));
    }
    validate_bounded_json(
        &value,
        MAX_TOOL_ARGUMENT_BYTES,
        "tool arguments",
        "tool output",
    )
    .map_err(|error| ToolParseError::new(error.to_string()))?;
    serde_json::to_string(&value).map_err(|error| {
        ToolParseError::new(format!("failed to serialize tool arguments: {error}"))
    })
}

fn parse_arguments(arguments: &str, parameter: &'static str) -> Result<Value, ToolOptionError> {
    if arguments.len() > MAX_TOOL_ARGUMENT_BYTES {
        return Err(ToolOptionError::new(
            format!("tool arguments exceed {MAX_TOOL_ARGUMENT_BYTES} bytes"),
            parameter,
        ));
    }
    let value: Value = serde_json::from_str(arguments).map_err(|error| {
        ToolOptionError::new(
            format!("tool arguments are not valid JSON: {error}"),
            parameter,
        )
    })?;
    if !value.is_object() {
        return Err(ToolOptionError::new(
            "tool arguments must encode a JSON object",
            parameter,
        ));
    }
    validate_bounded_json(&value, MAX_TOOL_ARGUMENT_BYTES, "tool arguments", parameter)?;
    Ok(value)
}

fn validate_name(name: &str, parameter: &'static str) -> Result<(), ToolOptionError> {
    if name.is_empty()
        || name.len() > MAX_TOOL_NAME_BYTES
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(ToolOptionError::new(
            format!(
                "function name must contain 1 to {MAX_TOOL_NAME_BYTES} ASCII letters, digits, underscores, or hyphens"
            ),
            parameter,
        ));
    }
    Ok(())
}

fn validate_bounded_json(
    value: &Value,
    max_bytes: usize,
    label: &str,
    parameter: &'static str,
) -> Result<(), ToolOptionError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ToolOptionError::new(format!("failed to serialize {label}: {error}"), parameter)
    })?;
    if bytes.len() > max_bytes {
        return Err(ToolOptionError::new(
            format!("{label} exceeds {max_bytes} bytes"),
            parameter,
        ));
    }
    let mut nodes = 0usize;
    validate_json_shape(value, 0, &mut nodes, label, parameter)
}

fn validate_json_shape(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
    label: &str,
    parameter: &'static str,
) -> Result<(), ToolOptionError> {
    *nodes = nodes.saturating_add(1);
    if *nodes > MAX_TOOL_JSON_NODES {
        return Err(ToolOptionError::new(
            format!("{label} exceeds {MAX_TOOL_JSON_NODES} JSON nodes"),
            parameter,
        ));
    }
    if depth > MAX_TOOL_JSON_DEPTH {
        return Err(ToolOptionError::new(
            format!("{label} exceeds nesting depth {MAX_TOOL_JSON_DEPTH}"),
            parameter,
        ));
    }
    match value {
        Value::Array(items) => {
            for item in items {
                validate_json_shape(item, depth + 1, nodes, label, parameter)?;
            }
        }
        Value::Object(object) => {
            for child in object.values() {
                validate_json_shape(child, depth + 1, nodes, label, parameter)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolOptionError {
    message: String,
    parameter: &'static str,
}

impl ToolOptionError {
    fn new(message: impl Into<String>, parameter: &'static str) -> Self {
        Self {
            message: message.into(),
            parameter,
        }
    }

    pub(crate) fn parameter(&self) -> &'static str {
        self.parameter
    }
}

impl fmt::Display for ToolOptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ToolOptionError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolParseError {
    message: String,
}

impl ToolParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ToolParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ToolParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn weather_tools() -> Option<Value> {
        Some(json!([{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the weather",
                "parameters": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"],
                    "additionalProperties": false
                },
                "strict": true
            }
        }]))
    }

    #[test]
    fn validates_and_renders_bounded_function_tools() -> Result<(), Box<dyn std::error::Error>> {
        let configuration = ToolConfiguration::from_request(
            &weather_tools(),
            &Some(json!("auto")),
            &Some(json!(false)),
        )?;

        let prompt = configuration.prompt_suffix()?.ok_or("missing prompt")?;
        assert!(prompt.contains("<tools>"));
        assert!(prompt.contains("\"name\":\"get_weather\""));
        assert!(prompt.contains("<tool_call>"));
        Ok(())
    }

    #[test]
    fn parses_tool_calls_without_executing_them() -> Result<(), Box<dyn std::error::Error>> {
        let configuration =
            ToolConfiguration::from_request(&weather_tools(), &None, &Some(json!(false)))?;
        let parsed = configuration.parse_output(
            "<tool_call>\n{\"name\":\"get_weather\",\"arguments\":{\"city\":\"Paris\"}}\n</tool_call>",
        )?;

        assert_eq!(parsed.content, None);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].name, "get_weather");
        assert_eq!(parsed.tool_calls[0].arguments, r#"{"city":"Paris"}"#);
        Ok(())
    }

    #[test]
    fn rejects_undefined_and_parallel_calls() -> Result<(), Box<dyn std::error::Error>> {
        let configuration =
            ToolConfiguration::from_request(&weather_tools(), &None, &Some(json!(false)))?;
        assert!(configuration
            .parse_output("<tool_call>{\"name\":\"delete_all\",\"arguments\":{}}</tool_call>")
            .is_err());
        assert!(configuration
            .parse_output(
                "<tool_call>{\"name\":\"get_weather\",\"arguments\":{}}</tool_call><tool_call>{\"name\":\"get_weather\",\"arguments\":{}}</tool_call>"
            )
            .is_err());
        Ok(())
    }

    #[test]
    fn rejects_malformed_or_unbounded_tool_definitions() {
        let malformed = Some(json!([{
            "type": "function",
            "function": {"name": "bad name", "parameters": {"type": "object"}}
        }]));
        assert!(ToolConfiguration::from_request(&malformed, &None, &None).is_err());

        let too_many = Some(Value::Array(
            (0..=MAX_TOOLS)
                .map(|index| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": format!("tool_{index}"),
                            "parameters": {"type": "object"}
                        }
                    })
                })
                .collect(),
        ));
        assert!(ToolConfiguration::from_request(&too_many, &None, &None).is_err());
    }
}
