mod support;

use serde_json::Value;
use std::{net::SocketAddr, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-135m";

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn live_http_server_generates_with_real_tier0_model() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = std::env::var_os("FERRITE_REAL_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!("missing real model artifact: {}", model_path.display()).into());
    }
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["text"], ".");
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 3);
    Ok(())
}

fn default_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_MODEL_PATH)
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

fn response_json(response: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    Ok(serde_json::from_str(body)?)
}
