use crate::jobs::JobSnapshot;
use crate::state::AppState;
use aurora_core::AppResult;
use aurora_migration::{
    AreaMigrationCandidate, AreaMigrationExportRequest, AreaMigrationPreview, audit_area_migration,
    export_area_migration, list_area_migration_candidates as list_candidates,
    validate_bundle_destination_with_sources,
};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

const JOB_PROGRESS_EVENT: &str = "job-progress";
const MAX_INLINE_MIGRATION_DIAGNOSTICS: usize = 500;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationAnalysisRequest {
    pub analysis_job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPreviewRequest {
    pub analysis_job_id: String,
    pub area_resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationExportRequest {
    pub analysis_job_id: String,
    pub area_resref: String,
    pub destination: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationJobRequest {
    pub analysis_job_id: String,
    pub area_resref: String,
}

#[tauri::command]
pub fn list_area_migration_candidates(
    state: State<'_, AppState>,
    request: MigrationAnalysisRequest,
) -> AppResult<Vec<AreaMigrationCandidate>> {
    let source = state.jobs.migration_source(&request.analysis_job_id)?;
    Ok(list_candidates(&source))
}

#[tauri::command]
pub fn preview_area_migration(
    state: State<'_, AppState>,
    request: MigrationPreviewRequest,
) -> AppResult<AreaMigrationPreview> {
    let source = state.jobs.migration_source(&request.analysis_job_id)?;
    let mut preview = audit_area_migration(
        &source,
        &request.area_resref,
        &std::sync::atomic::AtomicBool::new(false),
    )?;
    preview
        .diagnostics
        .truncate(MAX_INLINE_MIGRATION_DIAGNOSTICS);
    Ok(preview)
}

#[tauri::command]
pub fn get_area_migration_job(
    state: State<'_, AppState>,
    request: MigrationJobRequest,
) -> Option<JobSnapshot> {
    state
        .jobs
        .find_area_migration_job(&request.analysis_job_id, &request.area_resref)
}

#[tauri::command]
pub fn start_area_migration_export(
    app: AppHandle,
    state: State<'_, AppState>,
    request: MigrationExportRequest,
) -> AppResult<JobSnapshot> {
    let destination = PathBuf::from(&request.destination);
    let source = state.jobs.migration_source(&request.analysis_job_id)?;
    let registry = state.jobs.clone();
    validate_bundle_destination_with_sources(&destination, &source.protected_roots)?;
    let (job, cancellation) = registry.create_area_migration_job(
        &request.analysis_job_id,
        request.area_resref.clone(),
        destination.clone(),
    )?;
    let job_id = job.id.clone();
    let app_handle = app.clone();
    let source = registry.migration_source(&job_id)?;
    let export_request = AreaMigrationExportRequest {
        area_resref: request.area_resref,
        destination,
    };

    tauri::async_runtime::spawn_blocking(move || {
        if let Some(snapshot) = registry.set_running(&job_id) {
            emit_snapshot(&app_handle, &snapshot);
        }
        let result = export_area_migration(&source, &export_request, &cancellation, |progress| {
            if let Some(snapshot) = registry.set_migration_progress(&job_id, progress) {
                emit_snapshot(&app_handle, &snapshot);
            }
        });
        let snapshot = match result {
            Ok(mut result) => {
                result
                    .diagnostics
                    .truncate(MAX_INLINE_MIGRATION_DIAGNOSTICS);
                registry.complete_migration(&job_id, result)
            }
            Err(error) => registry.fail(&job_id, *error),
        };
        if let Some(snapshot) = snapshot {
            emit_snapshot(&app_handle, &snapshot);
        }
    });

    Ok(job)
}

fn emit_snapshot(app: &AppHandle, snapshot: &JobSnapshot) {
    if let Err(error) = app.emit(JOB_PROGRESS_EVENT, snapshot) {
        tracing::warn!(job_id = snapshot.id, %error, "cannot emit migration job progress");
    }
}
