use crate::assets::{PlannedFile, hash_bytes, size_limit_error};
use crate::model::{
    BUNDLE_CLASSIFICATION, BUNDLE_REDISTRIBUTION, BUNDLE_SCHEMA_VERSION, BundleFileRecord,
    BundleManifest, CoordinateConvention, IdentityMap, MAX_BUNDLE_BYTES, MAX_BUNDLE_FILES,
    MigrationAreaDocument, MigrationDiagnostic, MigrationReport, MigrationResourceVersion,
};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use serde::Serialize;
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct BundleDraft {
    pub area: MigrationAreaDocument,
    pub identity_map: IdentityMap,
    pub diagnostics: Vec<MigrationDiagnostic>,
    pub report: MigrationReport,
    pub manifest: BundleManifest,
    pub asset_files: Vec<PlannedFile>,
}

#[derive(Debug)]
pub(crate) struct WrittenBundle {
    pub path: PathBuf,
    pub manifest_file: BundleFileRecord,
    pub report: MigrationReport,
}

pub fn validate_bundle_destination(destination: impl AsRef<Path>) -> AppResult<PathBuf> {
    validate_bundle_destination_with_sources(destination, &[])
}

pub fn validate_bundle_destination_with_sources(
    destination: impl AsRef<Path>,
    protected_sources: &[PathBuf],
) -> AppResult<PathBuf> {
    let raw = destination.as_ref();
    if !raw.is_absolute()
        || raw.as_os_str().is_empty()
        || raw.components().any(|part| part == Component::ParentDir)
    {
        return Err(path_error(
            raw,
            "destination is empty or contains a parent traversal",
        ));
    }
    let absolute = raw.to_path_buf();
    let file_name = absolute
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            path_error(
                &absolute,
                "destination requires a UTF-8 leaf directory name",
            )
        })?;
    if file_name.is_empty()
        || file_name.ends_with(['.', ' '])
        || file_name
            .chars()
            .any(|value| matches!(value, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err(path_error(
            &absolute,
            "destination leaf contains characters unsafe on Windows",
        ));
    }
    let destination_exists = match fs::symlink_metadata(&absolute) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(Box::new(AppError::io(
                "inspect migration destination",
                "destination",
                &error,
            )));
        }
    };
    if destination_exists || absolute.exists() {
        return Err(Box::new(
            AppError::new(
                "MIGRATION_DESTINATION_EXISTS",
                "La destination existe déjà.",
                "destination already exists; choose a new bundle directory",
                ErrorSeverity::Error,
            )
            .with_resource(file_name)
            .with_import_stage("area_migration_path_validation"),
        ));
    }
    let parent = absolute
        .parent()
        .ok_or_else(|| path_error(&absolute, "destination has no parent directory"))?;
    if !parent.is_dir() {
        return Err(path_error(
            parent,
            "destination parent is not an existing directory",
        ));
    }
    let parent_metadata = fs::symlink_metadata(parent).map_err(|error| {
        Box::new(AppError::io(
            "inspect destination parent",
            "destination parent",
            &error,
        ))
    })?;
    if is_link_metadata(&parent_metadata) || contains_link_component(parent) {
        return Err(path_error(
            parent,
            "destination parent must not traverse a symbolic link or junction",
        ));
    }
    let canonical_parent = parent.canonicalize().map_err(|error| {
        Box::new(AppError::io(
            "canonicalize destination parent",
            "destination parent",
            &error,
        ))
    })?;
    let normalized = canonical_parent.join(file_name);
    for source in protected_sources {
        let Ok(canonical_source) = source.canonicalize() else {
            continue;
        };
        let candidate = normalized_path(&normalized);
        let protected = normalized_path(&canonical_source);
        if is_same_or_descendant(&candidate, &protected) {
            return Err(path_error(
                &normalized,
                "destination resolves to or below a protected NWN source root",
            ));
        }
    }
    Ok(normalized)
}

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

