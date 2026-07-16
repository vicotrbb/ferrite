use super::LongChatGateConfig;
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::oneshot,
    time::Instant,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongChatQueueProbeResult {
    holder_prompt_cache_key: String,
    contender_prompt_cache_key: String,
    holder_started_streaming: bool,
    holder_completed: bool,
    contender_status: u16,
    contender_completed: bool,
    contender_generated_event: bool,
    contender_admission_latency: Duration,
    contender_time_to_first_generated_event: Duration,
    contender_total_elapsed: Duration,
    max_tokens: usize,
}

impl LongChatQueueProbeResult {
    pub fn new(
        holder_prompt_cache_key: String,
        contender_prompt_cache_key: String,
        max_tokens: usize,
    ) -> Self {
        Self::from_observation(
            holder_prompt_cache_key,
            contender_prompt_cache_key,
            LongChatQueueProbeObservation::completed(),
            max_tokens,
        )
    }

    fn from_observation(
        holder_prompt_cache_key: String,
        contender_prompt_cache_key: String,
        observation: LongChatQueueProbeObservation,
        max_tokens: usize,
    ) -> Self {
        Self {
            holder_prompt_cache_key,
            contender_prompt_cache_key,
            holder_started_streaming: observation.holder_started_streaming,
            holder_completed: observation.holder_completed,
            contender_status: observation.contender_status,
            contender_completed: observation.contender_completed,
            contender_generated_event: observation.contender_generated_event,
            contender_admission_latency: observation.contender_admission_latency,
            contender_time_to_first_generated_event: observation
                .contender_time_to_first_generated_event,
            contender_total_elapsed: observation.contender_total_elapsed,
            max_tokens,
        }
    }

    pub fn holder_prompt_cache_key(&self) -> &str {
        &self.holder_prompt_cache_key
    }

    pub fn contender_prompt_cache_key(&self) -> &str {
        &self.contender_prompt_cache_key
    }

    pub fn holder_started_streaming(&self) -> bool {
        self.holder_started_streaming
    }

    pub fn holder_completed(&self) -> bool {
        self.holder_completed
    }

    pub fn contender_status(&self) -> u16 {
        self.contender_status
    }

    pub fn contender_completed(&self) -> bool {
        self.contender_completed
    }

    pub fn contender_generated_event(&self) -> bool {
        self.contender_generated_event
    }

    pub fn contender_admission_latency(&self) -> Duration {
        self.contender_admission_latency
    }

    pub fn contender_time_to_first_generated_event(&self) -> Duration {
        self.contender_time_to_first_generated_event
    }

    pub fn contender_total_elapsed(&self) -> Duration {
        self.contender_total_elapsed
    }

    pub fn contender_started_after_holder(&self) -> bool {
        self.holder_started_streaming && self.contender_status == 200
    }

    pub fn completed(&self) -> bool {
        self.holder_started_streaming
            && self.holder_completed
            && self.contender_status == 200
            && self.contender_completed
            && self.contender_generated_event
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
}

struct LongChatQueueProbeObservation {
    holder_started_streaming: bool,
    holder_completed: bool,
    contender_status: u16,
    contender_completed: bool,
    contender_generated_event: bool,
    contender_admission_latency: Duration,
    contender_time_to_first_generated_event: Duration,
    contender_total_elapsed: Duration,
}

impl LongChatQueueProbeObservation {
    fn completed() -> Self {
        Self {
            holder_started_streaming: true,
            holder_completed: true,
            contender_status: 200,
            contender_completed: true,
            contender_generated_event: true,
            contender_admission_latency: Duration::ZERO,
            contender_time_to_first_generated_event: Duration::ZERO,
            contender_total_elapsed: Duration::ZERO,
        }
    }
}

impl LongChatGateConfig {
    pub async fn run_queue_probe(&self) -> Result<LongChatQueueProbeResult, Box<dyn Error>> {
        let holder_key = self
            .prompt_cache_keys()
            .first()
            .ok_or("expected holder prompt-cache key")?
            .to_owned();
        let contender_key = self
            .prompt_cache_keys()
            .get(1)
            .ok_or("expected contender prompt-cache key")?
            .to_owned();
        let addr: SocketAddr = self.addr().parse()?;
        let max_tokens = self.queue_probe_max_tokens();
        let holder_body = self.queue_probe_body(&holder_key, max_tokens)?;
        let contender_body = self.queue_probe_body(&contender_key, max_tokens)?;
        let (started_tx, started_rx) = oneshot::channel();
        let holder_api_key = self.api_key().to_owned();

        let holder = tokio::spawn(async move {
            send_chat_stream_probe_with_start_signal(
                addr,
                &holder_api_key,
                &holder_body,
                started_tx,
            )
            .await
        });

        let holder_started_streaming = started_rx.await.unwrap_or(false);
        if !holder_started_streaming {
            return Err("queue probe holder did not start streaming generated content".into());
        }

        let contender = send_chat_stream_probe_timed(addr, self.api_key(), &contender_body).await?;
        let contender_status = http_status(&contender.response)?;
        let contender_completed =
            contender_status == 200 && validate_stream_response(&contender.response).is_ok();
        let contender_generated_event = has_generated_stream_event(&contender.response);
        let holder_completed = holder
            .await
            .map_err(|error| std::io::Error::other(format!("queue holder task failed: {error}")))?
            .map_err(std::io::Error::other)?;
        let result = LongChatQueueProbeResult::from_observation(
            holder_key,
            contender_key,
            LongChatQueueProbeObservation {
                holder_started_streaming,
                holder_completed,
                contender_status,
                contender_completed,
                contender_generated_event,
                contender_admission_latency: contender.admission_latency,
                contender_time_to_first_generated_event: contender.time_to_first_generated_event,
                contender_total_elapsed: contender.total_elapsed,
            },
            max_tokens,
        );

        if !result.completed() {
            return Err(format!(
                "queue probe did not complete: holder_completed={}, contender_status={}, contender_completed={}, contender_generated_event={}",
                result.holder_completed(),
                result.contender_status(),
                result.contender_completed(),
                result.contender_generated_event()
            )
            .into());
        }

        Ok(result)
    }

