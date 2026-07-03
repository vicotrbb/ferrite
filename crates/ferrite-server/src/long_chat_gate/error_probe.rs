use super::LongChatGateConfig;
use std::{error::Error, net::SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatErrorProbeResult {
    unauthorized_status: u16,
    reconnect_completed: bool,
    reconnect_generated_event: bool,
    max_tokens: usize,
}

impl LongChatErrorProbeResult {
    pub fn new(
        unauthorized_status: u16,
        reconnect_completed: bool,
        reconnect_generated_event: bool,
        max_tokens: usize,
    ) -> Self {
        Self {
            unauthorized_status,
            reconnect_completed,
            reconnect_generated_event,
            max_tokens,
        }
    }

    pub fn unauthorized_status(&self) -> u16 {
        self.unauthorized_status
    }

    pub fn reconnect_completed(&self) -> bool {
        self.reconnect_completed
    }

    pub fn reconnect_generated_event(&self) -> bool {
        self.reconnect_generated_event
    }

    pub fn reconnect_started_new_generation(&self) -> bool {
        self.reconnect_completed && self.reconnect_generated_event
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
}

impl LongChatGateConfig {
    pub async fn run_error_probe(&self) -> Result<LongChatErrorProbeResult, Box<dyn Error>> {
        let addr: SocketAddr = self.addr().parse()?;
        let body = self.error_probe_body()?;
        let unauthorized =
            send_chat_stream_probe(addr, "ferrite-invalid-local-secret", &body).await?;
        let unauthorized_status = http_status(&unauthorized)?;
        if unauthorized_status != 401 {
            return Err(format!(
                "expected unauthorized probe status 401, got {unauthorized_status}"
            )
            .into());
        }

        let reconnect = send_chat_stream_probe(addr, self.api_key(), &body).await?;
        validate_reconnect_response(&reconnect)?;
        Ok(LongChatErrorProbeResult::new(
            unauthorized_status,
            true,
            true,
            self.error_probe_max_tokens(),
        ))
    }

    fn error_probe_body(&self) -> Result<String, Box<dyn Error>> {
        let model = self.models().first().ok_or("expected at least one model")?;
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": self.prompt()},
                {"role": "assistant", "content": self.assistant_context()},
                {"role": "user", "content": self.follow_up()}
            ],
            "max_tokens": self.error_probe_max_tokens(),
            "stream": true
        });
        Ok(body.to_string())
    }

    fn error_probe_max_tokens(&self) -> usize {
        self.probe_max_tokens().unwrap_or(1)
    }
}

pub fn format_error_probe_result(result: &LongChatErrorProbeResult) -> String {
    format!(
        "long_chat_error_probe_unauthorized_status={}\nlong_chat_error_probe_reconnect_completed={}\nlong_chat_error_probe_reconnect_generated_event={}\nlong_chat_error_probe_reconnect_started_new_generation={}\nlong_chat_error_probe_max_tokens={}",
        result.unauthorized_status(),
        result.reconnect_completed(),
        result.reconnect_generated_event(),
        result.reconnect_started_new_generation(),
        result.max_tokens()
    )
}

async fn send_chat_stream_probe(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = format!(
        "POST /v1/chat/completions HTTP/1.1\r\n\
Host: {addr}\r\n\
Authorization: Bearer {api_key}\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    Ok(String::from_utf8(response)?)
}

fn http_status(response: &str) -> Result<u16, Box<dyn Error>> {
    let status_line = response.lines().next().ok_or("missing HTTP status line")?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or("missing HTTP status code")?
        .parse()?;
    Ok(status)
}

fn validate_reconnect_response(response: &str) -> Result<(), Box<dyn Error>> {
    let status = http_status(response)?;
    if status != 200 {
        return Err(format!("expected reconnect probe status 200, got {status}").into());
    }
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected reconnect response body")?;
    if !headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("content-type: text/event-stream"))
    {
        return Err("expected reconnect response content-type text/event-stream".into());
    }
    if !body.lines().any(|line| line == "data: [DONE]") {
        return Err("expected reconnect response streaming done event".into());
    }
    if !has_generated_stream_event(response) {
        return Err("expected reconnect response generated stream event".into());
    }
    Ok(())
}

fn has_generated_stream_event(response: &str) -> bool {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(str::trim)
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<serde_json::Value>(data).ok())
        .any(event_has_generated_text)
}

fn event_has_generated_text(event: serde_json::Value) -> bool {
    let Some(choices) = event.get("choices").and_then(serde_json::Value::as_array) else {
        return false;
    };
    choices.iter().any(|choice| {
        choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|content| !content.is_empty())
            || choice
                .get("text")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|text| !text.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reconnect_response_without_generated_event() -> Result<(), Box<dyn Error>> {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: [DONE]\n";

        let error = match validate_reconnect_response(response) {
            Ok(()) => return Err("done-only reconnect response should be rejected".into()),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("expected reconnect response generated stream event"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn accepts_reconnect_response_with_generated_event() -> Result<(), Box<dyn Error>> {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "content-type: text/event-stream\r\n",
            "\r\n",
            r#"data: {"choices":[{"delta":{"content":"ok"}}]}"#,
            "\n",
            "data: [DONE]\n",
        );

        validate_reconnect_response(response)
    }
}
