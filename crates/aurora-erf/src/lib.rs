use aurora_core::{AppError, AppResult, ErrorSeverity};
pub use aurora_core::{ResourceKey, resource_extension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const ERF_HEADER_SIZE: u64 = 160;
const ERF_KEY_SIZE: u64 = 24;
const ERF_RESOURCE_SIZE: u64 = 8;
const DEFAULT_MAX_ENTRIES: u32 = 250_000;
const DEFAULT_MAX_RESOURCE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerResource {
    pub key: ResourceKey,
    pub resource_id: u32,
    pub extension: Option<String>,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTypeSummary {
    pub resource_type: u16,
    pub extension: Option<String>,
    pub count: u32,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerInventory {
    pub file_type: String,
    pub file_version: String,
    pub build_year: u32,
    pub build_day: u32,
    pub resource_count: u32,
    pub resources: Vec<ContainerResource>,
    pub type_summaries: Vec<ResourceTypeSummary>,
}

pub trait ContainerReader {
    fn read_inventory(&self, path: &Path, cancelled: &AtomicBool) -> AppResult<ContainerInventory>;
}

#[derive(Debug, Clone, Copy)]
pub struct ErfReader {
    max_entries: u32,
}

impl Default for ErfReader {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

impl ErfReader {
    pub fn with_max_entries(max_entries: u32) -> Self {
        Self { max_entries }
    }

    pub fn read_resource(
        &self,
        path: &Path,
        resource: &ContainerResource,
        cancelled: &AtomicBool,
    ) -> AppResult<Vec<u8>> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(display(path)).into());
        }
        if resource.size > DEFAULT_MAX_RESOURCE_BYTES {
            return Err(format_error(
                path,
                "ERF_RESOURCE_LIMIT_EXCEEDED",
                format!(
                    "Resource {}.{} is {} bytes; in-memory limit is {DEFAULT_MAX_RESOURCE_BYTES}",
                    resource.key.resref,
                    resource.extension.as_deref().unwrap_or("unknown"),
                    resource.size
                ),
            ));
        }

        let mut file = File::open(path)
            .map_err(|error| AppError::io("open ERF resource", display(path), &error))?;
        let file_size = file
            .metadata()
            .map_err(|error| AppError::io("read ERF metadata", display(path), &error))?
            .len();
        ensure_range(
            path,
            "ERF_RESOURCE_OUT_OF_BOUNDS",
            resource.offset,
            resource.size,
            file_size,
        )?;
        read_range(&mut file, path, resource.offset, resource.size)
    }
}

impl ContainerReader for ErfReader {
    fn read_inventory(&self, path: &Path, cancelled: &AtomicBool) -> AppResult<ContainerInventory> {
        let mut file = File::open(path)
            .map_err(|error| AppError::io("open ERF container", display(path), &error))?;
        let file_size = file
            .metadata()
            .map_err(|error| AppError::io("read ERF metadata", display(path), &error))?
            .len();
        ensure_range(path, "ERF_HEADER_TOO_SHORT", 0, ERF_HEADER_SIZE, file_size)?;

        let header_bytes = read_range(&mut file, path, 0, ERF_HEADER_SIZE)?;
        let header = ErfHeader::parse(&header_bytes);
        validate_header(path, &header, file_size, self.max_entries)?;

        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(display(path)).into());
        }

        let key_table_size = table_size(path, header.entry_count, ERF_KEY_SIZE, "key")?;
        let resource_table_size =
            table_size(path, header.entry_count, ERF_RESOURCE_SIZE, "resource")?;
        let keys = read_range(&mut file, path, header.key_offset, key_table_size)?;
        let resource_table =
            read_range(&mut file, path, header.resource_offset, resource_table_size)?;

        let mut resources = Vec::with_capacity(header.entry_count as usize);
        let mut summaries = BTreeMap::<u16, (u32, u64)>::new();
        for index in 0..header.entry_count as usize {
            if index % 1024 == 0 && cancelled.load(Ordering::Relaxed) {
                return Err(AppError::job_cancelled(display(path)).into());
            }

            let key_start = index * ERF_KEY_SIZE as usize;
            let key = &keys[key_start..key_start + ERF_KEY_SIZE as usize];
            let resource_id = little_u32(key, 16);
            let resource_type = little_u16(key, 20);
            if resource_id >= header.entry_count {
                return Err(format_error(
                    path,
                    "ERF_RESOURCE_ID_INVALID",
                    format!(
                        "Key {index} references resource id {resource_id}, but the table has {} entries",
                        header.entry_count
                    ),
                ));
            }

            let resource_start = resource_id as usize * ERF_RESOURCE_SIZE as usize;
            let record =
                &resource_table[resource_start..resource_start + ERF_RESOURCE_SIZE as usize];
            let offset = u64::from(little_u32(record, 0));
            let size = u64::from(little_u32(record, 4));
            ensure_range(path, "ERF_RESOURCE_OUT_OF_BOUNDS", offset, size, file_size)?;

            let extension = resource_extension(resource_type).map(str::to_owned);
            resources.push(ContainerResource {
                key: ResourceKey::new(decode_resref(&key[..16]), resource_type),
                resource_id,
                extension: extension.clone(),
                offset,
                size,
            });
            let summary = summaries.entry(resource_type).or_default();
            summary.0 += 1;
            summary.1 = summary.1.checked_add(size).ok_or_else(|| {
                format_error(
                    path,
                    "ERF_SIZE_OVERFLOW",
                    format!("Total size overflow for resource type {resource_type}"),
                )
            })?;
        }

        let type_summaries = summaries
            .into_iter()
            .map(|(resource_type, (count, total_size))| ResourceTypeSummary {
                resource_type,
                extension: resource_extension(resource_type).map(str::to_owned),
                count,
                total_size,
            })
            .collect();

        Ok(ContainerInventory {
            file_type: header.file_type,
            file_version: header.file_version,
            build_year: header.build_year.saturating_add(1900),
            build_day: header.build_day,
            resource_count: header.entry_count,
            resources,
            type_summaries,
        })
    }
}

