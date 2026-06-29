use axum::body::{to_bytes, Body};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&to_text(body).await?)?)
}

pub(super) async fn to_text(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

pub(super) fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    write_fixture_model_bytes(ferrite_fixtures::scalar_llama_f32_gguf_fixture())
}

pub(super) fn write_chat_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    write_fixture_model_bytes(ferrite_fixtures::scalar_llama_chat_f32_gguf_fixture())
}

fn write_fixture_model_bytes(
    bytes: Vec<u8>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

pub(super) fn remove_fixture_model(
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
