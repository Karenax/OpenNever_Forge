use crate::jobs::JobSnapshot;
use crate::state::AppState;
use aurora_core::{AppError, AppResult};
use aurora_project::analyze_module_file;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

const JOB_PROGRESS_EVENT: &str = "job-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_version: &'static str,
    pub read_only: bool,
    pub database_schema_version: u32,
}

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppStatus {
    AppStatus {
        app_version: env!("CARGO_PKG_VERSION"),
        read_only: true,
        database_schema_version: state.database.schema_version,
    }
}

#[tauri::command]
pub fn start_module_analysis(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<JobSnapshot> {
    let module_path = PathBuf::from(&path);
    if !module_path.is_file() {
        return Err(AppError::module_not_found(path).into());
    }
    if !module_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("mod"))
    {
        return Err(AppError::invalid_path(
            module_path.display().to_string(),
            "Hash jobs accept only .mod files",
        )
        .into());
    }

    let registry = state.jobs.clone();
    let (job, cancellation) = registry.create_analysis_job(module_path.display().to_string());
    let job_id = job.id.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        if let Some(snapshot) = registry.set_running(&job_id) {
            emit_snapshot(&app_handle, &snapshot);
        }

        let result = analyze_module_file(&module_path, &cancellation, |progress| {
            if let Some(snapshot) = registry.set_progress(&job_id, progress) {
                emit_snapshot(&app_handle, &snapshot);
            }
        });

        let snapshot = match result {
            Ok(fingerprint) => registry.complete(&job_id, fingerprint),
            Err(error) => registry.fail(&job_id, *error),
        };
        if let Some(snapshot) = snapshot {
            emit_snapshot(&app_handle, &snapshot);
        }
    });

    Ok(job)
}

#[tauri::command]
pub fn get_job(state: State<'_, AppState>, id: String) -> Option<JobSnapshot> {
    state.jobs.get(&id)
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: String) -> AppResult<JobSnapshot> {
    state.jobs.cancel(&id)
}

fn emit_snapshot(app: &AppHandle, snapshot: &JobSnapshot) {
    if let Err(error) = app.emit(JOB_PROGRESS_EVENT, snapshot) {
        tracing::warn!(job_id = snapshot.id, %error, "cannot emit job progress");
    }
}