#[derive(Debug)]
struct ErfHeader {
    file_type: String,
    file_version: String,
    localized_string_size: u64,
    entry_count: u32,
    localized_string_offset: u64,
    key_offset: u64,
    resource_offset: u64,
    build_year: u32,
    build_day: u32,
}

impl ErfHeader {
    fn parse(bytes: &[u8]) -> Self {
        Self {
            file_type: String::from_utf8_lossy(&bytes[0..4]).into_owned(),
            file_version: String::from_utf8_lossy(&bytes[4..8]).into_owned(),
            localized_string_size: u64::from(little_u32(bytes, 12)),
            entry_count: little_u32(bytes, 16),
            localized_string_offset: u64::from(little_u32(bytes, 20)),
            key_offset: u64::from(little_u32(bytes, 24)),
            resource_offset: u64::from(little_u32(bytes, 28)),
            build_year: little_u32(bytes, 32),
            build_day: little_u32(bytes, 36),
        }
    }
}

fn validate_header(
    path: &Path,
    header: &ErfHeader,
    file_size: u64,
    max_entries: u32,
) -> AppResult<()> {
    if !matches!(
        header.file_type.as_str(),
        "ERF " | "HAK " | "MOD " | "NWM " | "SAV "
    ) {
        return Err(format_error(
            path,
            "ERF_UNSUPPORTED_TYPE",
            format!("Unsupported ERF file type {:?}", header.file_type),
        ));
    }
    if header.file_version != "V1.0" {
        return Err(format_error(
            path,
            "ERF_UNSUPPORTED_VERSION",
            format!(
                "Unsupported ERF version {:?}; expected V1.0",
                header.file_version
            ),
        ));
    }
    if header.entry_count > max_entries {
        return Err(format_error(
            path,
            "ERF_ENTRY_LIMIT_EXCEEDED",
            format!(
                "Container declares {} entries; configured limit is {max_entries}",
                header.entry_count
            ),
        ));
    }

    ensure_range(
        path,
        "ERF_LOCALIZED_STRINGS_OUT_OF_BOUNDS",
        header.localized_string_offset,
        header.localized_string_size,
        file_size,
    )?;
    let key_size = table_size(path, header.entry_count, ERF_KEY_SIZE, "key")?;
    let resource_size = table_size(path, header.entry_count, ERF_RESOURCE_SIZE, "resource")?;
    ensure_range(
        path,
        "ERF_KEY_TABLE_OUT_OF_BOUNDS",
        header.key_offset,
        key_size,
        file_size,
    )?;
    ensure_range(
        path,
        "ERF_RESOURCE_TABLE_OUT_OF_BOUNDS",
        header.resource_offset,
        resource_size,
        file_size,
    )
}