fn is_link_metadata(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn contains_link_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current)
            && is_link_metadata(&metadata)
        {
            return true;
        }
    }
    false
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    value
        .strip_prefix("//?/")
        .or_else(|| value.strip_prefix("//./"))
        .unwrap_or(&value)
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_same_or_descendant(candidate: &str, protected: &str) -> bool {
    candidate == protected
        || candidate
            .strip_prefix(protected)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(crate) fn write_bundle_atomic(
    destination: &Path,
    protected_sources: &[PathBuf],
    cancelled: &AtomicBool,
    mut draft: BundleDraft,
    verify_sources: impl FnOnce() -> AppResult<()>,
) -> AppResult<WrittenBundle> {
    let destination = validate_bundle_destination_with_sources(destination, protected_sources)?;
    check_cancelled(cancelled, "bundle")?;
    enforce_planned_file_limits(&draft.asset_files)?;
    let parent = destination
        .parent()
        .expect("validated destination has parent");
    let staging = tempfile::Builder::new()
        .prefix(".opennever-area-migration-")
        .tempdir_in(parent)
        .map_err(|error| {
            Box::new(AppError::io(
                "create migration staging directory",
                "destination parent",
                &error,
            ))
        })?;
    let staging_root = staging.path();

    for file in &draft.asset_files {
        check_cancelled(cancelled, &file.path)?;
        write_scratch_file(staging_root, file)?;
    }
    write_json_streaming(staging_root, "area.json", &draft.area)?;
    write_json_streaming(staging_root, "identity-map.json", &draft.identity_map)?;
    write_diagnostics_streaming(staging_root, &draft.diagnostics)?;

    let pre_report = inventory(staging_root, &roles(&draft.asset_files), false)?;
    draft.report.payload_file_count = pre_report.len();
    draft.report.payload_size_bytes = pre_report.iter().map(|file| file.size_bytes).sum();
    write_json_streaming(staging_root, "migration-report.json", &draft.report)?;

    let mut file_roles = roles(&draft.asset_files);
    file_roles.insert("area.json".to_owned(), "area".to_owned());
    file_roles.insert("identity-map.json".to_owned(), "identity-map".to_owned());
    file_roles.insert("diagnostics.jsonl".to_owned(), "diagnostics".to_owned());
    file_roles.insert("migration-report.json".to_owned(), "report".to_owned());
    let records = inventory(staging_root, &file_roles, false)?;
    enforce_limits(&records)?;
    draft.manifest.files = records;
    write_json_streaming(staging_root, "manifest.json", &draft.manifest)?;
    validate_bundle_directory(staging_root)?;
    verify_sources()?;
    check_cancelled(cancelled, "bundle-commit")?;

    let manifest_bytes = fs::read(staging_root.join("manifest.json")).map_err(|error| {
        Box::new(AppError::io(
            "read finalized manifest",
            "manifest.json",
            &error,
        ))
    })?;
    let manifest_file = BundleFileRecord {
        path: "manifest.json".to_owned(),
        size_bytes: manifest_bytes.len() as u64,
        sha256: hash_bytes(&manifest_bytes),
        role: "manifest".to_owned(),
    };
    let staging_path = staging.keep();
    if let Err(error) = fs::rename(&staging_path, &destination) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(Box::new(AppError::io(
            "atomically publish migration bundle",
            "destination",
            &error,
        )));
    }
    Ok(WrittenBundle {
        path: destination,
        manifest_file,
        report: draft.report,
    })
}

