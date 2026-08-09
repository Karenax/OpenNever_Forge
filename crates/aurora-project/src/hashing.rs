use aurora_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const HASH_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPhase {
    #[default]
    Hashing,
    Inventory,
    Dependencies,
    ResourceCatalog,
    StructuredResources,
    Scripts,
    Dialogues,
    World,
    Persisting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HashProgress {
    pub bytes_read: u64,
    pub total_bytes: u64,
    pub percent: f64,
    #[serde(default)]
    pub phase: AnalysisPhase,
}

impl HashProgress {
    pub fn stage(phase: AnalysisPhase, percent: f64) -> Self {
        Self {
            bytes_read: 0,
            total_bytes: 0,
            percent,
            phase,
        }
    }

    pub(crate) fn scaled(mut self, start: f64, end: f64) -> Self {
        self.percent = start + (end - start) * (self.percent / 100.0);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleFingerprint {
    pub sha256: String,
    pub size_bytes: u64,
}

pub fn hash_module_file<F>(
    path: &Path,
    cancelled: &AtomicBool,
    on_progress: F,
) -> AppResult<ModuleFingerprint>
where
    F: FnMut(HashProgress),
{
    if !path.is_file() {
        return Err(AppError::module_not_found(path.display().to_string()).into());
    }

    hash_existing_file(path, cancelled, on_progress)
}

pub(crate) fn hash_existing_file<F>(
    path: &Path,
    cancelled: &AtomicBool,
    mut on_progress: F,
) -> AppResult<ModuleFingerprint>
where
    F: FnMut(HashProgress),
{
    let file = File::open(path)
        .map_err(|error| AppError::io("open", path.display().to_string(), &error))?;
    let total_bytes = file
        .metadata()
        .map_err(|error| AppError::io("metadata", path.display().to_string(), &error))?
        .len();
    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_SIZE];
    let mut bytes_read = 0_u64;

    on_progress(progress(bytes_read, total_bytes));

    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(path.display().to_string()).into());
        }

        let count = reader
            .read(&mut buffer)
            .map_err(|error| AppError::io("read", path.display().to_string(), &error))?;
        if count == 0 {
            break;
        }

        digest.update(&buffer[..count]);
        bytes_read += count as u64;
        on_progress(progress(bytes_read, total_bytes));
    }

    Ok(ModuleFingerprint {
        sha256: hex::encode_upper(digest.finalize()),
        size_bytes: total_bytes,
    })
}

fn progress(bytes_read: u64, total_bytes: u64) -> HashProgress {
    let percent = if total_bytes == 0 {
        100.0
    } else {
        (bytes_read as f64 / total_bytes as f64) * 100.0
    };

    HashProgress {
        bytes_read,
        total_bytes,
        percent,
        phase: AnalysisPhase::Hashing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[test]
    fn hashes_a_file_in_streaming_mode() {
        let root = tempdir().expect("temp directory");
        let module = root.path().join("fixture.mod");
        fs::write(&module, b"OpenNever Forge").expect("write fixture");
        let mut updates = Vec::new();

        let fingerprint = hash_module_file(&module, &AtomicBool::new(false), |value| {
            updates.push(value)
        })
        .expect("hash succeeds");

        assert_eq!(
            fingerprint.sha256,
            "210444635B71D4C409F41A052AC514A0B5D0B31FC46DA03E3EF6E4CA3D0E4078"
        );
        assert_eq!(fingerprint.size_bytes, 15);
        assert_eq!(updates.last().map(|value| value.percent), Some(100.0));
    }

    #[test]
    fn observes_cancellation_before_reading() {
        let root = tempdir().expect("temp directory");
        let module = root.path().join("fixture.mod");
        fs::write(&module, vec![42_u8; 128]).expect("write fixture");

        let error = hash_module_file(&module, &AtomicBool::new(true), |_| {})
            .expect_err("hash must be cancelled");

        assert_eq!(error.code, "JOB_CANCELLED");
    }
}