    fn queue_probe_body(
        &self,
        prompt_cache_key: &str,
        max_tokens: usize,
    ) -> Result<String, Box<dyn Error>> {
        let model = self.models().first().ok_or("expected at least one model")?;
        let mut body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": self.prompt()},
                {"role": "assistant", "content": self.assistant_context()},
                {"role": "user", "content": self.follow_up()}
            ],
            "max_tokens": max_tokens,
            "stream": true,
            "prompt_cache_key": prompt_cache_key,
        });
        if self.prompt_cache_trace() {
            body["metadata"] = serde_json::json!({"ferrite_cache_trace": "true"});
        }
        Ok(body.to_string())
    }

    fn queue_probe_max_tokens(&self) -> usize {
        self.probe_max_tokens().unwrap_or(8)
    }
}

pub fn format_queue_probe_result(result: &LongChatQueueProbeResult) -> String {
    format!(
        "long_chat_queue_probe_holder_prompt_cache_key={}\nlong_chat_queue_probe_contender_prompt_cache_key={}\nlong_chat_queue_probe_holder_started_streaming={}\nlong_chat_queue_probe_holder_completed={}\nlong_chat_queue_probe_contender_status={}\nlong_chat_queue_probe_contender_completed={}\nlong_chat_queue_probe_contender_generated_event={}\nlong_chat_queue_probe_contender_started_after_holder={}\nlong_chat_queue_probe_contender_admission_latency_ms={}\nlong_chat_queue_probe_contender_time_to_first_generated_event_ms={}\nlong_chat_queue_probe_contender_total_elapsed_ms={}\nlong_chat_queue_probe_max_tokens={}",
        result.holder_prompt_cache_key(),
        result.contender_prompt_cache_key(),
        result.holder_started_streaming(),
        result.holder_completed(),
        result.contender_status(),
        result.contender_completed(),
        result.contender_generated_event(),
        result.contender_started_after_holder(),
        result.contender_admission_latency().as_millis(),
        result.contender_time_to_first_generated_event().as_millis(),
        result.contender_total_elapsed().as_millis(),
        result.max_tokens()
    )
}

struct TimedStreamResponse {
    response: String,
    admission_latency: Duration,
    time_to_first_generated_event: Duration,
    total_elapsed: Duration,
}

async fn send_chat_stream_probe_timed(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
) -> Result<TimedStreamResponse, Box<dyn Error>> {
    let started = Instant::now();
    let mut stream = TcpStream::connect(addr).await?;
    write_chat_stream_request(&mut stream, addr, api_key, body).await?;

    let mut response = Vec::new();
    let mut admission_latency = None;
    let mut time_to_first_generated_event = None;
    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..bytes_read]);
        let text = std::str::from_utf8(&response)?;
        if admission_latency.is_none() && text.contains("\r\n\r\n") {
            admission_latency = Some(started.elapsed());
        }
        if time_to_first_generated_event.is_none() && has_generated_stream_event(text) {
            time_to_first_generated_event = Some(started.elapsed());
        }
    }
    Ok(TimedStreamResponse {
        response: String::from_utf8(response)?,
        admission_latency: admission_latency.ok_or("queue probe response had no headers")?,
        time_to_first_generated_event: time_to_first_generated_event
            .ok_or("queue probe response had no generated event")?,
        total_elapsed: started.elapsed(),
    })
}

async fn send_chat_stream_probe_with_start_signal(
    addr: SocketAddr,
    api_key: &str,
    body: &str,
    started_tx: oneshot::Sender<bool>,
) -> Result<bool, String> {
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|error| error.to_string())?;
    write_chat_stream_request(&mut stream, addr, api_key, body)
        .await
        .map_err(|error| error.to_string())?;

    let mut response = Vec::new();
    let mut started_tx = Some(started_tx);
    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if bytes_read == 0 {
            if let Some(started_tx) = started_tx.take() {
                let _ = started_tx.send(false);
            }
            let response = String::from_utf8(response).map_err(|error| error.to_string())?;
            return validate_stream_response(&response)
                .map(|()| true)
                .map_err(|error| error.to_string());
        }
        response.extend_from_slice(&chunk[..bytes_read]);
        if started_tx.is_some() {
            let text = std::str::from_utf8(&response).map_err(|error| error.to_string())?;
            if has_generated_stream_event(text)
                && let Some(started_tx) = started_tx.take()
            {
                let _ = started_tx.send(true);
            }
        }
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

fn validate_stream_response(response: &str) -> Result<(), Box<dyn Error>> {
    let status = http_status(response)?;
    if status != 200 {
        return Err(format!("expected queue probe status 200, got {status}").into());
    }
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected queue probe response body")?;
    if !headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("content-type: text/event-stream"))
    {
        return Err("expected queue probe response content-type text/event-stream".into());
    }
    if !body.lines().any(|line| line == "data: [DONE]") {
        return Err("expected queue probe response streaming done event".into());
    }
    if !has_generated_stream_event(response) {
        return Err("expected queue probe response generated stream event".into());
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
