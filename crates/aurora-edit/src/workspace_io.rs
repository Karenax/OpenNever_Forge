use aurora_core::{AppError, AppResult, resource_type_for_extension};
use std::fs;
use std::path::{Path, PathBuf};

use super::{atomic_write, edit_error, sha256_bytes, sha256_file};
use crate::sync::{AURORA_SYNC_SCHEMA_VERSION, AuroraSyncManifest, resource_key_from_aurora_path};
use crate::types::DevelopmentFile;

pub fn scan_aurora_workspace(root: &Path) -> AppResult<AuroraSyncManifest> {
    if !root.is_dir() {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_ROOT_INVALID",
            format!("{} is not a directory", root.display()),
        ));
    }
    let mut paths = Vec::new();
    collect_aurora_files(root, root, &mut paths, 0)?;
    paths.sort();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| edit_error("EDIT_AURORA_SYNC_PATH_INVALID", path.display().to_string()))?;
        let metadata = fs::metadata(&path).map_err(|error| {
            Box::new(AppError::io(
                "inspect Aurora workspace file",
                path.display().to_string(),
                &error,
            ))
        })?;
        files.push(DevelopmentFile {
            name: relative.to_string_lossy().replace('\\', "/"),
            sha256: sha256_file(&path)?,
            size_bytes: metadata.len(),
        });
    }
    Ok(AuroraSyncManifest {
        schema_version: AURORA_SYNC_SCHEMA_VERSION,
        root: canonical_toolset_root(root)?,
        files,
    })
}

pub fn read_aurora_workspace_file(root: &Path, relative_path: &str) -> AppResult<Option<Vec<u8>>> {
    let path = safe_aurora_workspace_path(root, relative_path)?;
    if !path.is_file() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path).map_err(|error| {
        Box::new(AppError::io(
            "inspect Aurora synchronization file",
            path.display().to_string(),
            &error,
        ))
    })?;
    if metadata.len() > 256 * 1024 * 1024 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_FILE_TOO_LARGE",
            format!(
                "{} exceeds the 256 MiB synchronization limit",
                path.display()
            ),
        ));
    }
    fs::read(&path).map(Some).map_err(|error| {
        Box::new(AppError::io(
            "read Aurora synchronization file",
            path.display().to_string(),
            &error,
        ))
    })
}

pub fn write_aurora_workspace_file(
    root: &Path,
    relative_path: &str,
    bytes: Option<&[u8]>,
) -> AppResult<Option<String>> {
    let path = safe_aurora_workspace_path(root, relative_path)?;
    let backup = if path.is_file() {
        let previous = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "backup Aurora synchronization file",
                path.display().to_string(),
                &error,
            ))
        })?;
        let digest = sha256_bytes(&previous);
        let backup_path = root
            .join(".opennever-backups")
            .join(&digest[..16])
            .join(relative_path);
        if !backup_path.is_file() {
            atomic_write(&backup_path, &previous)?;
        }
        Some(backup_path.display().to_string())
    } else {
        None
    };
    match bytes {
        Some(bytes) => atomic_write(&path, bytes)?,
        None if path.is_file() => fs::remove_file(&path).map_err(|error| {
            Box::new(AppError::io(
                "remove synchronized Aurora file",
                path.display().to_string(),
                &error,
            ))
        })?,
        None => {}
    }
    Ok(backup)
}

fn safe_aurora_workspace_path(root: &Path, relative_path: &str) -> AppResult<PathBuf> {
    resource_key_from_aurora_path(relative_path)?;
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        Box::new(AppError::io(
            "canonicalize Aurora workspace",
            root.display().to_string(),
            &error,
        ))
    })?;
    let relative = Path::new(relative_path);
    let mut current = canonical_root.clone();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(edit_error("EDIT_AURORA_SYNC_PATH_INVALID", relative_path));
        };
        current.push(component);
        if current.exists()
            && fs::symlink_metadata(&current)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(true)
        {
            return Err(edit_error(
                "EDIT_AURORA_SYNC_SYMLINK_REJECTED",
                current.display().to_string(),
            ));
        }
    }
    if !current.starts_with(&canonical_root) {
        return Err(edit_error("EDIT_AURORA_SYNC_PATH_INVALID", relative_path));
    }
    Ok(current)
}

pub(crate) fn canonical_toolset_root(root: &Path) -> AppResult<String> {
    if !root.is_dir() {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_ROOT_INVALID",
            format!("{} is not a directory", root.display()),
        ));
    }
    fs::canonicalize(root)
        .map(|path| path.display().to_string())
        .map_err(|error| {
            Box::new(AppError::io(
                "canonicalize Aurora workspace",
                root.display().to_string(),
                &error,
            ))
        })
}

fn collect_aurora_files(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    depth: usize,
) -> AppResult<()> {
    if depth > 128 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_DEPTH_LIMIT",
            "Aurora workspace exceeds the directory depth limit of 128",
        ));
    }
    if paths.len() > 10_000 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_FILE_LIMIT",
            "Aurora workspace contains more than 10000 supported files",
        ));
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        Box::new(AppError::io(
            "scan Aurora workspace",
            directory.display().to_string(),
            &error,
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Box::new(AppError::io(
                "read Aurora workspace entry",
                directory.display().to_string(),
                &error,
            ))
        })?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            Box::new(AppError::io(
                "inspect Aurora workspace entry",
                entry.path().display().to_string(),
                &error,
            ))
        })?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        if metadata.is_dir()
            && entry.file_name() != ".opennever-backups"
            && entry.file_name() != ".git"
        {
            collect_aurora_files(root, &path, paths, depth + 1)?;
        } else if metadata.is_file() && supported_aurora_extension(&path) {
            if !path.starts_with(root) {
                return Err(edit_error(
                    "EDIT_AURORA_SYNC_PATH_INVALID",
                    path.display().to_string(),
                ));
            }
            if paths.len() >= 10_000 {
                return Err(edit_error(
                    "EDIT_AURORA_SYNC_FILE_LIMIT",
                    "Aurora workspace contains more than 10000 supported files",
                ));
            }
            paths.push(path);
        }
    }
    Ok(())
}

fn supported_aurora_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .and_then(resource_type_for_extension)
        .is_some_and(|resource_type| {
            !matches!(resource_type, 2011 | 2061 | 2062 | 9997 | 9998 | 9999)
        })
}

pub(crate) fn validate_dependency_name(value: &str, extension: &str) -> AppResult<()> {
    let path = Path::new(value);
    if value.is_empty()
        || path.components().count() != 1
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(edit_error(
            "EDIT_DEPENDENCY_NAME_INVALID",
            format!("{value:?} is not a safe {extension} dependency name"),
        ));
    }
    Ok(())
}
