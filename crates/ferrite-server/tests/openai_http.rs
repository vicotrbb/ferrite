use ferrite_server::{runtime::InferenceEngine, state::ServerState};
use serde_json::Value;
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn live_http_server_accepts_openai_style_chat_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
    let addr = listener.local_addr()?;
    let app = ferrite_server::router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let request_body = r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1}"#;
    let response = send_http_request(
        addr,
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    server.abort();
    remove_fixture_model(&model_path)?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    let body: Value = serde_json::from_str(body)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], "fixture-model");
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

fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-http-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(
        &path,
        ferrite_fixtures::scalar_llama_chat_f32_gguf_fixture(),
    )?;
    Ok(path)
}

fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
