use super::LongChatGateConfig;
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Instant, sleep},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatDisconnectProbeResult {
    aborted_after_generated_event: bool,
    reconnect_completed: bool,
    reconnect_attempts: usize,
    disconnect_to_reconnect_admission: Duration,
    disconnect_to_reconnect_first_generated_event: Duration,
    disconnect_to_reconnect_completion: Duration,
    max_tokens: usize,
}

impl LongChatDisconnectProbeResult {
    pub fn new(
        aborted_after_generated_event: bool,
        reconnect_completed: bool,
        max_tokens: usize,
    ) -> Self {
        Self {
            aborted_after_generated_event,
            reconnect_completed,
            reconnect_attempts: usize::from(reconnect_completed),
            disconnect_to_reconnect_admission: Duration::ZERO,
            disconnect_to_reconnect_first_generated_event: Duration::ZERO,
            disconnect_to_reconnect_completion: Duration::ZERO,
            max_tokens,
        }
    }

    fn with_recovery_timing(mut self, timing: ReconnectTiming) -> Self {
        self.reconnect_attempts = timing.attempts;
        self.disconnect_to_reconnect_admission = timing.admission;
        self.disconnect_to_reconnect_first_generated_event = timing.first_generated_event;
        self.disconnect_to_reconnect_completion = timing.completion;
        self
    }

    pub fn aborted_after_generated_event(&self) -> bool {
        self.aborted_after_generated_event
    }

    pub fn reconnect_completed(&self) -> bool {
        self.reconnect_completed
    }

    pub fn reconnect_generated_event(&self) -> bool {
        self.reconnect_completed
    }

    pub fn reconnect_started_new_generation(&self) -> bool {
        self.aborted_after_generated_event && self.reconnect_generated_event()
    }

    pub fn reconnect_attempts(&self) -> usize {
        self.reconnect_attempts
    }

    pub fn disconnect_to_reconnect_admission(&self) -> Duration {
        self.disconnect_to_reconnect_admission
    }

    pub fn disconnect_to_reconnect_first_generated_event(&self) -> Duration {
        self.disconnect_to_reconnect_first_generated_event
    }

    pub fn disconnect_to_reconnect_completion(&self) -> Duration {
        self.disconnect_to_reconnect_completion
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
}

impl LongChatGateConfig {
    pub async fn run_disconnect_probe(
        &self,
    ) -> Result<LongChatDisconnectProbeResult, Box<dyn Error>> {
        let addr: SocketAddr = self.addr().parse()?;
        let max_tokens = self.disconnect_probe_max_tokens();
        let abort_body = self.disconnect_probe_body(max_tokens)?;
        let aborted_after_generated_event =
            abort_after_generated_event(addr, self.api_key(), &abort_body).await?;
        if !aborted_after_generated_event {
            return Err("disconnect probe did not observe a generated stream event".into());
        }

        let reconnect_body = self.disconnect_probe_body(max_tokens)?;
        let disconnected_at = Instant::now();
        let recovery_timing = reconnect_until_completed(
            addr,
            self.api_key(),
            &reconnect_body,
            self.disconnect_reconnect_timeout(),
            disconnected_at,
        )
        .await?;
        Ok(LongChatDisconnectProbeResult::new(true, true, max_tokens)
            .with_recovery_timing(recovery_timing))
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

    fn disconnect_probe_max_tokens(&self) -> usize {
        self.probe_max_tokens().unwrap_or(8)
    }
}

pub fn format_disconnect_probe_result(result: &LongChatDisconnectProbeResult) -> String {
    format!(
        "long_chat_disconnect_probe_aborted_after_generated_event={}\nlong_chat_disconnect_probe_reconnect_completed={}\nlong_chat_disconnect_probe_reconnect_generated_event={}\nlong_chat_disconnect_probe_reconnect_started_new_generation={}\nlong_chat_disconnect_probe_reconnect_attempts={}\nlong_chat_disconnect_probe_disconnect_to_reconnect_admission_ms={}\nlong_chat_disconnect_probe_disconnect_to_reconnect_first_generated_event_ms={}\nlong_chat_disconnect_probe_disconnect_to_reconnect_completion_ms={}\nlong_chat_disconnect_probe_max_tokens={}",
        result.aborted_after_generated_event(),
        result.reconnect_completed(),
        result.reconnect_generated_event(),
        result.reconnect_started_new_generation(),
        result.reconnect_attempts(),
        result.disconnect_to_reconnect_admission().as_millis(),
        result
            .disconnect_to_reconnect_first_generated_event()
            .as_millis(),
        result.disconnect_to_reconnect_completion().as_millis(),
        result.max_tokens()
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

struct TimedStreamResponse {
    response: String,
    headers_at: Instant,
    first_generated_event_at: Option<Instant>,
    completed_at: Instant,
}

async fn send_chat_stream_probe_timed(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<TimedStreamResponse, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    write_chat_stream_request(&mut stream, addr, api_key, body).await?;

    let mut response = Vec::new();
    let mut headers_at = None;
    let mut first_generated_event_at = None;
    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..bytes_read]);
        let text = std::str::from_utf8(&response)?;
        if headers_at.is_none() && text.contains("\r\n\r\n") {
            headers_at = Some(Instant::now());
        }
        if first_generated_event_at.is_none() && has_generated_stream_event(text) {
            first_generated_event_at = Some(Instant::now());
        }
    }
    Ok(TimedStreamResponse {
        response: String::from_utf8(response)?,
        headers_at: headers_at.ok_or("reconnect response had no headers")?,
        first_generated_event_at,
        completed_at: Instant::now(),
    })
}

struct ReconnectTiming {
    attempts: usize,
    admission: Duration,
    first_generated_event: Duration,
    completion: Duration,
}

async fn reconnect_until_completed(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
    timeout: Duration,
    disconnected_at: Instant,
) -> Result<ReconnectTiming, Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    let mut attempts = 0;
    loop {
        attempts += 1;
        let response = send_chat_stream_probe_timed(addr, api_key, body).await?;
        let status = http_status(&response.response)?;
        if status == 200 {
            validate_reconnect_response(&response.response)?;
            return Ok(ReconnectTiming {
                attempts,
                admission: response.headers_at.duration_since(disconnected_at),
                first_generated_event: response
                    .first_generated_event_at
                    .ok_or("reconnect response had no generated event")?
                    .duration_since(disconnected_at),
                completion: response.completed_at.duration_since(disconnected_at),
            });
        }
        if !is_retryable_reconnect_status(status) {
            return Err(format!("expected reconnect probe status 200, got {status}").into());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "reconnect probe did not complete after retryable status {status}"
            )
            .into());
        }
        sleep(Duration::from_millis(250)).await;
    }
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
    if !has_generated_stream_event(response) {
        return Err("expected reconnect response generated stream event".into());
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

    #[test]
    fn rejects_reconnect_response_without_generated_event() -> Result<(), Box<dyn std::error::Error>>
    {
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
}
