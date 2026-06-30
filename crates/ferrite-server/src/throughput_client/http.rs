use super::{
    OpenAiEndpoint, StreamingFinishSummary, StreamingTimingSummary, StreamingUsageSummary,
};
use std::{
    error::Error,
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Clone, Debug)]
pub struct OpenAiHttpResponse {
    raw: String,
    streaming_finish: Option<StreamingFinishSummary>,
    streaming_timing: Option<StreamingTimingSummary>,
    streaming_usage: Option<StreamingUsageSummary>,
}

impl OpenAiHttpResponse {
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn streaming_finish(&self) -> Option<StreamingFinishSummary> {
        self.streaming_finish.clone()
    }

    pub fn streaming_timing(&self) -> Option<StreamingTimingSummary> {
        self.streaming_timing
    }

    pub fn streaming_usage(&self) -> Option<StreamingUsageSummary> {
        self.streaming_usage
    }
}

pub async fn send_openai_request(
    addr: SocketAddr,
    api_key: &str,
    path: &str,
    body: &[u8],
) -> Result<OpenAiHttpResponse, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = format!(
        "POST {path} HTTP/1.1\r\n\
Host: {addr}\r\n\
Authorization: Bearer {api_key}\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(body).await?;
    let (response, streaming_timing) = read_http_response(&mut stream).await?;
    let raw = String::from_utf8(response)?;
    let streaming_usage = raw
        .split_once("\r\n\r\n")
        .and_then(|(_, body)| StreamingUsageSummary::from_sse_body(body));
    let streaming_finish = raw
        .split_once("\r\n\r\n")
        .and_then(|(_, body)| StreamingFinishSummary::from_sse_body(body));
    Ok(OpenAiHttpResponse {
        raw,
        streaming_finish,
        streaming_timing,
        streaming_usage,
    })
}

pub fn validate_openai_response(
    endpoint: OpenAiEndpoint,
    stream: bool,
    expect_stream_usage: bool,
    response: &str,
) -> Result<(), Box<dyn Error>> {
    if !response.starts_with("HTTP/1.1 200 OK") {
        return Err(format!("unexpected response: {response}").into());
    }
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    if stream {
        return validate_streaming_response(headers, body, expect_stream_usage);
    }
    let body: serde_json::Value = serde_json::from_str(body)?;
    match endpoint {
        OpenAiEndpoint::Completions => validate_completion_body(&body),
        OpenAiEndpoint::ChatCompletions => validate_chat_completion_body(&body),
    }
}

fn validate_completion_body(body: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    if body["object"] != "text_completion" {
        return Err(format!("unexpected completion object: {}", body["object"]).into());
    }
    if !body["choices"][0]["text"].is_string() {
        return Err("missing completion text".into());
    }
    Ok(())
}

fn validate_chat_completion_body(body: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    if body["object"] != "chat.completion" {
        return Err(format!("unexpected chat completion object: {}", body["object"]).into());
    }
    if !body["choices"][0]["message"]["content"].is_string() {
        return Err("missing chat completion message content".into());
    }
    Ok(())
}

fn validate_streaming_response(
    headers: &str,
    body: &str,
    expect_stream_usage: bool,
) -> Result<(), Box<dyn Error>> {
    let content_type = header_value(headers, "content-type").unwrap_or_default();
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    {
        return Err(
            format!("expected text/event-stream content type, got {content_type:?}").into(),
        );
    }
    let done_events = body.lines().filter(|line| *line == "data: [DONE]").count();
    if done_events != 1 {
        return Err(format!("expected exactly one streaming done event, got {done_events}").into());
    }
    let has_json_event = body
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(str::trim)
        .filter(|data| *data != "[DONE]")
        .any(|data| serde_json::from_str::<serde_json::Value>(data).is_ok());
    if !has_json_event {
        return Err("missing streaming JSON data event".into());
    }
    if StreamingFinishSummary::from_sse_body(body).is_none() {
        return Err("missing streaming finish_reason".into());
    }
    if expect_stream_usage && StreamingUsageSummary::from_sse_body(body).is_none() {
        return Err("missing streaming usage chunk".into());
    }
    Ok(())
}

fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|line| {
        let (candidate, value) = line.split_once(':')?;
        candidate.eq_ignore_ascii_case(name).then_some(value.trim())
    })
}

async fn read_http_response(
    stream: &mut TcpStream,
) -> Result<(Vec<u8>, Option<StreamingTimingSummary>), Box<dyn Error>> {
    let mut response = Vec::new();
    let mut content_length = None;
    let mut header_end = None;
    let mut streaming_tracker = StreamingEventTracker::default();
    let read_started = Instant::now();

    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..bytes_read]);
        streaming_tracker.observe(&response, read_started.elapsed());

        if header_end.is_none() {
            if let Some(index) = find_header_end(&response) {
                header_end = Some(index);
                content_length = parse_content_length(&response[..index])?;
            }
        }

        if let (Some(index), Some(length)) = (header_end, content_length) {
            if response.len() >= index + 4 + length {
                break;
            }
        }
    }

    Ok((response, streaming_tracker.summary()))
}

#[cfg(test)]
pub(super) fn streaming_timing_from_response_snapshots<'a>(
    snapshots: impl IntoIterator<Item = (&'a [u8], Duration)>,
) -> Option<StreamingTimingSummary> {
    let mut tracker = StreamingEventTracker::default();
    for (response, offset) in snapshots {
        tracker.observe(response, offset);
    }
    tracker.summary()
}

#[derive(Default)]
struct StreamingEventTracker {
    header_end: Option<usize>,
    seen_token_events: usize,
    event_offsets: Vec<Duration>,
}

impl StreamingEventTracker {
    fn observe(&mut self, response: &[u8], offset: Duration) {
        if self.header_end.is_none() {
            self.header_end = find_header_end(response);
        }
        let Some(header_end) = self.header_end else {
            return;
        };
        let body_start = header_end + 4;
        if response.len() <= body_start {
            return;
        }
        let Ok(body) = std::str::from_utf8(&response[body_start..]) else {
            return;
        };
        let token_events = count_streaming_token_events(body);
        while self.seen_token_events < token_events {
            self.event_offsets.push(offset);
            self.seen_token_events += 1;
        }
    }

    fn summary(&self) -> Option<StreamingTimingSummary> {
        StreamingTimingSummary::from_event_offsets(&self.event_offsets)
    }
}

fn count_streaming_token_events(body: &str) -> usize {
    let mut token_events = 0;
    let mut event_data: Vec<&str> = Vec::new();

    for line in body.lines() {
        if line.is_empty() {
            if event_data
                .iter()
                .any(|data| streaming_event_has_generated_text(data))
            {
                token_events += 1;
            }
            event_data.clear();
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();
            if data != "[DONE]" {
                event_data.push(data);
            }
        }
    }

    token_events
}

fn streaming_event_has_generated_text(data: &str) -> bool {
    let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
        return false;
    };
    let Some(choices) = event.get("choices").and_then(serde_json::Value::as_array) else {
        return false;
    };
    choices.iter().any(choice_has_generated_text)
}

fn choice_has_generated_text(choice: &serde_json::Value) -> bool {
    choice
        .get("text")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| !text.is_empty())
        || choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|content| !content.is_empty())
}

fn find_header_end(response: &[u8]) -> Option<usize> {
    response.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers: &[u8]) -> Result<Option<usize>, Box<dyn Error>> {
    let headers = std::str::from_utf8(headers)?;
    for line in headers.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                return Ok(Some(value.trim().parse()?));
            }
        }
    }
    Ok(None)
}
