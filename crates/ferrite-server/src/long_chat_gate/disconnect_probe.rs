use super::LongChatGateConfig;
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatDisconnectProbeResult {
    aborted_after_generated_event: bool,
    reconnect_completed: bool,
}

impl LongChatDisconnectProbeResult {
    pub fn new(aborted_after_generated_event: bool, reconnect_completed: bool) -> Self {
        Self {
            aborted_after_generated_event,
            reconnect_completed,
        }
    }

    pub fn aborted_after_generated_event(&self) -> bool {
        self.aborted_after_generated_event
    }

    pub fn reconnect_completed(&self) -> bool {
        self.reconnect_completed
    }
}

impl LongChatGateConfig {
    pub async fn run_disconnect_probe(
        &self,
    ) -> Result<LongChatDisconnectProbeResult, Box<dyn Error>> {
        let addr: SocketAddr = self.addr().parse()?;
        let abort_body = self.disconnect_probe_body(8)?;
        let aborted_after_generated_event =
            abort_after_generated_event(addr, self.api_key(), &abort_body).await?;
        if !aborted_after_generated_event {
            return Err("disconnect probe did not observe a generated stream event".into());
        }

        let reconnect_body = self.disconnect_probe_body(1)?;
        reconnect_until_completed(addr, self.api_key(), &reconnect_body).await?;
        Ok(LongChatDisconnectProbeResult::new(true, true))
    }

    fn disconnect_probe_body(&self, max_tokens: usize) -> Result<String, Box<dyn Error>> {
        let model = self.models().first().ok_or("expected at least one model")?;
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": self.prompt()},
                {"role": "assistant", "content": self.assistant_context()},
                {"role": "user", "content": self.follow_up()}
            ],
            "max_tokens": max_tokens,
            "stream": true
        });
        Ok(body.to_string())
    }
}

pub fn format_disconnect_probe_result(result: &LongChatDisconnectProbeResult) -> String {
    format!(
        "long_chat_disconnect_probe_aborted_after_generated_event={}\nlong_chat_disconnect_probe_reconnect_completed={}",
        result.aborted_after_generated_event(),
        result.reconnect_completed()
    )
}

async fn abort_after_generated_event(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<bool, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    write_chat_stream_request(&mut stream, addr, api_key, body).await?;

    let mut response = Vec::new();
    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            return Ok(false);
        }
        response.extend_from_slice(&chunk[..bytes_read]);
        let text = std::str::from_utf8(&response)?;
        if has_generated_stream_event(text) {
            return Ok(true);
        }
    }
}

async fn send_chat_stream_probe(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    write_chat_stream_request(&mut stream, addr, api_key, body).await?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    Ok(String::from_utf8(response)?)
}

async fn reconnect_until_completed(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<(), Box<dyn Error>> {
    let mut last_status = None;
    for _ in 0..20 {
        let response = send_chat_stream_probe(addr, api_key, body).await?;
        let status = http_status(&response)?;
        if status == 200 {
            return validate_reconnect_response(&response);
        }
        if !is_retryable_reconnect_status(status) {
            return Err(format!("expected reconnect probe status 200, got {status}").into());
        }
        last_status = Some(status);
        sleep(Duration::from_millis(250)).await;
    }

    Err(format!(
        "reconnect probe did not complete after retryable status {}",
        last_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    )
    .into())
}

async fn write_chat_stream_request(
    stream: &mut TcpStream,
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<(), Box<dyn Error>> {
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
    Ok(())
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
    Ok(())
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

fn is_retryable_reconnect_status(status: u16) -> bool {
    status == 429
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
    fn treats_rate_limited_reconnect_as_retryable() {
        assert!(is_retryable_reconnect_status(429));
    }
}
