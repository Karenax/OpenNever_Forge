use crate::jobs::JobRegistry;
use aurora_index::DatabaseInfo;
use std::sync::Arc;
use tracing_appender::non_blocking::WorkerGuard;

pub struct AppState {
    pub database: DatabaseInfo,
    pub jobs: Arc<JobRegistry>,
    _log_guard: WorkerGuard,
}

impl AppState {
    pub fn new(database: DatabaseInfo, log_guard: WorkerGuard) -> Self {
        Self {
            database,
            jobs: Arc::new(JobRegistry::default()),
            _log_guard: log_guard,
        }
    }
}
