use super::LongChatGateConfig;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct LongChatProofArtifacts {
    log_file: Option<File>,
    exit_code_path: Option<PathBuf>,
}

impl LongChatProofArtifacts {
    pub fn create(config: &LongChatGateConfig) -> io::Result<Self> {
        let log_file = config.proof_log_path().map(open_artifact_log).transpose()?;
        Ok(Self {
            log_file,
            exit_code_path: config.proof_exit_code_path().map(Path::to_path_buf),
        })
    }

    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        let Some(log_file) = &mut self.log_file else {
            return Ok(());
        };
        writeln!(log_file, "{line}")?;
        log_file.flush()
    }

    pub fn write_exit_code(&self, code: i32) -> io::Result<()> {
        let Some(path) = &self.exit_code_path else {
            return Ok(());
        };
        ensure_parent_directory(path)?;
        fs::write(path, format!("{code}\n"))
    }
}

fn open_artifact_log(path: &Path) -> io::Result<File> {
    ensure_parent_directory(path)?;
    File::create(path)
}

fn ensure_parent_directory(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
}
