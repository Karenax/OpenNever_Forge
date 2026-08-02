use aurora_core::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DATABASE_SCHEMA_VERSION: u32 = 1;

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
    let version: u32 = connection
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
            .pragma_update(None, "user_version", DATABASE_SCHEMA_VERSION)
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
    } else if version != DATABASE_SCHEMA_VERSION {
        return Err(AppError::database(
            path.display().to_string(),
            format!("unsupported schema version {version}, expected {DATABASE_SCHEMA_VERSION}"),
        )
        .into());
    }

    Ok(())
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
    }
}
