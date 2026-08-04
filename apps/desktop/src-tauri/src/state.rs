use crate::jobs::JobRegistry;
use aurora_index::DatabaseInfo;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_appender::non_blocking::WorkerGuard;

pub struct AppState {
    pub database: DatabaseInfo,
    pub jobs: Arc<JobRegistry>,
    pub asset_cache_root: PathBuf,
    _log_guard: WorkerGuard,
}

impl AppState {
    pub fn new(database: DatabaseInfo, asset_cache_root: PathBuf, log_guard: WorkerGuard) -> Self {
        Self {
            database,
            jobs: Arc::new(JobRegistry::default()),
            asset_cache_root,
            _log_guard: log_guard,
        }
    }
}
