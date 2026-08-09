use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey, resource_type_for_extension};
use aurora_erf::{ContainerReader, ErfReader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

mod catalog_cache;

pub use catalog_cache::{ResourceCatalogCacheState, ResourceCatalogCacheSummary};

const KEY_HEADER_SIZE: usize = 64;
const KEY_FILE_RECORD_SIZE: usize = 12;
const KEY_RESOURCE_RECORD_SIZE: usize = 22;
const BIF_HEADER_SIZE: usize = 20;
const BIF_VARIABLE_RECORD_SIZE: usize = 16;
const MAX_KEY_FILES: u32 = 16_384;
const MAX_KEY_RESOURCES: u32 = 2_000_000;
const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSourceKind {
    Development,
    Override,
    Module,
    Hak,
    Patch,
    KeyBif,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResourceLocation {
    File {
        path: String,
    },
    Erf {
        path: String,
        offset: u64,
        size: u64,
    },
    Bif {
        path: String,
        offset: u64,
        size: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceVersion {
    pub key: ResourceKey,
    pub source_kind: ResourceSourceKind,
    pub source_name: String,
    pub source_path: String,
    pub priority: u32,
    pub offset: u64,
    pub size: u64,
    pub sha256: Option<String>,
    pub location: ResourceLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedResource {
    pub key: ResourceKey,
    pub selected: ResourceVersion,
    pub shadowed: Vec<ResourceVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDiagnostic {
    pub code: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCatalog {
    pub entries: Vec<ResolvedResource>,
    pub version_count: usize,
    pub shadowed_count: usize,
    pub diagnostics: Vec<ResourceDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePage {
    pub items: Vec<ResolvedResource>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCatalogSummary {
    pub resource_count: usize,
    pub version_count: usize,
    pub shadowed_count: usize,
    pub diagnostic_count: usize,
    pub type_counts: Vec<ResourceTypeCount>,
    pub source_counts: Vec<ResourceSourceCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTypeCount {
    pub resource_type: u16,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSourceCount {
    pub source: ResourceSourceKind,
    pub count: usize,
}

impl ResourceCatalog {
    pub fn summary(&self) -> ResourceCatalogSummary {
        let mut types = BTreeMap::<u16, usize>::new();
        let mut sources = BTreeMap::<ResourceSourceKind, usize>::new();
        for entry in &self.entries {
            *types.entry(entry.key.resource_type).or_default() += 1;
            *sources.entry(entry.selected.source_kind).or_default() += 1;
        }
        ResourceCatalogSummary {
            resource_count: self.entries.len(),
            version_count: self.version_count,
            shadowed_count: self.shadowed_count,
            diagnostic_count: self.diagnostics.len(),
            type_counts: types
                .into_iter()
                .map(|(resource_type, count)| ResourceTypeCount {
                    resource_type,
                    count,
                })
                .collect(),
            source_counts: sources
                .into_iter()
                .map(|(source, count)| ResourceSourceCount { source, count })
                .collect(),
        }
    }

    pub fn get(&self, key: &ResourceKey) -> Option<&ResolvedResource> {
        self.entries.iter().find(|entry| entry.key == *key)
    }

    pub fn search(
        &self,
        query: &str,
        resource_type: Option<u16>,
        source: Option<ResourceSourceKind>,
        offset: usize,
        limit: usize,
    ) -> ResourcePage {
        self.search_many(
            query,
            resource_type
                .map(|value| vec![value])
                .as_deref()
                .unwrap_or_default(),
            source,
            offset,
            limit,
        )
    }

    pub fn search_many(
        &self,
        query: &str,
        resource_types: &[u16],
        source: Option<ResourceSourceKind>,
        offset: usize,
        limit: usize,
    ) -> ResourcePage {
        let query = query.trim().to_ascii_lowercase();
        let matches = self.entries.iter().filter(|entry| {
            (query.is_empty() || entry.key.file_name().contains(&query))
                && (resource_types.is_empty() || resource_types.contains(&entry.key.resource_type))
                && source.is_none_or(|value| entry.selected.source_kind == value)
        });
        let total = matches.clone().count();
        let items = matches
            .skip(offset)
            .take(limit.clamp(1, 500))
            .cloned()
            .collect();
        ResourcePage {
            items,
            offset,
            limit: limit.clamp(1, 500),
            total,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceManagerConfig {
    pub module_path: PathBuf,
    pub hak_paths: Vec<PathBuf>,
    pub game_install_path: Option<PathBuf>,
    pub user_data_path: Option<PathBuf>,
}

pub struct ResourceManager;

#[derive(Debug, Clone)]
pub struct ResourceCatalogBuild {
    pub catalog: ResourceCatalog,
    pub cache: ResourceCatalogCacheSummary,
}

impl ResourceManager {
    pub fn build(
        config: &ResourceManagerConfig,
        cancelled: &AtomicBool,
    ) -> AppResult<ResourceCatalog> {
        Ok(Self::build_with_cache(config, None, cancelled)?.catalog)
    }

    pub fn build_with_cache(
        config: &ResourceManagerConfig,
        cache_path: Option<&Path>,
        cancelled: &AtomicBool,
    ) -> AppResult<ResourceCatalogBuild> {
        let mut versions = BTreeMap::<ResourceKey, Vec<ResourceVersion>>::new();
        let mut diagnostics = Vec::new();

        if let Some(user) = &config.user_data_path {
            scan_directory(
                &user.join("development"),
                ResourceSourceKind::Development,
                0,
                &mut versions,
                &mut diagnostics,
                cancelled,
            )?;
            scan_directory(
                &user.join("override"),
                ResourceSourceKind::Override,
                10,
                &mut versions,
                &mut diagnostics,
                cancelled,
            )?;
        }

        scan_erf(
            &config.module_path,
            ErfScan {
                kind: ResourceSourceKind::Module,
                source_name: "module".into(),
                priority: 20,
                required: true,
            },
            &mut versions,
            &mut diagnostics,
            cancelled,
        )?;
        for (index, path) in config.hak_paths.iter().enumerate() {
            scan_erf(
                path,
                ErfScan {
                    kind: ResourceSourceKind::Hak,
                    source_name: format!("hak:{index}"),
                    priority: 30 + index as u32,
                    required: false,
                },
                &mut versions,
                &mut diagnostics,
                cancelled,
            )?;
        }

        let cache = if let Some(game) = &config.game_install_path {
            let language_root = preferred_language_root(game);
            let signature =
                catalog_cache::game_source_signature(game, language_root.as_deref(), cancelled)?;
            let cached = cache_path
                .and_then(|path| catalog_cache::load(path, &signature))
                .map(|catalog| (catalog, ResourceCatalogCacheState::Hit));
            let (game_catalog, state) = match cached {
                Some(value) => value,
                None => {
                    let mut game_versions = BTreeMap::new();
                    let mut game_diagnostics = Vec::new();
                    scan_directory(
                        &game.join("ovr"),
                        ResourceSourceKind::Override,
                        12,
                        &mut game_versions,
                        &mut game_diagnostics,
                        cancelled,
                    )?;
                    if let Some(language_root) = &language_root {
                        scan_directory(
                            &language_root.join("data/ovr"),
                            ResourceSourceKind::Override,
                            11,
                            &mut game_versions,
                            &mut game_diagnostics,
                            cancelled,
                        )?;
                    }
                    scan_keys(game, &mut game_versions, &mut game_diagnostics, cancelled)?;
                    let catalog = finalize_catalog(game_versions, game_diagnostics);
                    if let Some(path) = cache_path {
                        catalog_cache::store(path, &signature, &catalog)?;
                    }
                    (catalog, ResourceCatalogCacheState::Miss)
                }
            };
            let game_resource_count = game_catalog.entries.len();
            merge_catalog(&mut versions, &mut diagnostics, game_catalog);
            ResourceCatalogCacheSummary {
                state,
                signature: Some(signature),
                path: cache_path.map(|path| path.display().to_string()),
                game_resource_count,
            }
        } else {
            ResourceCatalogCacheSummary::default()
        };

        Ok(ResourceCatalogBuild {
            catalog: finalize_catalog(versions, diagnostics),
            cache,
        })
    }

    pub fn read(version: &ResourceVersion, cancelled: &AtomicBool) -> AppResult<Vec<u8>> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(version.key.to_string()).into());
        }
        if version.size > MAX_RESOURCE_BYTES {
            return Err(resource_error(
                "RESOURCE_READ_LIMIT_EXCEEDED",
                &version.source_path,
                format!(
                    "{} declares {} bytes; limit is {MAX_RESOURCE_BYTES}",
                    version.key, version.size
                ),
            ));
        }
        match &version.location {
            ResourceLocation::File { path } => {
                let bytes = fs::read(path)
                    .map_err(|error| AppError::io("read loose resource", path, &error))?;
                if bytes.len() as u64 > MAX_RESOURCE_BYTES {
                    return Err(resource_error(
                        "RESOURCE_READ_LIMIT_EXCEEDED",
                        path,
                        format!("Loose resource is {} bytes", bytes.len()),
                    ));
                }
                Ok(bytes)
            }
            ResourceLocation::Erf { path, offset, size }
            | ResourceLocation::Bif { path, offset, size } => {
                read_range(Path::new(path), *offset, *size)
            }
        }
    }

    pub fn hash(version: &ResourceVersion, cancelled: &AtomicBool) -> AppResult<String> {
        let bytes = Self::read(version, cancelled)?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }

    pub fn extract_to_cache(
        version: &ResourceVersion,
        cache_root: &Path,
        cancelled: &AtomicBool,
    ) -> AppResult<PathBuf> {
        let file_name = version.key.file_name();
        if !matches!(
            Path::new(&file_name)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [Component::Normal(_)]
        ) {
            return Err(resource_error(
                "RESOURCE_CACHE_PATH_INVALID",
                &version.source_path,
                format!("Unsafe resource name {file_name:?}"),
            ));
        }
        fs::create_dir_all(cache_root).map_err(|error| {
            AppError::io(
                "create resource cache",
                cache_root.display().to_string(),
                &error,
            )
        })?;
        let target = cache_root.join(file_name);
        let bytes = Self::read(version, cancelled)?;
        fs::write(&target, bytes).map_err(|error| {
            AppError::io("write resource cache", target.display().to_string(), &error)
        })?;
        Ok(target)
    }
}

fn finalize_catalog(
    versions: BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    diagnostics: Vec<ResourceDiagnostic>,
) -> ResourceCatalog {
    let mut entries = Vec::with_capacity(versions.len());
    let mut version_count = 0;
    let mut shadowed_count = 0;
    for (key, mut candidates) in versions {
        candidates.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.source_path.cmp(&right.source_path))
                .then_with(|| left.offset.cmp(&right.offset))
        });
        version_count += candidates.len();
        shadowed_count += candidates.len().saturating_sub(1);
        let selected = candidates.remove(0);
        entries.push(ResolvedResource {
            key,
            selected,
            shadowed: candidates,
        });
    }
    ResourceCatalog {
        entries,
        version_count,
        shadowed_count,
        diagnostics,
    }
}

fn merge_catalog(
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    diagnostics: &mut Vec<ResourceDiagnostic>,
    catalog: ResourceCatalog,
) {
    diagnostics.extend(catalog.diagnostics);
    for entry in catalog.entries {
        push_version(versions, entry.selected);
        for shadowed in entry.shadowed {
            push_version(versions, shadowed);
        }
    }
}

fn scan_directory(
    directory: &Path,
    kind: ResourceSourceKind,
    priority: u32,
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    diagnostics: &mut Vec<ResourceDiagnostic>,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let read_dir = fs::read_dir(directory).map_err(|error| {
        AppError::io(
            "scan resource directory",
            directory.display().to_string(),
            &error,
        )
    })?;
    for entry in read_dir {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(directory.display().to_string()).into());
        }
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(ResourceDiagnostic {
                    code: "RESOURCE_DIRECTORY_ENTRY_UNREADABLE".into(),
                    message: error.to_string(),
                    source: directory.display().to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(resource_type) = resource_type_for_extension(extension) else {
            continue;
        };
        let Some(resref) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let size = match entry.metadata() {
            Ok(value) => value.len(),
            Err(error) => {
                diagnostics.push(ResourceDiagnostic {
                    code: "RESOURCE_METADATA_UNREADABLE".into(),
                    message: error.to_string(),
                    source: path.display().to_string(),
                });
                continue;
            }
        };
        let key = ResourceKey::new(resref, resource_type);
        push_version(
            versions,
            ResourceVersion {
                key,
                source_kind: kind,
                source_name: directory
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("directory")
                    .to_owned(),
                source_path: path.display().to_string(),
                priority,
                offset: 0,
                size,
                sha256: None,
                location: ResourceLocation::File {
                    path: path.display().to_string(),
                },
            },
        );
    }
    Ok(())
}

struct ErfScan {
    kind: ResourceSourceKind,
    source_name: String,
    priority: u32,
    required: bool,
}

fn scan_erf(
    path: &Path,
    scan: ErfScan,
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    diagnostics: &mut Vec<ResourceDiagnostic>,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    let inventory = match ErfReader::default().read_inventory(path, cancelled) {
        Ok(value) => value,
        Err(error) if !scan.required => {
            diagnostics.push(ResourceDiagnostic {
                code: error.code.clone(),
                message: error.user_message.clone(),
                source: path.display().to_string(),
            });
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    for resource in inventory.resources {
        push_version(
            versions,
            ResourceVersion {
                key: resource.key,
                source_kind: scan.kind,
                source_name: scan.source_name.clone(),
                source_path: path.display().to_string(),
                priority: scan.priority,
                offset: resource.offset,
                size: resource.size,
                sha256: None,
                location: ResourceLocation::Erf {
                    path: path.display().to_string(),
                    offset: resource.offset,
                    size: resource.size,
                },
            },
        );
    }
    Ok(())
}

fn scan_keys(
    game_root: &Path,
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    diagnostics: &mut Vec<ResourceDiagnostic>,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    let mut keys = BTreeSet::new();
    let mut directories = vec![game_root.to_path_buf(), game_root.join("data")];
    if let Some(language_root) = preferred_language_root(game_root) {
        directories.push(language_root.join("data"));
    }
    for directory in directories {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.eq_ignore_ascii_case("key"))
            {
                keys.insert(path);
            }
        }
    }
    for (index, key_path) in keys.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(key_path.display().to_string()).into());
        }
        if let Err(error) = scan_key(
            &key_path,
            game_root,
            key_priority(&key_path, index as u32),
            versions,
            cancelled,
        ) {
            diagnostics.push(ResourceDiagnostic {
                code: error.code.clone(),
                message: error.user_message.clone(),
                source: key_path.display().to_string(),
            });
        }
    }
    Ok(())
}

fn scan_key(
    path: &Path,
    game_root: &Path,
    ordinal: u32,
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    let bytes = fs::read(path)
        .map_err(|error| AppError::io("read KEY", path.display().to_string(), &error))?;
    ensure_slice(path, &bytes, 0, KEY_HEADER_SIZE, "KEY_HEADER_TOO_SHORT")?;
    if &bytes[0..4] != b"KEY " || &bytes[4..8] != b"V1  " {
        return Err(resource_error(
            "KEY_HEADER_UNSUPPORTED",
            &path.display().to_string(),
            "Expected KEY V1".into(),
        ));
    }
    let bif_count = u32_at(&bytes, 8);
    let key_count = u32_at(&bytes, 12);
    if bif_count > MAX_KEY_FILES || key_count > MAX_KEY_RESOURCES {
        return Err(resource_error(
            "KEY_COUNT_LIMIT_EXCEEDED",
            &path.display().to_string(),
            format!("{bif_count} BIF files and {key_count} resources"),
        ));
    }
    let file_offset = u32_at(&bytes, 16) as usize;
    let key_offset = u32_at(&bytes, 20) as usize;
    ensure_slice(
        path,
        &bytes,
        file_offset,
        bif_count as usize * KEY_FILE_RECORD_SIZE,
        "KEY_FILE_TABLE_OUT_OF_BOUNDS",
    )?;
    ensure_slice(
        path,
        &bytes,
        key_offset,
        key_count as usize * KEY_RESOURCE_RECORD_SIZE,
        "KEY_RESOURCE_TABLE_OUT_OF_BOUNDS",
    )?;
    let mut bif_paths = Vec::with_capacity(bif_count as usize);
    for index in 0..bif_count as usize {
        let base = file_offset + index * KEY_FILE_RECORD_SIZE;
        let name_offset = u32_at(&bytes, base + 4) as usize;
        let name_size = u16_at(&bytes, base + 8) as usize;
        ensure_slice(
            path,
            &bytes,
            name_offset,
            name_size,
            "KEY_FILENAME_OUT_OF_BOUNDS",
        )?;
        let name = decode_path(&bytes[name_offset..name_offset + name_size]);
        let relative = PathBuf::from(name.replace('\\', "/"));
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(resource_error(
                "KEY_BIF_PATH_INVALID",
                &path.display().to_string(),
                format!("Unsafe BIF path {relative:?}"),
            ));
        }
        let key_root = path.parent().and_then(Path::parent).unwrap_or(game_root);
        let candidates = [
            key_root.join(&relative),
            game_root.join(&relative),
            path.parent()
                .unwrap_or(game_root)
                .join(relative.file_name().unwrap_or_default()),
        ];
        bif_paths.push(
            candidates
                .into_iter()
                .find(|candidate| candidate.is_file())
                .unwrap_or_else(|| game_root.join(relative)),
        );
    }
    let mut bif_tables = BTreeMap::<usize, BTreeMap<u32, BifResource>>::new();
    for index in 0..key_count as usize {
        if index % 4096 == 0 && cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(path.display().to_string()).into());
        }
        let base = key_offset + index * KEY_RESOURCE_RECORD_SIZE;
        let resref = decode_resref(&bytes[base..base + 16]);
        let resource_type = u16_at(&bytes, base + 16);
        let resource_id = u32_at(&bytes, base + 18);
        let bif_index = (resource_id >> 20) as usize;
        let local_id = resource_id & 0x000f_ffff;
        let Some(bif_path) = bif_paths.get(bif_index) else {
            continue;
        };
        if !bif_path.is_file() {
            continue;
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = bif_tables.entry(bif_index) {
            entry.insert(read_bif_table(bif_path)?);
        }
        let Some(record) = bif_tables
            .get(&bif_index)
            .and_then(|table| table.get(&local_id))
        else {
            continue;
        };
        if record.resource_type != resource_type {
            return Err(resource_error(
                "KEY_BIF_TYPE_MISMATCH",
                &path.display().to_string(),
                format!(
                    "{} declares type {resource_type} in KEY and {} in BIF",
                    resref, record.resource_type
                ),
            ));
        }
        let kind = if path
            .file_name()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.to_ascii_lowercase().contains("patch"))
        {
            ResourceSourceKind::Patch
        } else {
            ResourceSourceKind::KeyBif
        };
        let priority = ordinal;
        let key = ResourceKey::new(resref, resource_type);
        push_version(
            versions,
            ResourceVersion {
                key,
                source_kind: kind,
                source_name: path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("key")
                    .to_owned(),
                source_path: bif_path.display().to_string(),
                priority,
                offset: record.offset,
                size: record.size,
                sha256: None,
                location: ResourceLocation::Bif {
                    path: bif_path.display().to_string(),
                    offset: record.offset,
                    size: record.size,
                },
            },
        );
    }
    Ok(())
}

fn preferred_language_root(game_root: &Path) -> Option<PathBuf> {
    let languages = game_root.join("lang");
    if languages.join("en").is_dir() {
        return Some(languages.join("en"));
    }
    let mut candidates = fs::read_dir(languages)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next()
}

fn key_priority(path: &Path, ordinal: u32) -> u32 {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("patch") {
        100 + ordinal
    } else if name == "nwn_retail_loc" {
        110
    } else if name == "nwn_retail" {
        120
    } else if name == "nwn_base_loc" {
        130
    } else if name == "nwn_base" {
        140
    } else {
        150 + ordinal
    }
}

#[derive(Debug, Clone, Copy)]
struct BifResource {
    offset: u64,
    size: u64,
    resource_type: u16,
}

fn read_bif_table(path: &Path) -> AppResult<BTreeMap<u32, BifResource>> {
    let bytes = fs::read(path)
        .map_err(|error| AppError::io("read BIF", path.display().to_string(), &error))?;
    ensure_slice(path, &bytes, 0, BIF_HEADER_SIZE, "BIF_HEADER_TOO_SHORT")?;
    if &bytes[0..4] != b"BIFF" || &bytes[4..8] != b"V1  " {
        return Err(resource_error(
            "BIF_HEADER_UNSUPPORTED",
            &path.display().to_string(),
            "Expected BIFF V1".into(),
        ));
    }
    let count = u32_at(&bytes, 8);
    let fixed_count = u32_at(&bytes, 12);
    if fixed_count > 0 {
        return Err(resource_error(
            "BIF_FIXED_RESOURCES_UNSUPPORTED",
            &path.display().to_string(),
            format!("BIF contains {fixed_count} fixed resources"),
        ));
    }
    if count > MAX_KEY_RESOURCES {
        return Err(resource_error(
            "BIF_COUNT_LIMIT_EXCEEDED",
            &path.display().to_string(),
            format!("{count} variable resources"),
        ));
    }
    let offset = u32_at(&bytes, 16) as usize;
    ensure_slice(
        path,
        &bytes,
        offset,
        count as usize * BIF_VARIABLE_RECORD_SIZE,
        "BIF_TABLE_OUT_OF_BOUNDS",
    )?;
    let mut resources = BTreeMap::new();
    for index in 0..count as usize {
        let base = offset + index * BIF_VARIABLE_RECORD_SIZE;
        let id = u32_at(&bytes, base) & 0x000f_ffff;
        let resource_offset = u32_at(&bytes, base + 4) as u64;
        let size = u32_at(&bytes, base + 8) as u64;
        let resource_type = u16_at(&bytes, base + 12);
        if resource_offset
            .checked_add(size)
            .is_some_and(|end| end <= bytes.len() as u64)
        {
            resources.insert(
                id,
                BifResource {
                    offset: resource_offset,
                    size,
                    resource_type,
                },
            );
        }
    }
    Ok(resources)
}

fn push_version(
    versions: &mut BTreeMap<ResourceKey, Vec<ResourceVersion>>,
    version: ResourceVersion,
) {
    versions
        .entry(version.key.clone())
        .or_default()
        .push(version);
}

fn read_range(path: &Path, offset: u64, size: u64) -> AppResult<Vec<u8>> {
    if size > MAX_RESOURCE_BYTES {
        return Err(resource_error(
            "RESOURCE_READ_LIMIT_EXCEEDED",
            &path.display().to_string(),
            format!("{size} bytes"),
        ));
    }
    let mut file = File::open(path).map_err(|error| {
        AppError::io("open resource source", path.display().to_string(), &error)
    })?;
    let file_size = file
        .metadata()
        .map_err(|error| {
            AppError::io(
                "read resource source metadata",
                path.display().to_string(),
                &error,
            )
        })?
        .len();
    if offset.checked_add(size).is_none_or(|end| end > file_size) {
        return Err(resource_error(
            "RESOURCE_RANGE_OUT_OF_BOUNDS",
            &path.display().to_string(),
            format!("{offset}+{size} exceeds {file_size}"),
        ));
    }
    file.seek(SeekFrom::Start(offset)).map_err(|error| {
        AppError::io("seek resource source", path.display().to_string(), &error)
    })?;
    let mut bytes = vec![0; size as usize];
    file.read_exact(&mut bytes).map_err(|error| {
        AppError::io("read resource source", path.display().to_string(), &error)
    })?;
    Ok(bytes)
}

fn ensure_slice(
    path: &Path,
    bytes: &[u8],
    offset: usize,
    size: usize,
    code: &str,
) -> AppResult<()> {
    if offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
        return Err(resource_error(
            code,
            &path.display().to_string(),
            format!("{offset}+{size} exceeds {}", bytes.len()),
        ));
    }
    Ok(())
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
fn decode_resref(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.iter().position(|v| *v == 0).unwrap_or(bytes.len())])
        .into_owned()
}
fn decode_path(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_matches('\0').to_owned()
}

