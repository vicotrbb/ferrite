use ferrite_fixtures::{
    scalar_llama_f32_gguf_fixture, scalar_llama_f32_gguf_fixture_with_eos_token_id,
    scalar_llama_f32_gguf_fixture_with_eot_token_id, scalar_llama_q4_k_gguf_fixture,
};
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn cli_binary() -> Result<OsString, Box<dyn Error>> {
    std::env::var_os("CARGO_BIN_EXE_ferrite").ok_or_else(|| "missing CARGO_BIN_EXE_ferrite".into())
}

pub(crate) fn write_fixture_model() -> Result<PathBuf, Box<dyn Error>> {
    write_fixture_model_bytes(scalar_llama_f32_gguf_fixture())
}

pub(crate) fn write_q4_k_fixture_model() -> Result<PathBuf, Box<dyn Error>> {
    write_fixture_model_bytes(scalar_llama_q4_k_gguf_fixture())
}

pub(crate) fn write_fixture_model_with_eos_token_id(
    eos_token_id: u64,
) -> Result<PathBuf, Box<dyn Error>> {
    write_fixture_model_bytes(scalar_llama_f32_gguf_fixture_with_eos_token_id(
        eos_token_id,
    ))
}

pub(crate) fn write_fixture_model_with_eot_token_id(
    eot_token_id: u64,
) -> Result<PathBuf, Box<dyn Error>> {
    write_fixture_model_bytes(scalar_llama_f32_gguf_fixture_with_eot_token_id(
        eot_token_id,
    ))
}

fn write_fixture_model_bytes(bytes: Vec<u8>) -> Result<PathBuf, Box<dyn Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-cli-fixture-{}-{}.gguf",
        std::process::id(),
        unique_suffix()
    ));
    fs::write(&path, bytes)?;
    Ok(path)
}

pub(crate) fn remove_fixture_model(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn unique_suffix() -> u128 {
    u128::from(FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed))
}
