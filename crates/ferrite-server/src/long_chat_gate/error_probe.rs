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
}

impl LongChatErrorProbeResult {
    pub fn new(unauthorized_status: u16, reconnect_completed: bool) -> Self {
        Self {
            unauthorized_status,
            reconnect_completed,
        }
    }

    pub fn unauthorized_status(&self) -> u16 {
        self.unauthorized_status
    }

    pub fn reconnect_completed(&self) -> bool {
        self.reconnect_completed
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
        Ok(LongChatErrorProbeResult::new(unauthorized_status, true))
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
            "max_tokens": 1,
            "stream": true
        });
        Ok(body.to_string())
    }
}

pub fn format_error_probe_result(result: &LongChatErrorProbeResult) -> String {
    format!(
        "long_chat_error_probe_unauthorized_status={}\nlong_chat_error_probe_reconnect_completed={}",
        result.unauthorized_status(),
        result.reconnect_completed()
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
    Ok(())
}
