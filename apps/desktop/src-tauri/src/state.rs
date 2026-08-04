use crate::jobs::JobRegistry;
use aurora_edit::EditWorkspace;
use aurora_index::DatabaseInfo;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing_appender::non_blocking::WorkerGuard;

pub struct AppState {
    pub database: DatabaseInfo,
    pub jobs: Arc<JobRegistry>,
    pub asset_cache_root: PathBuf,
    pub edit_workspace_root: PathBuf,
    pub edit_workspaces: Mutex<HashMap<String, EditWorkspace>>,
    _log_guard: WorkerGuard,
}

impl AppState {
    pub fn new(database: DatabaseInfo, asset_cache_root: PathBuf, log_guard: WorkerGuard) -> Self {
        let edit_workspace_root = database_path_parent(&database).join("workspaces");
        Self {
            database,
            jobs: Arc::new(JobRegistry::default()),
            asset_cache_root,
            edit_workspace_root,
            edit_workspaces: Mutex::new(HashMap::new()),
            _log_guard: log_guard,
        }
    }
}

fn database_path_parent(database: &DatabaseInfo) -> PathBuf {
    PathBuf::from(&database.path)
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