fn read_range(file: &mut File, path: &Path, offset: u64, size: u64) -> AppResult<Vec<u8>> {
    let allocation_size = usize::try_from(size).map_err(|_| {
        format_error(
            path,
            "ERF_ALLOCATION_LIMIT_EXCEEDED",
            format!("Cannot allocate a {size}-byte ERF table on this platform"),
        )
    })?;
    let mut bytes = vec![0_u8; allocation_size];
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| AppError::io("seek ERF container", display(path), &error))?;
    file.read_exact(&mut bytes)
        .map_err(|error| AppError::io("read ERF container", display(path), &error))?;
    Ok(bytes)
}

fn table_size(path: &Path, count: u32, record_size: u64, table: &str) -> AppResult<u64> {
    u64::from(count).checked_mul(record_size).ok_or_else(|| {
        format_error(
            path,
            "ERF_TABLE_SIZE_OVERFLOW",
            format!("The {table} table size overflows"),
        )
    })
}

fn ensure_range(path: &Path, code: &str, offset: u64, size: u64, file_size: u64) -> AppResult<()> {
    let end = offset.checked_add(size).ok_or_else(|| {
        format_error(
            path,
            code,
            format!("Range offset {offset} plus size {size} overflows"),
        )
    })?;
    if end > file_size {
        return Err(format_error(
            path,
            code,
            format!("Range {offset}..{end} exceeds file size {file_size}"),
        ));
    }
    Ok(())
}

