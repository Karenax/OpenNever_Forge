use crate::state::AppState;
use aurora_asset_export::{
    AssetExportCandidate, AssetExportPreview, AssetExportResult, export_asset,
    list_asset_export_candidates as list_candidates, preview_asset_export as preview_export,
    validate_asset_export_destination,
};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportAnalysisRequest {
    pub analysis_job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportPreviewRequest {
    pub analysis_job_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportRequest {
    pub analysis_job_id: String,
    pub resref: String,
    pub destination: String,
    pub local_only_acknowledged: bool,
}

#[tauri::command]
pub fn list_asset_export_candidates(
    state: State<'_, AppState>,
    request: AssetExportAnalysisRequest,
) -> AppResult<Vec<AssetExportCandidate>> {
    let source = state.jobs.asset_export_source(&request.analysis_job_id)?;
    Ok(list_candidates(&source))
}

#[tauri::command]
pub fn preview_asset_export(
    state: State<'_, AppState>,
    request: AssetExportPreviewRequest,
) -> AppResult<AssetExportPreview> {
    let source = state.jobs.asset_export_source(&request.analysis_job_id)?;
    preview_export(&source, &request.resref, &AtomicBool::new(false))
}

#[tauri::command]
pub async fn export_asset_bundle(
    state: State<'_, AppState>,
    request: AssetExportRequest,
) -> AppResult<AssetExportResult> {
    if !request.local_only_acknowledged {
        return Err(Box::new(
            AppError::new(
                "ASSET_EXPORT_LOCAL_ONLY_REQUIRED",
                "Confirmez que cet export reste local avant de continuer.",
                "asset export requires an explicit local-only acknowledgement",
                ErrorSeverity::Warning,
            )
            .with_resource(request.resref)
            .with_import_stage("asset_export_authorization"),
        ));
    }
    let source = state.jobs.asset_export_source(&request.analysis_job_id)?;
    let destination = PathBuf::from(request.destination);
    validate_asset_export_destination(&destination, &source.protected_roots)?;
    let resref = request.resref;
    tauri::async_runtime::spawn_blocking(move || {
        export_asset(&source, &resref, &destination, &AtomicBool::new(false))
    })
    .await
    .map_err(|error| {
        Box::new(
            AppError::new(
                "ASSET_EXPORT_TASK_FAILED",
                "La tâche d'export de l'asset s'est interrompue.",
                error.to_string(),
                ErrorSeverity::Error,
            )
            .with_import_stage("asset_export_task"),
        )
    })?
}
