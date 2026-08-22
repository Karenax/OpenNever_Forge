use crate::commands::resolved_resource_bytes;
use crate::state::AppState;
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_dialogue::{DialogueGraph, DialogueSearchHit, adapt_dialogue};
use aurora_dialogue_export::{
    DialogueExportPreview, DialogueExportResult, DialogueExportRevision, DialogueExportSource,
    export_dialogue, preview_dialogue_export as preview_export,
    validate_dialogue_export_destination,
};
use aurora_gff::parse_gff;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportAnalysisRequest {
    pub analysis_job_id: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportPreviewRequest {
    pub analysis_job_id: String,
    pub workspace_id: Option<String>,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportRequest {
    pub analysis_job_id: String,
    pub workspace_id: Option<String>,
    pub resref: String,
    pub destination: String,
    pub expected_source_resource_sha256: String,
    pub local_only_acknowledged: bool,
}

#[tauri::command]
pub fn list_dialogue_export_candidates(
    state: State<'_, AppState>,
    request: DialogueExportAnalysisRequest,
) -> AppResult<Vec<DialogueSearchHit>> {
    let mut candidates = state
        .jobs
        .with_analysis(&request.analysis_job_id, |analysis| {
            Ok(analysis
                .dialogue_index
                .dialogues
                .iter()
                .map(|dialogue| DialogueSearchHit {
                    resref: dialogue.key.resref.clone(),
                    node_count: dialogue.nodes.len(),
                    link_count: dialogue.links.len(),
                    cycle_count: dialogue.cycles.len(),
                    diagnostic_count: dialogue.diagnostics.len(),
                    preview: dialogue
                        .nodes
                        .iter()
                        .find_map(|node| node.display_text.clone()),
                })
                .collect::<Vec<_>>())
        })?;
    if let Some(workspace_id) = request.workspace_id.as_deref() {
        let workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces.get(workspace_id).ok_or_else(|| {
            Box::new(
                AppError::new(
                    "EDIT_WORKSPACE_NOT_OPEN",
                    "L’espace d’édition n’est pas ouvert.",
                    format!("No open edit workspace has id {workspace_id}"),
                    ErrorSeverity::Warning,
                )
                .with_import_stage("dialogue_export_workspace"),
            )
        })?;
        for modified in workspace
            .modified_resources()
            .into_iter()
            .filter(|modified| modified.resource.resource_type == 2029)
        {
            let Some(bytes) = workspace.staged_resource_bytes(&modified.resource)? else {
                continue;
            };
            let raw = parse_gff(
                &bytes,
                &format!("workspace::{}", modified.resource.file_name()),
            )?;
            let graph = adapt_dialogue(
                modified.resource.clone(),
                format!("workspace::{}", modified.resource.file_name()),
                raw,
            );
            let candidate = DialogueSearchHit {
                resref: graph.key.resref,
                node_count: graph.nodes.len(),
                link_count: graph.links.len(),
                cycle_count: graph.cycles.len(),
                diagnostic_count: graph.diagnostics.len(),
                preview: graph
                    .nodes
                    .iter()
                    .find_map(|node| node.display_text.clone()),
            };
            if let Some(existing) = candidates
                .iter_mut()
                .find(|existing| existing.resref == candidate.resref)
            {
                *existing = candidate;
            } else {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_by(|left, right| left.resref.cmp(&right.resref));
    Ok(candidates)
}

#[tauri::command]
pub fn preview_dialogue_export(
    state: State<'_, AppState>,
    request: DialogueExportPreviewRequest,
) -> AppResult<DialogueExportPreview> {
    let source = build_dialogue_export_source(
        &state,
        &request.analysis_job_id,
        request.workspace_id.as_deref(),
        &request.resref,
    )?;
    preview_export(&source)
}

#[tauri::command]
pub async fn export_dialogue_bundle(
    state: State<'_, AppState>,
    request: DialogueExportRequest,
) -> AppResult<DialogueExportResult> {
    if !request.local_only_acknowledged {
        return Err(Box::new(
            AppError::new(
                "DIALOGUE_EXPORT_LOCAL_ONLY_REQUIRED",
                "Confirmez que cet export reste local avant de continuer.",
                "dialogue export requires an explicit local-only acknowledgement",
                ErrorSeverity::Warning,
            )
            .with_resource(request.resref)
            .with_import_stage("dialogue_export_authorization"),
        ));
    }
    let source = build_dialogue_export_source(
        &state,
        &request.analysis_job_id,
        request.workspace_id.as_deref(),
        &request.resref,
    )?;
    let destination = PathBuf::from(request.destination);
    validate_dialogue_export_destination(&destination, &source.protected_roots)?;
    let actual_sha256 = preview_export(&source)?.source_resource_sha256;
    if actual_sha256 != request.expected_source_resource_sha256 {
        return Err(Box::new(
            AppError::new(
                "DIALOGUE_EXPORT_SOURCE_CHANGED",
                "Le dialogue a changé depuis l’aperçu. Rechargez-le avant d’exporter.",
                format!(
                    "dialogue preview sha256 {} differs from current {}",
                    request.expected_source_resource_sha256, actual_sha256
                ),
                ErrorSeverity::Warning,
            )
            .with_resource(request.resref)
            .with_import_stage("dialogue_export_source_check"),
        ));
    }
    tauri::async_runtime::spawn_blocking(move || export_dialogue(&source, &destination))
        .await
        .map_err(|error| {
            Box::new(
                AppError::new(
                    "DIALOGUE_EXPORT_TASK_FAILED",
                    "La tâche d'export du dialogue s'est interrompue.",
                    error.to_string(),
                    ErrorSeverity::Error,
                )
                .with_import_stage("dialogue_export_task"),
            )
        })?
}

fn build_dialogue_export_source(
    state: &AppState,
    analysis_job_id: &str,
    workspace_id: Option<&str>,
    resref: &str,
) -> AppResult<DialogueExportSource> {
    let resource = ResourceKey::new(resref, 2029);
    let indexed = state.jobs.with_analysis(analysis_job_id, |analysis| {
        Ok(analysis.dialogue_index.get(resref).cloned())
    })?;
    let staged = if let Some(workspace_id) = workspace_id {
        let workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces.get(workspace_id).ok_or_else(|| {
            Box::new(
                AppError::new(
                    "EDIT_WORKSPACE_NOT_OPEN",
                    "L’espace d’édition n’est pas ouvert.",
                    format!("No open edit workspace has id {workspace_id}"),
                    ErrorSeverity::Warning,
                )
                .with_import_stage("dialogue_export_workspace"),
            )
        })?;
        workspace.staged_resource_bytes(&resource)?
    } else {
        None
    };

    let (graph, resource_bytes, revision) = if let Some(bytes) = staged {
        let raw = parse_gff(&bytes, &format!("workspace::{}", resource.file_name()))?;
        let mut graph = adapt_dialogue(
            resource.clone(),
            format!("workspace::{}", resource.file_name()),
            raw,
        );
        if let Some(indexed) = &indexed {
            merge_indexed_display_text(&mut graph, indexed);
            graph.references = indexed.references.clone();
        }
        (graph, bytes, DialogueExportRevision::Workspace)
    } else {
        let graph = indexed.ok_or_else(|| {
            AppError::invalid_path(resref, "dialogue not found in the indexed catalog")
        })?;
        let source_bytes = resolved_resource_bytes(state, analysis_job_id, &resource)?;
        (graph, source_bytes, DialogueExportRevision::Analysis)
    };
    Ok(DialogueExportSource {
        graph,
        resource_bytes,
        revision,
        protected_roots: state
            .jobs
            .dialogue_export_protected_roots(analysis_job_id)?,
    })
}

fn merge_indexed_display_text(graph: &mut DialogueGraph, indexed: &DialogueGraph) {
    for node in &mut graph.nodes {
        let Some(indexed_node) = indexed
            .nodes
            .iter()
            .find(|candidate| candidate.id == node.id)
        else {
            continue;
        };
        if node.display_text.is_none() && node.text == indexed_node.text {
            node.display_text = indexed_node.display_text.clone();
        }
    }
}
