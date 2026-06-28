mod support;

use serde_json::Value;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[tokio::test]
async fn live_http_server_accepts_openai_style_chat_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":1}}"#,
        support::MODEL_ID
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    let body: Value = serde_json::from_str(body)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], support::MODEL_ID);
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

async fn send_http_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\n\
Host: {addr}\r\n\
Authorization: Bearer local-test\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(body).await?;

    let response = read_http_response(&mut stream).await?;
    Ok(String::from_utf8(response)?)
}

async fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

fn parse_content_length(headers: &[u8]) -> Result<Option<usize>, Box<dyn std::error::Error>> {
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