pub fn validate_bundle_directory(root: impl AsRef<Path>) -> AppResult<BundleManifest> {
    let root = root.as_ref();
    let manifest_path = root.join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        Box::new(AppError::io(
            "read migration manifest",
            "manifest.json",
            &error,
        ))
    })?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        Box::new(
            AppError::new(
                "MIGRATION_MANIFEST_INVALID",
                "Le manifeste du bundle n'est pas valide.",
                format!("cannot decode manifest.json: {error}"),
                ErrorSeverity::Error,
            )
            .with_source("manifest.json")
            .with_import_stage("area_migration_verification"),
        )
    })?;
    if manifest.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(Box::new(
            AppError::new(
                "MIGRATION_SCHEMA_UNSUPPORTED",
                "La version du bundle n'est pas prise en charge.",
                format!(
                    "expected {BUNDLE_SCHEMA_VERSION}, got {}",
                    manifest.schema_version
                ),
                ErrorSeverity::Error,
            )
            .with_source("manifest.json"),
        ));
    }
    validate_manifest_contract(&manifest, &manifest_path)?;
    enforce_limits(&manifest.files)?;
    let mut listed = BTreeSet::new();
    for record in &manifest.files {
        if !is_safe_relative_path(&record.path) || !listed.insert(record.path.clone()) {
            return Err(path_error(
                &root.join(&record.path),
                "manifest contains an unsafe or duplicate relative path",
            ));
        }
        let path = root.join(path_from_bundle(&record.path));
        let (size_bytes, sha256) = hash_file_streaming(&path)?;
        if size_bytes != record.size_bytes || sha256 != record.sha256 {
            return Err(Box::new(
                AppError::new(
                    "MIGRATION_FILE_INTEGRITY_MISMATCH",
                    "L'intégrité d'un fichier du bundle est invalide.",
                    format!("size or SHA-256 mismatch for {}", record.path),
                    ErrorSeverity::Error,
                )
                .with_source(record.path.clone())
                .with_import_stage("area_migration_verification"),
            ));
        }
    }
    for required in [
        "area.json",
        "identity-map.json",
        "diagnostics.jsonl",
        "migration-report.json",
    ] {
        if !listed.contains(required) {
            return Err(Box::new(
                AppError::new(
                    "MIGRATION_REQUIRED_FILE_MISSING",
                    "Un fichier obligatoire du bundle est absent.",
                    format!("manifest does not list {required}"),
                    ErrorSeverity::Error,
                )
                .with_source("bundle"),
            ));
        }
    }
    let actual = collect_relative_files(root)?;
    let expected = listed
        .into_iter()
        .chain(std::iter::once("manifest.json".to_owned()))
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(Box::new(
            AppError::new(
                "MIGRATION_UNLISTED_FILE",
                "Le bundle contient un fichier non déclaré.",
                format!("actual files {actual:?} differ from manifest inventory {expected:?}"),
                ErrorSeverity::Error,
            )
            .with_source("bundle"),
        ));
    }
    Ok(manifest)
}

fn validate_manifest_contract(manifest: &BundleManifest, _source: &Path) -> AppResult<()> {
    let mut violations = Vec::new();
    if !is_sha256(&manifest.module_sha256) {
        violations.push("moduleSha256 must be 64 lowercase hexadecimal characters".to_owned());
    }
    if manifest.area_resref.is_empty()
        || manifest.area_resref.len() > 64
        || !manifest
            .area_resref
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-'))
    {
        violations.push("areaResref is outside the bounded portable ResRef alphabet".to_owned());
    }
    if manifest.exporter.is_empty()
        || manifest.resolution_policy.is_empty()
        || manifest.integrity_scope.is_empty()
    {
        violations
            .push("exporter, resolutionPolicy and integrityScope must be non-empty".to_owned());
    }
    if manifest.classification != BUNDLE_CLASSIFICATION {
        violations.push(format!("classification must be {BUNDLE_CLASSIFICATION}"));
    }
    if manifest.redistribution != BUNDLE_REDISTRIBUTION {
        violations.push(format!("redistribution must be {BUNDLE_REDISTRIBUTION}"));
    }
    if manifest.coordinate_system != CoordinateConvention::default() {
        violations.push("coordinateSystem does not match nwn-to-canonical@1".to_owned());
    }
    for dependency in &manifest.dependencies {
        if dependency.logical_name.is_empty() {
            violations.push("dependency logicalName must be non-empty".to_owned());
        }
        for (field, value) in [
            (
                "selectedPathSha256",
                dependency.selected_path_sha256.as_deref(),
            ),
            (
                "selectedContentSha256",
                dependency.selected_content_sha256.as_deref(),
            ),
        ] {
            if value.is_some_and(|value| !is_sha256(value)) {
                violations.push(format!("dependency {field} is not a SHA-256"));
            }
        }
    }
    for resource in &manifest.resources {
        if resource.resource_key.is_empty() || resource.purpose.is_empty() {
            violations.push("resourceKey and purpose must be non-empty".to_owned());
        }
        validate_resource_version(&resource.selected, &mut violations);
        for version in &resource.shadowed {
            validate_resource_version(version, &mut violations);
        }
    }
    for file in &manifest.files {
        if file.role.is_empty() || !is_sha256(&file.sha256) {
            violations.push(format!(
                "file {} requires a non-empty role and lowercase SHA-256",
                file.path
            ));
        }
    }
    if violations.is_empty() {
        return Ok(());
    }
    Err(Box::new(
        AppError::new(
            "MIGRATION_MANIFEST_SCHEMA_INVALID",
            "Le manifeste ne respecte pas le contrat Area Migration Bundle v1.",
            violations.join("; "),
            ErrorSeverity::Error,
        )
        .with_source("manifest.json")
        .with_import_stage("area_migration_schema_validation"),
    ))
}

