use super::OpenAiEndpoint;
use std::{error::Error, net::SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub async fn send_openai_request(
    addr: SocketAddr,
    api_key: &str,
    path: &str,
    body: &[u8],
) -> Result<String, Box<dyn Error>> {
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
    Ok(String::from_utf8(read_http_response(&mut stream).await?)?)
}

pub fn validate_openai_response(
    endpoint: OpenAiEndpoint,
    stream: bool,
    response: &str,
) -> Result<(), Box<dyn Error>> {
    if !response.starts_with("HTTP/1.1 200 OK") {
        return Err(format!("unexpected response: {response}").into());
    }
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    if stream {
        return validate_streaming_body(body);
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

fn validate_streaming_body(body: &str) -> Result<(), Box<dyn Error>> {
    if !body.lines().any(|line| line.starts_with("data: ")) {
        return Err("missing streaming data event".into());
    }
    if !body.lines().any(|line| line == "data: [DONE]") {
        return Err("missing streaming done event".into());
    }
    Ok(())
}

async fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut response = Vec::new();
    let mut content_length = None;
    let mut header_end = None;

    loop {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..bytes_read]);

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

    Ok(response)
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