fn format_error(path: &Path, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Le conteneur NWN est invalide ou n'est pas pris en charge.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(display(path))
        .with_import_stage("erf_inventory")
        .with_suggestion("Vérifiez la copie du module et consultez les diagnostics techniques."),
    )
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn little_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn little_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn decode_resref(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    bytes[..end]
        .iter()
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' {
                char::from(*byte).to_string()
            } else {
                format!("\\x{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reads_metadata_without_loading_resource_payloads() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("fixture.mod");
        fs::write(&module, synthetic_erf()).expect("write ERF fixture");

        let inventory = ErfReader::default()
            .read_inventory(&module, &AtomicBool::new(false))
            .expect("valid inventory");

        assert_eq!(inventory.file_type, "MOD ");
        assert_eq!(inventory.file_version, "V1.0");
        assert_eq!(inventory.build_year, 2026);
        assert_eq!(inventory.resource_count, 2);
        assert_eq!(inventory.resources[0].key.resref, "module");
        assert_eq!(inventory.resources[0].extension.as_deref(), Some("ifo"));
        assert_eq!(inventory.resources[1].size, 3);
    }

    #[test]
    fn rejects_a_resource_outside_the_file() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("truncated.mod");
        let mut bytes = synthetic_erf();
        let resource_table = ERF_HEADER_SIZE as usize + 2 * ERF_KEY_SIZE as usize;
        put_u32(&mut bytes, resource_table + 4, u32::MAX);
        fs::write(&module, bytes).expect("write ERF fixture");

        let error = ErfReader::default()
            .read_inventory(&module, &AtomicBool::new(false))
            .expect_err("invalid resource range");

        assert_eq!(error.code, "ERF_RESOURCE_OUT_OF_BOUNDS");
    }

    #[test]
    fn rejects_unsupported_versions_and_entry_bombs() {
        let root = tempdir().expect("temporary directory");
        let versioned = root.path().join("version.mod");
        let mut version_bytes = synthetic_erf();
        version_bytes[4..8].copy_from_slice(b"V9.9");
        fs::write(&versioned, version_bytes).expect("write version fixture");
        let version_error = ErfReader::default()
            .read_inventory(&versioned, &AtomicBool::new(false))
            .expect_err("unsupported version");
        assert_eq!(version_error.code, "ERF_UNSUPPORTED_VERSION");

        let bomb = root.path().join("bomb.mod");
        let mut bomb_bytes = synthetic_erf();
        put_u32(&mut bomb_bytes, 16, 3);
        fs::write(&bomb, bomb_bytes).expect("write bomb fixture");
        let bomb_error = ErfReader::with_max_entries(2)
            .read_inventory(&bomb, &AtomicBool::new(false))
            .expect_err("entry limit");
        assert_eq!(bomb_error.code, "ERF_ENTRY_LIMIT_EXCEEDED");
    }

    #[test]
    fn observes_cancellation_before_reading_tables() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("fixture.mod");
        fs::write(&module, synthetic_erf()).expect("write ERF fixture");

        let error = ErfReader::default()
            .read_inventory(&module, &AtomicBool::new(true))
            .expect_err("cancelled inventory");

        assert_eq!(error.code, "JOB_CANCELLED");
    }

    #[test]
    fn reads_one_bounded_resource_on_demand() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("fixture.mod");
        fs::write(&module, synthetic_erf()).expect("write ERF fixture");
        let reader = ErfReader::default();
        let cancelled = AtomicBool::new(false);
        let inventory = reader
            .read_inventory(&module, &cancelled)
            .expect("valid inventory");

        let payload = reader
            .read_resource(&module, &inventory.resources[0], &cancelled)
            .expect("read one resource");

        assert_eq!(payload, b"IFO!");
    }

    fn synthetic_erf() -> Vec<u8> {
        let entry_count = 2_u32;
        let key_offset = ERF_HEADER_SIZE as usize;
        let resource_offset = key_offset + entry_count as usize * ERF_KEY_SIZE as usize;
        let data_offset = resource_offset + entry_count as usize * ERF_RESOURCE_SIZE as usize;
        let mut bytes = vec![0_u8; data_offset];
        bytes[0..4].copy_from_slice(b"MOD ");
        bytes[4..8].copy_from_slice(b"V1.0");
        put_u32(&mut bytes, 16, entry_count);
        put_u32(&mut bytes, 20, ERF_HEADER_SIZE as u32);
        put_u32(&mut bytes, 24, key_offset as u32);
        put_u32(&mut bytes, 28, resource_offset as u32);
        put_u32(&mut bytes, 32, 126);
        put_u32(&mut bytes, 36, 213);

        put_key(&mut bytes, key_offset, b"module", 0, 2014);
        put_key(
            &mut bytes,
            key_offset + ERF_KEY_SIZE as usize,
            b"start",
            1,
            2009,
        );
        put_u32(&mut bytes, resource_offset, data_offset as u32);
        put_u32(&mut bytes, resource_offset + 4, 4);
        put_u32(&mut bytes, resource_offset + 8, data_offset as u32 + 4);
        put_u32(&mut bytes, resource_offset + 12, 3);
        bytes.extend_from_slice(b"IFO!NSS");
        bytes
    }

    fn put_key(bytes: &mut [u8], offset: usize, name: &[u8], id: u32, resource_type: u16) {
        bytes[offset..offset + name.len()].copy_from_slice(name);
        put_u32(bytes, offset + 16, id);
        bytes[offset + 20..offset + 22].copy_from_slice(&resource_type.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