fn validate_resource_version(version: &MigrationResourceVersion, violations: &mut Vec<String>) {
    if version.source_name.is_empty()
        || version.source_file_name.is_empty()
        || version.source_file_name.contains('/')
        || version.source_file_name.contains('\\')
        || !is_sha256(&version.source_path_sha256)
        || version
            .content_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
    {
        violations.push(format!(
            "resource version {} has invalid sanitized provenance or SHA-256",
            version.source_name
        ));
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn write_json_streaming<T: Serialize>(root: &Path, relative: &str, value: &T) -> AppResult<()> {
    let path = safe_output_path(root, relative)?;
    let file = fs::File::create(&path).map_err(|error| {
        Box::new(AppError::io(
            "create bundle JSON",
            relative.to_owned(),
            &error,
        ))
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|error| {
        Box::new(
            AppError::new(
                "MIGRATION_JSON_SERIALIZATION_FAILED",
                "Une donnée du bundle n'a pas pu être sérialisée.",
                format!("cannot serialize {relative}: {error}"),
                ErrorSeverity::Error,
            )
            .with_import_stage("area_migration_bundle"),
        )
    })?;
    writer.write_all(b"\n").map_err(|error| {
        Box::new(AppError::io(
            "write bundle JSON",
            relative.to_owned(),
            &error,
        ))
    })?;
    writer.flush().map_err(|error| {
        Box::new(AppError::io(
            "flush bundle JSON",
            relative.to_owned(),
            &error,
        ))
    })
}

fn write_diagnostics_streaming(root: &Path, diagnostics: &[MigrationDiagnostic]) -> AppResult<()> {
    let path = safe_output_path(root, "diagnostics.jsonl")?;
    let file = fs::File::create(&path).map_err(|error| {
        Box::new(AppError::io(
            "create migration diagnostics",
            "diagnostics.jsonl",
            &error,
        ))
    })?;
    let mut writer = BufWriter::new(file);
    for diagnostic in diagnostics {
        serde_json::to_writer(&mut writer, diagnostic).map_err(|error| {
            Box::new(
                AppError::new(
                    "MIGRATION_DIAGNOSTICS_SERIALIZATION_FAILED",
                    "Les diagnostics du bundle n'ont pas pu être sérialisés.",
                    error.to_string(),
                    ErrorSeverity::Error,
                )
                .with_import_stage("area_migration_bundle"),
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            Box::new(AppError::io(
                "write migration diagnostics",
                "diagnostics.jsonl",
                &error,
            ))
        })?;
    }
    writer.flush().map_err(|error| {
        Box::new(AppError::io(
            "flush migration diagnostics",
            "diagnostics.jsonl",
            &error,
        ))
    })
}

#[allow(dead_code)]
fn write_json<T: Serialize>(root: &Path, relative: &str, value: &T) -> AppResult<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        Box::new(
            AppError::new(
                "MIGRATION_JSON_SERIALIZATION_FAILED",
                "Une donnée du bundle n'a pas pu être sérialisée.",
                format!("cannot serialize {relative}: {error}"),
                ErrorSeverity::Error,
            )
            .with_import_stage("area_migration_bundle"),
        )
    })?;
    bytes.push(b'\n');
    write_relative(root, relative, &bytes)
}

#[allow(dead_code)]
fn write_diagnostics(root: &Path, diagnostics: &[MigrationDiagnostic]) -> AppResult<()> {
    let mut bytes = Vec::new();
    for diagnostic in diagnostics {
        serde_json::to_writer(&mut bytes, diagnostic).map_err(|error| {
            Box::new(
                AppError::new(
                    "MIGRATION_DIAGNOSTICS_SERIALIZATION_FAILED",
                    "Les diagnostics du bundle n'ont pas pu être sérialisés.",
                    error.to_string(),
                    ErrorSeverity::Error,
                )
                .with_import_stage("area_migration_bundle"),
            )
        })?;
        bytes.push(b'\n');
    }
    write_relative(root, "diagnostics.jsonl", &bytes)
}

fn write_scratch_file(root: &Path, file: &PlannedFile) -> AppResult<()> {
    let destination = safe_output_path(root, &file.path)?;
    let mut source = fs::File::open(&file.scratch_path).map_err(|error| {
        Box::new(AppError::io(
            "read migration scratch payload",
            "scratch",
            &error,
        ))
    })?;
    let mut output = fs::File::create(&destination)
        .map_err(|error| Box::new(AppError::io("create bundle payload", &file.path, &error)))?;
    let copied = std::io::copy(&mut source, &mut output)
        .map_err(|error| Box::new(AppError::io("copy bundle payload", &file.path, &error)))?;
    if copied != file.size_bytes {
        return Err(size_limit_error(format!(
            "scratch payload {} changed size before publication",
            file.path
        )));
    }
    Ok(())
}

fn safe_output_path(root: &Path, relative: &str) -> AppResult<PathBuf> {
    if !is_safe_relative_path(relative) {
        return Err(path_error(
            root,
            "bundle path is not a safe normalized relative path",
        ));
    }
    let path = root.join(path_from_bundle(relative));
    if let Some(parent) = path.parent() {
        if contains_link_component(parent) {
            return Err(path_error(
                parent,
                "bundle output path must not traverse a symbolic link or junction",
            ));
        }
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create bundle directory",
                relative.to_owned(),
                &error,
            ))
        })?;
    }
    Ok(path)
}

