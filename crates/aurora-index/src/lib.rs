use aurora_core::{AppError, AppResult};
use aurora_dialogue::{DialogueIndex, DialogueNodeKind};
use aurora_nwscript::{ScriptIndex, ScriptSymbolKind};
use aurora_resource::{ResourceCatalog, ResourceSourceKind, ResourceVersion};
use aurora_world::WorldIndex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DATABASE_SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub schema_version: u32,
    pub path: String,
}

pub fn initialize_database(path: &Path) -> AppResult<DatabaseInfo> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::io(
                "create database directory",
                parent.display().to_string(),
                &error,
            )
        })?;
    }

    let mut connection = Connection::open(path).map_err(|error| {
        AppError::database(path.display().to_string(), format!("open failed: {error}"))
    })?;

    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot enable WAL mode: {error}"),
            )
        })?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot enable foreign keys: {error}"),
            )
        })?;

    migrate(&mut connection, path)?;

    Ok(DatabaseInfo {
        schema_version: DATABASE_SCHEMA_VERSION,
        path: path.display().to_string(),
    })
}

fn migrate(connection: &mut Connection, path: &Path) -> AppResult<()> {
    let mut version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot read schema version: {error}"),
            )
        })?;

    if version == 0 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0001 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 1_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration: {error}"),
            )
        })?;
        version = 1;
    }
    if version == 1 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration 0002: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0002_resource_catalog.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0002 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 2_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration 0002: {error}"),
            )
        })?;
        version = 2;
    }
    if version == 2 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration 0003: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0003_dependency_baselines.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0003 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 3_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration 0003: {error}"),
            )
        })?;
        version = 3;
    }
    if version == 3 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration 0004: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0004_script_index.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0004 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 4_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration 0004: {error}"),
            )
        })?;
        version = 4;
    }
    if version == 4 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration 0005: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0005_dialogue_index.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0005 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 5_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration 0005: {error}"),
            )
        })?;
        version = 5;
    }
    if version == 5 {
        let transaction = connection.transaction().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot begin migration 0006: {error}"),
            )
        })?;
        transaction
            .execute_batch(include_str!("../migrations/0006_world_report.sql"))
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("migration 0006 failed: {error}"),
                )
            })?;
        transaction
            .pragma_update(None, "user_version", 6_u32)
            .map_err(|error| {
                AppError::database(
                    path.display().to_string(),
                    format!("cannot update schema version: {error}"),
                )
            })?;
        transaction.commit().map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot commit migration 0006: {error}"),
            )
        })?;
        version = 6;
    }
    if version != DATABASE_SCHEMA_VERSION {
        return Err(AppError::database(
            path.display().to_string(),
            format!("unsupported schema version {version}, expected {DATABASE_SCHEMA_VERSION}"),
        )
        .into());
    }

    Ok(())
}

pub struct CatalogPersistence<'a> {
    pub project_id: &'a str,
    pub source_digest: &'a str,
    pub catalog: &'a ResourceCatalog,
    pub structured_summary_json: &'a str,
    pub source_path: &'a str,
    pub dependency_report_json: &'a str,
    pub script_index: &'a ScriptIndex,
    pub dialogue_index: &'a DialogueIndex,
    pub world_index: &'a WorldIndex,
}

