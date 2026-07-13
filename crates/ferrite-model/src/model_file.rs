//! Shared read-only model-file mapping for zero-copy tensor storage.
#![allow(
    unsafe_code,
    reason = "read-only file mapping is isolated behind an immutable slice API"
)]

use memmap2::{Mmap, MmapOptions};
use std::{fs::File, io, path::Path, sync::Arc};

/// A read-only operating-system mapping of one model file.
///
/// Mapping avoids allocating a model-sized heap buffer and can be cloned
/// cheaply so validated tensors retain ranges of the same operating-system
/// mapping.
#[derive(Clone, Debug)]
pub struct MappedModelFile {
    bytes: Arc<Mmap>,
}

impl MappedModelFile {
    /// Opens and maps a non-empty model file read-only.
    ///
    /// # Safety
    ///
    /// The underlying file must not be modified or truncated until this value
    /// and every clone derived from it have been dropped. File-backed mappings
    /// cannot enforce that condition against other processes.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the file cannot be opened, inspected, or
    /// mapped, or when it is empty.
    pub unsafe fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "model file must not be empty",
            ));
        }
        // SAFETY: the caller accepts the file-stability requirement documented
        // by this function. The mapping is read-only, `Mmap` keeps it alive
        // independently of the `File`, and this API exposes immutable slices.
        let bytes = Arc::new(unsafe { MmapOptions::new().map(&file)? });
        Ok(Self { bytes })
    }

    /// Returns the complete mapped file contents.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::MappedModelFile;
    use std::{fs, io, path::PathBuf};

    #[test]
    fn maps_complete_file_contents_read_only() -> io::Result<()> {
        let path = temporary_path("contents");
        fs::write(&path, b"GGUF-test-bytes")?;

        // SAFETY: the test owns the temporary file and does not modify or
        // remove it until the mapping is dropped.
        let mapped = unsafe { MappedModelFile::open(&path)? };
        assert_eq!(mapped.as_bytes(), b"GGUF-test-bytes");

        drop(mapped);
        fs::remove_file(path)
    }

    #[test]
    fn rejects_empty_model_file() -> io::Result<()> {
        let path = temporary_path("empty");
        fs::write(&path, [])?;

        // SAFETY: the test owns the temporary file and does not modify it
        // while the attempted mapping is alive.
        let error = unsafe { MappedModelFile::open(&path) }
            .err()
            .ok_or_else(|| io::Error::other("empty model file unexpectedly mapped"))?;

        fs::remove_file(path)?;
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        Ok(())
    }

    fn temporary_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ferrite-mapped-model-{label}-{}",
            std::process::id()
        ))
    }
}
