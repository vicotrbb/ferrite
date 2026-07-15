use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModelArtifact {
    pub(crate) id: &'static str,
    pub(crate) source: &'static str,
    pub(crate) revision: &'static str,
    pub(crate) license: &'static str,
    pub(crate) license_url: &'static str,
    pub(crate) filename: &'static str,
    pub(crate) size: u64,
    pub(crate) sha256: &'static str,
    pub(crate) download_url: &'static str,
}

pub(crate) const PHI3_MINI_4K_INSTRUCT_Q4: ModelArtifact = ModelArtifact {
    id: "phi3-mini-4k-instruct-q4",
    source: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf",
    revision: "a64113399c2f6b8ad3e11c394733a2ddadaa7f33",
    license: "MIT",
    license_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/blob/a64113399c2f6b8ad3e11c394733a2ddadaa7f33/LICENSE",
    filename: "Phi-3-mini-4k-instruct-q4.gguf",
    size: 2_393_231_072,
    sha256: "8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef",
    download_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/a64113399c2f6b8ad3e11c394733a2ddadaa7f33/Phi-3-mini-4k-instruct-q4.gguf",
};

const BUILTIN_MODELS: &[ModelArtifact] = &[PHI3_MINI_4K_INSTRUCT_Q4];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AcquiredModel {
    path: PathBuf,
    artifact: ModelArtifact,
}

impl AcquiredModel {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn artifact(&self) -> ModelArtifact {
        self.artifact
    }
}