fn resource_error(code: &str, source: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Une source de ressources NWN est invalide ou illisible.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(source)
        .with_import_stage("resource_manager")
        .with_suggestion("Consultez la provenance et les diagnostics de cette source."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loose_development_resources_shadow_override_resources() {
        let root = tempdir().expect("temp");
        let user = root.path().join("user");
        fs::create_dir_all(user.join("development")).expect("development");
        fs::create_dir_all(user.join("override")).expect("override");
        fs::write(user.join("development/shared.2da"), b"development").expect("dev file");
        fs::write(user.join("override/shared.2da"), b"override").expect("override file");
        let mut versions = BTreeMap::new();
        let mut diagnostics = Vec::new();
        scan_directory(
            &user.join("development"),
            ResourceSourceKind::Development,
            0,
            &mut versions,
            &mut diagnostics,
            &AtomicBool::new(false),
        )
        .expect("scan dev");
        scan_directory(
            &user.join("override"),
            ResourceSourceKind::Override,
            10,
            &mut versions,
            &mut diagnostics,
            &AtomicBool::new(false),
        )
        .expect("scan override");
        let candidates = versions
            .get(&ResourceKey::new("shared", 2017))
            .expect("shared resource");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].source_kind, ResourceSourceKind::Development);
    }

    #[test]
    fn search_is_bounded_and_keeps_shadowed_provenance() {
        let key = ResourceKey::new("shared", 2017);
        let version = ResourceVersion {
            key: key.clone(),
            source_kind: ResourceSourceKind::Module,
            source_name: "module".into(),
            source_path: "fixture.mod".into(),
            priority: 20,
            offset: 4,
            size: 8,
            sha256: None,
            location: ResourceLocation::Erf {
                path: "fixture.mod".into(),
                offset: 4,
                size: 8,
            },
        };
        let catalog = ResourceCatalog {
            entries: vec![ResolvedResource {
                key,
                selected: version,
                shadowed: Vec::new(),
            }],
            version_count: 1,
            shadowed_count: 0,
            diagnostics: Vec::new(),
        };
        assert_eq!(catalog.search("sha", None, None, 0, 20).total, 1);
        assert_eq!(catalog.search("", None, None, 0, 0).limit, 1);
    }

    #[test]
    fn key_and_bif_records_keep_offsets_and_can_be_read_on_demand() {
        let root = tempdir().expect("temp");
        let game = root.path().join("game");
        fs::create_dir_all(game.join("data")).expect("data");
        let bif_path = game.join("data/base.bif");
        let mut bif = vec![0_u8; BIF_HEADER_SIZE + BIF_VARIABLE_RECORD_SIZE + 3];
        bif[0..4].copy_from_slice(b"BIFF");
        bif[4..8].copy_from_slice(b"V1  ");
        bif[8..12].copy_from_slice(&1_u32.to_le_bytes());
        bif[16..20].copy_from_slice(&(BIF_HEADER_SIZE as u32).to_le_bytes());
        bif[24..28]
            .copy_from_slice(&((BIF_HEADER_SIZE + BIF_VARIABLE_RECORD_SIZE) as u32).to_le_bytes());
        bif[28..32].copy_from_slice(&3_u32.to_le_bytes());
        bif[32..36].copy_from_slice(&2017_u32.to_le_bytes());
        bif[BIF_HEADER_SIZE + BIF_VARIABLE_RECORD_SIZE..].copy_from_slice(b"2DA");
        fs::write(&bif_path, &bif).expect("BIF");

        let key_path = game.join("data/chitin.key");
        let bif_name = b"data\\base.bif\0";
        let file_offset = KEY_HEADER_SIZE;
        let key_offset = file_offset + KEY_FILE_RECORD_SIZE;
        let name_offset = key_offset + KEY_RESOURCE_RECORD_SIZE;
        let mut key = vec![0_u8; name_offset + bif_name.len()];
        key[0..4].copy_from_slice(b"KEY ");
        key[4..8].copy_from_slice(b"V1  ");
        key[8..12].copy_from_slice(&1_u32.to_le_bytes());
        key[12..16].copy_from_slice(&1_u32.to_le_bytes());
        key[16..20].copy_from_slice(&(file_offset as u32).to_le_bytes());
        key[20..24].copy_from_slice(&(key_offset as u32).to_le_bytes());
        key[file_offset..file_offset + 4].copy_from_slice(&(bif.len() as u32).to_le_bytes());
        key[file_offset + 4..file_offset + 8].copy_from_slice(&(name_offset as u32).to_le_bytes());
        key[file_offset + 8..file_offset + 10]
            .copy_from_slice(&(bif_name.len() as u16).to_le_bytes());
        key[key_offset..key_offset + 6].copy_from_slice(b"shared");
        key[key_offset + 16..key_offset + 18].copy_from_slice(&2017_u16.to_le_bytes());
        key[name_offset..].copy_from_slice(bif_name);
        fs::write(&key_path, key).expect("KEY");

        let mut versions = BTreeMap::new();
        scan_key(&key_path, &game, 0, &mut versions, &AtomicBool::new(false))
            .expect("scan KEY/BIF");
        let version = &versions[&ResourceKey::new("shared", 2017)][0];
        assert_eq!(version.source_kind, ResourceSourceKind::KeyBif);
        assert_eq!(
            ResourceManager::read(version, &AtomicBool::new(false)).expect("read BIF resource"),
            b"2DA"
        );
    }

    #[test]
    fn persistent_game_catalog_cache_hits_and_invalidates() {
        let root = tempdir().expect("temp");
        let game = root.path().join("game");
        fs::create_dir_all(game.join("ovr")).expect("override directory");
        let loose = game.join("ovr/shared.2da");
        fs::write(&loose, b"first").expect("loose resource");
        let module = root.path().join("fixture.mod");
        let mut module_bytes = vec![0_u8; 160];
        module_bytes[0..4].copy_from_slice(b"MOD ");
        module_bytes[4..8].copy_from_slice(b"V1.0");
        module_bytes[20..24].copy_from_slice(&160_u32.to_le_bytes());
        module_bytes[24..28].copy_from_slice(&160_u32.to_le_bytes());
        module_bytes[28..32].copy_from_slice(&160_u32.to_le_bytes());
        fs::write(&module, module_bytes).expect("module");
        let cache = root.path().join("catalog.json");
        let config = ResourceManagerConfig {
            module_path: module,
            game_install_path: Some(game),
            ..ResourceManagerConfig::default()
        };
        let cancelled = AtomicBool::new(false);

        let first = ResourceManager::build_with_cache(&config, Some(&cache), &cancelled)
            .expect("cold build");
        let second = ResourceManager::build_with_cache(&config, Some(&cache), &cancelled)
            .expect("warm build");
        fs::write(&loose, b"second version").expect("change source");
        let third = ResourceManager::build_with_cache(&config, Some(&cache), &cancelled)
            .expect("invalidated build");

        assert_eq!(first.cache.state, ResourceCatalogCacheState::Miss);
        assert_eq!(second.cache.state, ResourceCatalogCacheState::Hit);
        assert_eq!(third.cache.state, ResourceCatalogCacheState::Miss);
        assert_eq!(second.cache.game_resource_count, 1);
    }
}