fn write_relative(root: &Path, relative: &str, bytes: &[u8]) -> AppResult<()> {
    if !is_safe_relative_path(relative) {
        return Err(path_error(
            &root.join(relative),
            "bundle path is not a safe normalized relative path",
        ));
    }
    let path = root.join(path_from_bundle(relative));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create bundle directory",
                "bundle directory",
                &error,
            ))
        })?;
    }
    fs::write(&path, bytes).map_err(|error| {
        Box::new(AppError::io(
            "write bundle payload",
            relative.to_owned(),
            &error,
        ))
    })
}

fn inventory(
    root: &Path,
    roles: &BTreeMap<String, String>,
    include_manifest: bool,
) -> AppResult<Vec<BundleFileRecord>> {
    let files = collect_relative_files(root)?;
    let mut records = Vec::with_capacity(files.len());
    for relative in files {
        if !include_manifest && relative == "manifest.json" {
            continue;
        }
        let path = root.join(path_from_bundle(&relative));
        let (size_bytes, sha256) = hash_file_streaming(&path)?;
        records.push(BundleFileRecord {
            role: roles
                .get(&relative)
                .cloned()
                .unwrap_or_else(|| "payload".to_owned()),
            path: relative,
            size_bytes,
            sha256,
        });
    }
    records.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(records)
}