pub fn replace_resource_catalog(path: &Path, write: CatalogPersistence<'_>) -> AppResult<()> {
    let CatalogPersistence {
        project_id,
        source_digest,
        catalog,
        structured_summary_json,
        source_path,
        dependency_report_json,
        script_index,
        dialogue_index,
        world_index,
    } = write;
    let mut connection = Connection::open(path).map_err(|error| {
        AppError::database(path.display().to_string(), format!("open failed: {error}"))
    })?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot enable foreign keys: {error}"),
            )
        })?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot start resource index transaction: {error}"),
        )
    })?;
    transaction
        .execute(
            "DELETE FROM resource_catalogs WHERE project_id = ?1",
            [project_id],
        )
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot replace resource catalog: {error}"),
            )
        })?;
    transaction.execute(
        "INSERT INTO resource_catalogs(project_id, source_digest, resource_count, version_count, shadowed_count) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![project_id, source_digest, catalog.entries.len() as i64, catalog.version_count as i64, catalog.shadowed_count as i64],
    ).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert resource catalog: {error}")))?;
    for entry in &catalog.entries {
        insert_version(&transaction, project_id, &entry.selected, true, path)?;
        for version in &entry.shadowed {
            insert_version(&transaction, project_id, version, false, path)?;
        }
    }
    transaction
        .execute(
            "INSERT INTO structured_summaries(project_id, summary_json) VALUES (?1, ?2)",
            rusqlite::params![project_id, structured_summary_json],
        )
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot insert structured summary: {error}"),
            )
        })?;
    transaction.execute(
        "INSERT INTO dependency_baselines(source_path, report_json, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP) ON CONFLICT(source_path) DO UPDATE SET report_json=excluded.report_json, updated_at=CURRENT_TIMESTAMP",
        rusqlite::params![source_path, dependency_report_json],
    ).map_err(|error| AppError::database(path.display().to_string(), format!("cannot persist dependency baseline: {error}")))?;
    for script in &script_index.documents {
        transaction.execute(
            "INSERT INTO scripts(project_id, resref, has_nss, has_ncs, source_text, source_path, bytecode_path, line_count, symbol_count, diagnostic_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![project_id, script.resref, script.nss.is_some(), script.ncs.is_some(), script.nss.as_ref().map(|value| value.text.as_str()), script.nss.as_ref().map(|value| value.source.as_str()), script.ncs.as_ref().map(|value| value.source.as_str()), script.nss.as_ref().map_or(0_i64, |value| value.line_count as i64), script.nss.as_ref().map_or(0_i64, |value| value.symbols.len() as i64), (script.diagnostics.len() + script.nss.as_ref().map_or(0, |value| value.diagnostics.len())) as i64],
        ).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert script: {error}")))?;
        if let Some(nss) = &script.nss {
            for symbol in &nss.symbols {
                let kind = match symbol.kind {
                    ScriptSymbolKind::Function => "function",
                    ScriptSymbolKind::Constant => "constant",
                };
                transaction.execute("INSERT INTO script_symbols(project_id, script_resref, name, kind, line, declaration) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![project_id, script.resref, symbol.name, kind, symbol.line as i64, symbol.declaration]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert script symbol: {error}")))?;
            }
            for include in &nss.includes {
                transaction.execute("INSERT INTO script_includes(project_id, script_resref, include_resref, line, resolved) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![project_id, script.resref, include.resref, include.line as i64, include.resolved]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert script include: {error}")))?;
            }
        }
        for reference in &script.inbound_references {
            transaction.execute("INSERT INTO script_references(project_id, script_resref, resource_resref, resource_type, field_path, source_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![project_id, script.resref, reference.resource.resref, i64::from(reference.resource.resource_type), reference.field_path, reference.source]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert script reference: {error}")))?;
        }
    }
    for dialogue in &dialogue_index.dialogues {
        transaction.execute("INSERT INTO dialogues(project_id, resref, source_path, node_count, link_count, cycle_count, diagnostic_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![project_id, dialogue.key.resref, dialogue.source, dialogue.nodes.len() as i64, dialogue.links.len() as i64, dialogue.cycles.len() as i64, dialogue.diagnostics.len() as i64]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert dialogue: {error}")))?;
        for node in &dialogue.nodes {
            let kind = match node.kind {
                DialogueNodeKind::Entry => "entry",
                DialogueNodeKind::Reply => "reply",
            };
            transaction.execute("INSERT INTO dialogue_nodes(project_id, dialogue_resref, node_id, kind, node_index, display_text, speaker, comment, action_script) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", rusqlite::params![project_id, dialogue.key.resref, node.id, kind, node.index as i64, node.display_text, node.speaker, node.comment, node.action_script]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert dialogue node: {error}")))?;
        }
        for link in &dialogue.links {
            transaction.execute("INSERT INTO dialogue_links(project_id, dialogue_resref, link_id, source_node, target_node, condition_script, action_script, is_child, broken) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", rusqlite::params![project_id, dialogue.key.resref, link.id, link.source, link.target, link.condition_script, link.action_script, link.is_child, link.broken]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert dialogue link: {error}")))?;
        }
        for reference in &dialogue.references {
            transaction.execute("INSERT INTO dialogue_references(project_id, dialogue_resref, resource_resref, resource_type, field_path, source_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![project_id, dialogue.key.resref, reference.resource.resref, i64::from(reference.resource.resource_type), reference.field_path, reference.source]).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert dialogue reference: {error}")))?;
        }
    }
    let report = world_index.report(source_digest);
    let summary_json = serde_json::to_string(&world_index.summary).map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot serialize world summary: {error}"),
        )
    })?;
    transaction.execute(
        "INSERT INTO world_reports(project_id, schema_version, summary_json, report_json, diagnostic_count) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![project_id, i64::from(report.schema_version), summary_json, report.stable_json(), report.diagnostics.len() as i64],
    ).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert world report: {error}")))?;
    transaction.commit().map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot commit resource index: {error}"),
        )
    })?;
    Ok(())
}