pub(crate) fn builtin_model(id: &str) -> Result<ModelArtifact, Box<dyn Error>> {
    BUILTIN_MODELS
        .iter()
        .find(|artifact| artifact.id == id)
        .copied()
        .ok_or_else(|| {
            io::Error::other(format!(
                "unknown built-in model {id:?}; available models: {}",
                BUILTIN_MODELS
                    .iter()
                    .map(|artifact| artifact.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .into()
        })
}

pub(crate) fn acquire_builtin_model(
    id: &str,
    cache_root: Option<&Path>,
    offline: bool,
) -> Result<AcquiredModel, Box<dyn Error>> {
    let artifact = builtin_model(id)?;
    let cache_root = match cache_root {
        Some(path) => path.to_owned(),
        None => default_cache_root()?,
    };
    acquire_with_downloader(artifact, &cache_root, offline, |partial, artifact| {
        download_with_curl(partial, artifact)
    })
}

fn acquire_with_downloader(
    artifact: ModelArtifact,
    cache_root: &Path,
    offline: bool,
    downloader: impl FnOnce(&Path, ModelArtifact) -> Result<(), Box<dyn Error>>,
) -> Result<AcquiredModel, Box<dyn Error>> {
    let revision_dir = cache_root.join(artifact.id).join(artifact.revision);
    fs::create_dir_all(&revision_dir)?;
    let final_path = revision_dir.join(artifact.filename);
    let metadata_path = revision_dir.join("artifact.json");

    if final_path.exists() {
        reject_symlink(&final_path)?;
        verify_artifact(&final_path, artifact)?;
        set_readonly(&final_path, true)?;
        ensure_metadata(&metadata_path, artifact)?;
        return Ok(AcquiredModel {
            path: final_path,
            artifact,
        });
    }
    if offline {
        return Err(io::Error::other(format!(
            "built-in model {} is not cached at {}; remove --offline to acquire it",
            artifact.id,
            final_path.display()
        ))
        .into());
    }

    let lock_path = revision_dir.join(".acquire.lock");
    let _lock = AcquisitionLock::create(&lock_path)?;
    if final_path.exists() {
        reject_symlink(&final_path)?;
        verify_artifact(&final_path, artifact)?;
        set_readonly(&final_path, true)?;
        ensure_metadata(&metadata_path, artifact)?;
        return Ok(AcquiredModel {
            path: final_path,
            artifact,
        });
    }

    let partial_path = revision_dir.join(format!("{}.partial", artifact.filename));
    if partial_path.exists() {
        reject_symlink(&partial_path)?;
        set_readonly(&partial_path, false)?;
        let partial_size = fs::metadata(&partial_path)?.len();
        if partial_size > artifact.size {
            return Err(io::Error::other(format!(
                "partial model {} is {partial_size} bytes, larger than expected {}; remove it before retrying",
                partial_path.display(),
                artifact.size
            ))
            .into());
        }
    }

    downloader(&partial_path, artifact)?;
    verify_artifact(&partial_path, artifact)?;
    fs::rename(&partial_path, &final_path)?;
    set_readonly(&final_path, true)?;
    ensure_metadata(&metadata_path, artifact)?;
    Ok(AcquiredModel {
        path: final_path,
        artifact,
    })
}

fn default_cache_root() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = nonempty_env_path("FERRITE_MODEL_CACHE") {
        return Ok(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = nonempty_env_path("LOCALAPPDATA") {
        return Ok(path.join("Ferrite").join("models"));
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = nonempty_env_path("HOME") {
        return Ok(path
            .join("Library")
            .join("Caches")
            .join("ferrite")
            .join("models"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Some(path) = nonempty_env_path("XDG_CACHE_HOME") {
        return Ok(path.join("ferrite").join("models"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Some(path) = nonempty_env_path("HOME") {
        return Ok(path.join(".cache").join("ferrite").join("models"));
    }
    Err(io::Error::other(
        "cannot determine a model cache directory; set FERRITE_MODEL_CACHE or pass --model-cache",
    )
    .into())
}

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn download_with_curl(path: &Path, artifact: ModelArtifact) -> Result<(), Box<dyn Error>> {
    eprintln!(
        "acquiring model={} revision={} bytes={} sha256={}",
        artifact.id, artifact.revision, artifact.size, artifact.sha256
    );
    let status = Command::new("curl")
        .arg("--disable")
        .arg("--fail")
        .arg("--location")
        .arg("--proto")
        .arg("=https")
        .arg("--proto-redir")
        .arg("=https")
        .arg("--tlsv1.2")
        .arg("--retry")
        .arg("3")
        .arg("--retry-delay")
        .arg("1")
        .arg("--continue-at")
        .arg("-")
        .arg("--progress-bar")
        .arg("--output")
        .arg(path)
        .arg(artifact.download_url)
        .status()
        .map_err(|error| {
            io::Error::other(format!(
                "failed to start curl for resumable model acquisition: {error}; install curl or acquire the pinned artifact manually"
            ))
        })?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "curl failed with {status}; the partial file remains at {} for a resumable retry",
            path.display()
        ))
        .into());
    }
    Ok(())
}

fn verify_artifact(path: &Path, artifact: ModelArtifact) -> Result<(), Box<dyn Error>> {
    let actual_size = fs::metadata(path)?.len();
    if actual_size != artifact.size {
        return Err(io::Error::other(format!(
            "model {} has {actual_size} bytes, expected {}",
            path.display(),
            artifact.size
        ))
        .into());
    }
    let actual_hash = sha256_file(path)?;
    if actual_hash != artifact.sha256 {
        return Err(io::Error::other(format!(
            "model {} SHA-256 {actual_hash} does not match expected {}",
            path.display(),
            artifact.sha256
        ))
        .into());
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn ensure_metadata(path: &Path, artifact: ModelArtifact) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        reject_symlink(path)?;
        let parsed: serde_json::Value =
            serde_json::from_slice(&fs::read(path)?).map_err(|error| {
                io::Error::other(format!(
                    "model metadata {} is invalid JSON: {error}",
                    path.display()
                ))
            })?;
        if parsed != metadata_value(artifact) {
            return Err(io::Error::other(format!(
                "model metadata {} does not match the built-in artifact registry",
                path.display()
            ))
            .into());
        }
        set_readonly(path, true)?;
        return Ok(());
    }

    let temporary = path.with_extension("json.partial");
    if temporary.exists() {
        reject_symlink(&temporary)?;
        set_readonly(&temporary, false)?;
        fs::remove_file(&temporary)?;
    }
    let bytes = serde_json::to_vec_pretty(&metadata_value(artifact))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    use std::io::Write;
    let mut file = options.open(&temporary)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)?;
    set_readonly(path, true)
}

fn metadata_value(artifact: ModelArtifact) -> serde_json::Value {
    json!({
        "id": artifact.id,
        "source": artifact.source,
        "revision": artifact.revision,
        "license": artifact.license,
        "license_url": artifact.license_url,
        "filename": artifact.filename,
        "size": artifact.size,
        "sha256": artifact.sha256,
        "download_url": artifact.download_url,
    })
}

fn reject_symlink(path: &Path) -> Result<(), Box<dyn Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::other(format!(
            "refusing to use symbolic link in model cache: {}",
            path.display()
        ))
        .into());
    }
    if !metadata.is_file() {
        return Err(io::Error::other(format!(
            "model cache path is not a regular file: {}",
            path.display()
        ))
        .into());
    }
    Ok(())
}

fn set_readonly(path: &Path, readonly: bool) -> Result<(), Box<dyn Error>> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        permissions.set_mode(if readonly {
            mode & !0o222
        } else {
            mode | 0o200
        });
    }
    #[cfg(not(unix))]
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

struct AcquisitionLock {
    path: PathBuf,
}

impl AcquisitionLock {
    fn create(path: &Path) -> Result<Self, Box<dyn Error>> {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => {
                use std::io::Write;
                writeln!(file, "pid={}", std::process::id())?;
                file.sync_all()?;
                Ok(Self {
                    path: path.to_owned(),
                })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(io::Error::other(format!(
                    "model acquisition is already locked at {}; if no acquisition is running, remove this stale lock",
                    path.display()
                ))
                .into())
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for AcquisitionLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    const TEST_ARTIFACT: ModelArtifact = ModelArtifact {
        id: "test-model",
        source: "https://example.invalid/test-model",
        revision: "revision",
        license: "MIT",
        license_url: "https://example.invalid/test-model/LICENSE",
        filename: "model.gguf",
        size: 3,
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        download_url: "https://example.invalid/test-model/model.gguf",
    };

    #[test]
    fn registry_exposes_pinned_phi3_artifact() -> Result<(), Box<dyn Error>> {
        let artifact = builtin_model("phi3-mini-4k-instruct-q4")?;
        assert_eq!(artifact.size, 2_393_231_072);
        assert_eq!(artifact.license, "MIT");
        assert_eq!(artifact.sha256.len(), 64);
        assert!(artifact.download_url.contains(artifact.revision));
        Ok(())
    }

    #[test]
    fn acquisition_resumes_then_verifies_and_makes_cache_readonly() -> Result<(), Box<dyn Error>> {
        let root = test_root("resume")?;
        let revision_dir = root.join(TEST_ARTIFACT.id).join(TEST_ARTIFACT.revision);
        fs::create_dir_all(&revision_dir)?;
        let partial = revision_dir.join("model.gguf.partial");
        fs::write(&partial, b"a")?;

        let acquired = acquire_with_downloader(TEST_ARTIFACT, &root, false, |path, _artifact| {
            assert_eq!(fs::read(path)?, b"a");
            let mut file = OpenOptions::new().append(true).open(path)?;
            use std::io::Write;
            file.write_all(b"bc")?;
            Ok(())
        })?;

        assert_eq!(fs::read(acquired.path())?, b"abc");
        assert!(fs::metadata(acquired.path())?.permissions().readonly());
        let metadata = revision_dir.join("artifact.json");
        assert!(fs::metadata(metadata)?.permissions().readonly());
        cleanup(&root)?;
        Ok(())
    }

    #[test]
    fn offline_mode_requires_a_verified_cached_artifact() -> Result<(), Box<dyn Error>> {
        let root = test_root("offline")?;
        let result = acquire_with_downloader(TEST_ARTIFACT, &root, true, |_path, _artifact| {
            Err(io::Error::other("downloader must not run in offline mode").into())
        });
        let error = match result {
            Ok(_) => return Err("offline cache miss should fail".into()),
            Err(error) => error,
        };
        assert!(error.to_string().contains("not cached"));
        cleanup(&root)?;
        Ok(())
    }

    #[test]
    fn hash_mismatch_never_publishes_the_final_artifact() -> Result<(), Box<dyn Error>> {
        let root = test_root("mismatch")?;
        let result = acquire_with_downloader(TEST_ARTIFACT, &root, false, |path, _artifact| {
            fs::write(path, b"abd")?;
            Ok(())
        });
        let error = match result {
            Ok(_) => return Err("hash mismatch should fail".into()),
            Err(error) => error,
        };
        assert!(error.to_string().contains("does not match expected"));
        assert!(!root
            .join(TEST_ARTIFACT.id)
            .join(TEST_ARTIFACT.revision)
            .join(TEST_ARTIFACT.filename)
            .exists());
        cleanup(&root)?;
        Ok(())
    }

    fn test_root(label: &str) -> Result<PathBuf, Box<dyn Error>> {
        let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "ferrite-model-acquisition-{}-{label}-{sequence}",
            std::process::id()
        ));
        if root.exists() {
            cleanup(&root)?;
        }
        Ok(root)
    }

    fn cleanup(root: &Path) -> Result<(), Box<dyn Error>> {
        if !root.exists() {
            return Ok(());
        }
        for entry in walk_files(root)? {
            set_readonly(&entry, false)?;
        }
        fs::remove_dir_all(root)?;
        Ok(())
    }

    fn walk_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let mut pending = vec![root.to_owned()];
        let mut files = Vec::new();
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    pending.push(entry.path());
                } else {
                    files.push(entry.path());
                }
            }
        }
        Ok(files)
    }
}