fn hash_file_streaming(path: &Path) -> AppResult<(u64, String)> {
    let file = fs::File::open(path).map_err(|error| {
        Box::new(AppError::io(
            "read bundle payload",
            "bundle payload",
            &error,
        ))
    })?;
    let mut reader = BufReader::new(file);
    let mut hasher = sha2::Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| {
            Box::new(AppError::io(
                "hash bundle payload",
                "bundle payload",
                &error,
            ))
        })?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| size_limit_error("bundle payload size overflow"))?;
        if total > crate::model::MAX_BUNDLE_BYTES {
            return Err(size_limit_error(
                "bundle payload exceeds the v1 byte budget",
            ));
        }
        hasher.update(&buffer[..count]);
    }
    Ok((total, format!("{:x}", hasher.finalize())))
}

fn collect_relative_files(root: &Path) -> AppResult<BTreeSet<String>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeSet<String>) -> AppResult<()> {
        let mut entries = fs::read_dir(current)
            .map_err(|error| {
                Box::new(AppError::io(
                    "list bundle directory",
                    "bundle directory",
                    &error,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                Box::new(AppError::io(
                    "read bundle entry",
                    "bundle directory",
                    &error,
                ))
            })?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let file_type = entry.file_type().map_err(|error| {
                Box::new(AppError::io("inspect bundle entry", "bundle entry", &error))
            })?;
            if file_type.is_symlink() {
                return Err(path_error(
                    &entry.path(),
                    "symbolic links are forbidden in bundles",
                ));
            }
            if file_type.is_dir() {
                visit(root, &entry.path(), output)?;
            } else if file_type.is_file() {
                let entry_path = entry.path();
                let relative = entry_path.strip_prefix(root).map_err(|error| {
                    path_error(
                        &entry_path,
                        format!("cannot relativize bundle path: {error}"),
                    )
                })?;
                output.insert(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut output = BTreeSet::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn roles(files: &[PlannedFile]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|file| (file.path.clone(), file.role.clone()))
        .collect()
}

fn enforce_planned_file_limits(files: &[PlannedFile]) -> AppResult<()> {
    if files.len() > MAX_BUNDLE_FILES {
        return Err(size_limit_error(format!(
            "{} payload files exceed limit {MAX_BUNDLE_FILES}",
            files.len()
        )));
    }
    let total = files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.size_bytes));
    if total.is_none_or(|value| value > MAX_BUNDLE_BYTES) {
        return Err(size_limit_error(format!(
            "planned payload size exceeds {MAX_BUNDLE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn enforce_limits(records: &[BundleFileRecord]) -> AppResult<()> {
    if records.len() > MAX_BUNDLE_FILES {
        return Err(size_limit_error(format!(
            "{} files exceed limit {MAX_BUNDLE_FILES}",
            records.len()
        )));
    }
    let total = records
        .iter()
        .try_fold(0_u64, |total, record| total.checked_add(record.size_bytes));
    if total.is_none_or(|value| value > MAX_BUNDLE_BYTES) {
        return Err(size_limit_error(format!(
            "payload size exceeds {MAX_BUNDLE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') || value.starts_with('/') || value.contains(':') {
        return false;
    }
    let path = Path::new(value);
    path.components().all(|component| {
        matches!(component, Component::Normal(_))
            && component
                .as_os_str()
                .to_str()
                .is_some_and(|part| part != "." && part != "..")
    })
}

fn path_from_bundle(value: &str) -> PathBuf {
    value.split('/').collect()
}

fn path_error(path: &Path, detail: impl Into<String>) -> Box<AppError> {
    let label = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("destination");
    Box::new(
        AppError::new(
            "MIGRATION_DESTINATION_UNSAFE",
            "La destination du bundle n'est pas sûre.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(label)
        .with_import_stage("area_migration_path_validation")
        .with_suggestion("Choisissez un nouveau dossier local vide, hors des sources NWN."),
    )
}

fn check_cancelled(cancelled: &AtomicBool, resource: &str) -> AppResult<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(Box::new(AppError::job_cancelled(resource)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CoordinateConvention, MigrationCounts};

    #[test]
    fn accepts_only_normalized_relative_payload_paths() {
        assert!(is_safe_relative_path("assets/models/model-a.glb"));
        assert!(!is_safe_relative_path("../escape.json"));
        assert!(!is_safe_relative_path("C:/absolute.json"));
        assert!(!is_safe_relative_path("assets\\model.glb"));
    }

    #[test]
    fn destination_must_be_new_and_must_not_traverse() {
        let root = tempfile::tempdir().expect("root");
        let valid = validate_bundle_destination(root.path().join("bundle")).expect("valid");
        assert_eq!(
            valid.file_name().and_then(|value| value.to_str()),
            Some("bundle")
        );
        let traversal = root.path().join("nested").join("..").join("bundle");
        let error = validate_bundle_destination(traversal).expect_err("traversal");
        assert_eq!(error.code, "MIGRATION_DESTINATION_UNSAFE");
        let existing = root.path().join("existing");
        fs::create_dir(&existing).expect("existing");
        assert!(validate_bundle_destination(existing).is_err());
    }

    #[test]
    fn cancellation_before_publish_removes_the_atomic_staging_directory() {
        let root = tempfile::tempdir().expect("root");
        let scratch = tempfile::tempdir().expect("scratch");
        let destination = root.path().join("cancelled.area-migration-v1");
        let cancelled = AtomicBool::new(false);
        let counts = MigrationCounts::default();
        let error = write_bundle_atomic(
            &destination,
            &[],
            &cancelled,
            BundleDraft {
                area: MigrationAreaDocument {
                    schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
                    resref: "area_a".to_owned(),
                    name: "Synthetic".to_owned(),
                    area_kind: "unknown".to_owned(),
                    dimensions: [1, 1],
                    grid_size_meters: 10.0,
                    tileset: None,
                    tiles: Vec::new(),
                    instances: Vec::new(),
                    assets: Vec::new(),
                    source_files: vec!["area_a.are".to_owned()],
                },
                identity_map: IdentityMap {
                    schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
                    module_sha256: "a".repeat(64),
                    area_resref: "area_a".to_owned(),
                    entries: Vec::new(),
                },
                diagnostics: Vec::new(),
                report: MigrationReport {
                    schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
                    area_resref: "area_a".to_owned(),
                    complete: true,
                    counts: counts.clone(),
                    navigation_converted: false,
                    navigation_status: "preserved-not-converted".to_owned(),
                    diagnostics_file: "diagnostics.jsonl".to_owned(),
                    bundle_is_local_only: true,
                    source_module_immutable: true,
                    payload_file_count: 0,
                    payload_size_bytes: 0,
                },
                manifest: BundleManifest {
                    schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
                    exporter: "test".to_owned(),
                    module_sha256: "a".repeat(64),
                    module_size_bytes: 1,
                    area_resref: "area_a".to_owned(),
                    classification: "local_only_proprietary".to_owned(),
                    redistribution: "not_redistributable_without_separate_rights".to_owned(),
                    coordinate_system: CoordinateConvention::default(),
                    resolution_policy: "test".to_owned(),
                    dependencies: Vec::new(),
                    resources: Vec::new(),
                    counts,
                    files: Vec::new(),
                    integrity_scope: "test".to_owned(),
                },
                asset_files: vec![
                    crate::assets::materialize_file(
                        scratch.path(),
                        "assets/models/synthetic.glb".to_owned(),
                        "model",
                        b"synthetic GLB fixture".to_vec(),
                        &mut crate::assets::MaterializationBudget::default(),
                    )
                    .expect("scratch fixture"),
                ],
            },
            || {
                cancelled.store(true, Ordering::Relaxed);
                Ok(())
            },
        )
        .expect_err("cancelled before atomic rename");

        assert_eq!(error.code, "JOB_CANCELLED");
        assert!(!destination.exists());
        let leftovers = fs::read_dir(root.path())
            .expect("list root")
            .collect::<Result<Vec<_>, _>>()
            .expect("entries");
        assert!(leftovers.is_empty(), "staging directory must be cleaned");
    }
}