pub fn load_dependency_baseline(path: &Path, source_path: &str) -> AppResult<Option<String>> {
    let connection = Connection::open(path).map_err(|error| {
        AppError::database(path.display().to_string(), format!("open failed: {error}"))
    })?;
    let mut statement = connection
        .prepare("SELECT report_json FROM dependency_baselines WHERE source_path = ?1")
        .map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot prepare dependency baseline query: {error}"),
            )
        })?;
    let mut rows = statement.query([source_path]).map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot query dependency baseline: {error}"),
        )
    })?;
    match rows.next().map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot read dependency baseline: {error}"),
        )
    })? {
        Some(row) => row.get(0).map(Some).map_err(|error| {
            AppError::database(
                path.display().to_string(),
                format!("cannot decode dependency baseline: {error}"),
            )
            .into()
        }),
        None => Ok(None),
    }
}

fn insert_version(
    connection: &Connection,
    project_id: &str,
    version: &ResourceVersion,
    selected: bool,
    path: &Path,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO resource_versions(project_id, resref, resource_type, source_kind, source_name, source_path, priority, resource_offset, resource_size, sha256, is_selected) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![project_id, version.key.resref, i64::from(version.key.resource_type), source_kind(version.source_kind), version.source_name, version.source_path, i64::from(version.priority), version.offset as i64, version.size as i64, version.sha256, selected],
    ).map_err(|error| AppError::database(path.display().to_string(), format!("cannot insert resource version: {error}")))?;
    Ok(())
}

fn source_kind(kind: ResourceSourceKind) -> &'static str {
    match kind {
        ResourceSourceKind::Standalone => "standalone",
        ResourceSourceKind::Development => "development",
        ResourceSourceKind::Override => "override",
        ResourceSourceKind::Module => "module",
        ResourceSourceKind::Hak => "hak",
        ResourceSourceKind::Patch => "patch",
        ResourceSourceKind::KeyBif => "key_bif",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn initializes_and_reopens_the_database() {
        let root = tempdir().expect("temp directory");
        let path = root.path().join("cache/index.sqlite3");

        let first = initialize_database(&path).expect("first initialization");
        let second = initialize_database(&path).expect("second initialization");

        assert_eq!(first.schema_version, DATABASE_SCHEMA_VERSION);
        assert_eq!(first, second);

        let connection = Connection::open(path).expect("open database");
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get(0),
            )
            .expect("query schema");
        assert_eq!(count, 1);
        assert_eq!(first.schema_version, 6);
    }

    #[test]
    fn migrates_an_existing_version_one_database() {
        let root = tempdir().expect("temp directory");
        let path = root.path().join("index.sqlite3");
        let connection = Connection::open(&path).expect("database");
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .expect("v1 schema");
        connection
            .pragma_update(None, "user_version", 1_u32)
            .expect("v1 marker");
        drop(connection);
        let info = initialize_database(&path).expect("migration");
        assert_eq!(info.schema_version, 6);
        let connection = Connection::open(path).expect("reopen");
        let count: u32 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='resource_versions'", [], |row| row.get(0)).expect("resource table");
        assert_eq!(count, 1);
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='world_reports'",
                [],
                |row| row.get(0),
            )
            .expect("world report table");
        assert_eq!(count, 1);
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='scripts'",
                [],
                |row| row.get(0),
            )
            .expect("scripts table");
        assert_eq!(count, 1);
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dialogues'",
                [],
                |row| row.get(0),
            )
            .expect("dialogues table");
        assert_eq!(count, 1);
        let count: u32 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dependency_baselines'", [], |row| row.get(0)).expect("baseline table");
        assert_eq!(count, 1);
    }

    #[test]
    fn persists_resource_indexes_and_dependency_baselines_atomically() {
        let root = tempdir().expect("temp directory");
        let path = root.path().join("index.sqlite3");
        initialize_database(&path).expect("database");
        replace_resource_catalog(
            &path,
            CatalogPersistence {
                project_id: "project",
                source_digest: "digest",
                catalog: &ResourceCatalog::default(),
                structured_summary_json: "{}",
                source_path: "C:/fixture.mod",
                dependency_report_json: "{\"dependencies\":[]}",
                script_index: &ScriptIndex::default(),
                dialogue_index: &DialogueIndex::default(),
                world_index: &WorldIndex::default(),
            },
        )
        .expect("catalog");
        assert_eq!(
            load_dependency_baseline(&path, "C:/fixture.mod")
                .expect("baseline")
                .as_deref(),
            Some("{\"dependencies\":[]}")
        );
    }
}
