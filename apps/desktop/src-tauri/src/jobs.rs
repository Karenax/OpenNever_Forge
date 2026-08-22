use aurora_asset_export::AssetExportSource;
use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_migration::{AreaMigrationExportResult, AreaMigrationSource, MigrationProgress};
use aurora_project::{
    AnalysisPhase, HashProgress, ModuleAnalysis, ModuleDependencyReport, compare_dependency_reports,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub kind: String,
    pub state: JobState,
    pub source_path: String,
    pub progress: HashProgress,
    pub result: Option<ModuleAnalysis>,
    #[serde(default)]
    pub migration_progress: Option<MigrationProgress>,
    #[serde(default)]
    pub migration_result: Option<AreaMigrationExportResult>,
    #[serde(default)]
    pub migration_analysis_job_id: Option<String>,
    #[serde(default)]
    pub migration_area_resref: Option<String>,
    #[serde(default)]
    pub migration_destination: Option<String>,
    pub error: Option<AppError>,
}

struct JobEntry {
    cancellation: Arc<AtomicBool>,
    snapshot: JobSnapshot,
    protected_roots: Vec<PathBuf>,
    asset_export_source: Option<AssetExportSource>,
    migration_source: Option<AreaMigrationSource>,
    created_order: u64,
}

#[derive(Default)]
pub struct JobRegistry {
    entries: Mutex<HashMap<String, JobEntry>>,
    dependency_baselines: Mutex<HashMap<String, ModuleDependencyReport>>,
    next_order: Mutex<u64>,
}

impl JobRegistry {
    #[allow(dead_code)]
    pub fn create_analysis_job(&self, source_path: String) -> (JobSnapshot, Arc<AtomicBool>) {
        self.create_analysis_job_with_roots(source_path, Vec::new())
    }

    pub fn create_analysis_job_with_roots(
        &self,
        source_path: String,
        protected_roots: Vec<PathBuf>,
    ) -> (JobSnapshot, Arc<AtomicBool>) {
        let id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        let snapshot = JobSnapshot {
            id: id.clone(),
            kind: "module_analysis".to_owned(),
            state: JobState::Queued,
            source_path,
            progress: HashProgress {
                bytes_read: 0,
                total_bytes: 0,
                percent: 0.0,
                phase: AnalysisPhase::Hashing,
            },
            result: None,
            migration_progress: None,
            migration_result: None,
            migration_analysis_job_id: None,
            migration_area_resref: None,
            migration_destination: None,
            error: None,
        };

        self.entries.lock().expect("job registry poisoned").insert(
            id,
            JobEntry {
                cancellation: Arc::clone(&cancellation),
                snapshot: snapshot.clone(),
                protected_roots,
                asset_export_source: None,
                migration_source: None,
                created_order: self.allocate_order(),
            },
        );
        (snapshot, cancellation)
    }

    #[allow(dead_code)]
    pub fn create_migration_job(&self, source_path: String) -> (JobSnapshot, Arc<AtomicBool>) {
        let id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        let snapshot = JobSnapshot {
            id: id.clone(),
            kind: "area_migration_export".to_owned(),
            state: JobState::Queued,
            source_path,
            progress: HashProgress::stage(AnalysisPhase::Persisting, 0.0),
            result: None,
            migration_progress: None,
            migration_result: None,
            migration_analysis_job_id: None,
            migration_area_resref: None,
            migration_destination: None,
            error: None,
        };
        self.entries.lock().expect("job registry poisoned").insert(
            id,
            JobEntry {
                cancellation: Arc::clone(&cancellation),
                snapshot: snapshot.clone(),
                protected_roots: Vec::new(),
                asset_export_source: None,
                migration_source: None,
                created_order: self.allocate_order(),
            },
        );
        (snapshot, cancellation)
    }

    pub fn create_area_migration_job(
        &self,
        analysis_job_id: &str,
        area_resref: String,
        destination: PathBuf,
    ) -> AppResult<(JobSnapshot, Arc<AtomicBool>)> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        if entries.values().any(|entry| {
            entry.snapshot.kind == "area_migration_export"
                && entry.snapshot.migration_analysis_job_id.as_deref() == Some(analysis_job_id)
                && entry.snapshot.migration_area_resref.as_deref() == Some(area_resref.as_str())
                && matches!(
                    entry.snapshot.state,
                    JobState::Queued | JobState::Running | JobState::Cancelling
                )
        }) {
            return Err(Box::new(
                AppError::new(
                    "MIGRATION_JOB_ALREADY_ACTIVE",
                    "Une exportation de cette zone est déjà active.",
                    "an area migration export is already active for this analysis and area",
                    ErrorSeverity::Warning,
                )
                .with_resource(area_resref)
                .with_import_stage("area_migration_job"),
            ));
        }
        let analysis_entry = entries
            .get_mut(analysis_job_id)
            .ok_or_else(|| job_not_found(analysis_job_id))?;
        let analysis = analysis_entry.snapshot.result.as_ref().ok_or_else(|| {
            AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L'analyse du module n'est pas terminée.",
                format!("Job {analysis_job_id} has no completed analysis"),
                ErrorSeverity::Warning,
            )
        })?;
        let source = analysis_entry
            .migration_source
            .get_or_insert_with(|| {
                AreaMigrationSource::from_analysis_with_roots(
                    analysis,
                    &analysis_entry.snapshot.source_path,
                    analysis_entry.protected_roots.clone(),
                )
            })
            .clone();
        let id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        let snapshot = JobSnapshot {
            id: id.clone(),
            kind: "area_migration_export".to_owned(),
            state: JobState::Queued,
            source_path: source.module_path.display().to_string(),
            progress: HashProgress::stage(AnalysisPhase::Persisting, 0.0),
            result: None,
            migration_progress: None,
            migration_result: None,
            migration_analysis_job_id: Some(analysis_job_id.to_owned()),
            migration_area_resref: Some(area_resref),
            migration_destination: Some(destination.display().to_string()),
            error: None,
        };
        entries.insert(
            id,
            JobEntry {
                cancellation: Arc::clone(&cancellation),
                snapshot: snapshot.clone(),
                protected_roots: source.protected_roots.clone(),
                asset_export_source: None,
                migration_source: Some(source),
                created_order: self.allocate_order(),
            },
        );
        Ok((snapshot, cancellation))
    }

    pub fn find_area_migration_job(
        &self,
        analysis_job_id: &str,
        area_resref: &str,
    ) -> Option<JobSnapshot> {
        self.entries
            .lock()
            .expect("job registry poisoned")
            .values()
            .filter(|entry| {
                entry.snapshot.kind == "area_migration_export"
                    && entry.snapshot.migration_analysis_job_id.as_deref() == Some(analysis_job_id)
                    && entry.snapshot.migration_area_resref.as_deref() == Some(area_resref)
            })
            .max_by_key(|entry| entry.created_order)
            .map(|entry| entry.snapshot.transport_clone())
    }

    pub fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.entries
            .lock()
            .expect("job registry poisoned")
            .get(id)
            .map(|entry| entry.snapshot.transport_clone())
    }

    pub fn restore_completed_analysis(
        &self,
        source_path: String,
        analysis: ModuleAnalysis,
    ) -> JobSnapshot {
        let (job, _) = self.create_analysis_job(source_path);
        self.set_running(&job.id);
        self.complete(&job.id, analysis)
            .expect("restored analysis job exists")
    }

    pub fn with_analysis<T>(
        &self,
        id: &str,
        read: impl FnOnce(&ModuleAnalysis) -> AppResult<T>,
    ) -> AppResult<T> {
        let entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get(id).ok_or_else(|| job_not_found(id))?;
        let analysis = entry.snapshot.result.as_ref().ok_or_else(|| {
            AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L'analyse du module n'est pas terminée.",
                format!("Job {id} has no completed analysis"),
                ErrorSeverity::Warning,
            )
        })?;
        read(analysis)
    }

    pub fn migration_source(&self, id: &str) -> AppResult<AreaMigrationSource> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get_mut(id).ok_or_else(|| job_not_found(id))?;
        if let Some(source) = &entry.migration_source {
            return Ok(source.clone());
        }
        let analysis = entry.snapshot.result.as_ref().ok_or_else(|| {
            AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L'analyse du module n'est pas terminée.",
                format!("Job {id} has no completed analysis"),
                ErrorSeverity::Warning,
            )
        })?;
        let source = AreaMigrationSource::from_analysis_with_roots(
            analysis,
            &entry.snapshot.source_path,
            entry.protected_roots.clone(),
        );
        entry.migration_source = Some(source.clone());
        Ok(source)
    }

    pub fn asset_export_source(&self, id: &str) -> AppResult<AssetExportSource> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get_mut(id).ok_or_else(|| job_not_found(id))?;
        if let Some(source) = &entry.asset_export_source {
            return Ok(source.clone());
        }
        let analysis = entry.snapshot.result.as_ref().ok_or_else(|| {
            AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L'analyse du module n'est pas terminée.",
                format!("Job {id} has no completed analysis"),
                ErrorSeverity::Warning,
            )
        })?;
        let source = AssetExportSource::from_analysis_with_roots(
            analysis,
            &entry.snapshot.source_path,
            entry.protected_roots.clone(),
        );
        entry.asset_export_source = Some(source.clone());
        Ok(source)
    }

    pub fn dialogue_export_protected_roots(&self, id: &str) -> AppResult<Vec<PathBuf>> {
        let entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get(id).ok_or_else(|| job_not_found(id))?;
        if entry.snapshot.result.is_none() {
            return Err(AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L'analyse du module n'est pas terminée.",
                format!("Job {id} has no completed analysis"),
                ErrorSeverity::Warning,
            )
            .into());
        }
        let mut roots = entry.protected_roots.clone();
        if let Some(parent) = Path::new(&entry.snapshot.source_path).parent() {
            roots.push(parent.to_path_buf());
        }
        roots.sort();
        roots.dedup();
        Ok(roots)
    }

    pub fn set_running(&self, id: &str) -> Option<JobSnapshot> {
        self.update(id, |snapshot| snapshot.state = JobState::Running)
    }

    pub fn set_progress(&self, id: &str, progress: HashProgress) -> Option<JobSnapshot> {
        self.update(id, |snapshot| {
            let mut progress = progress;
            if !matches!(
                snapshot.state,
                JobState::Completed | JobState::Failed | JobState::Cancelled
            ) {
                // 100 % is a terminal promise. The ERF reader can finish before the
                // resource catalog and dependency baseline have been persisted.
                progress.percent = progress.percent.clamp(0.0, 99.0);
            }
            snapshot.progress = progress;
        })
    }

    pub fn set_migration_progress(
        &self,
        id: &str,
        progress: MigrationProgress,
    ) -> Option<JobSnapshot> {
        self.update(id, |snapshot| {
            let mut progress = progress;
            if !matches!(
                snapshot.state,
                JobState::Completed | JobState::Failed | JobState::Cancelled
            ) {
                progress.percent = progress.percent.clamp(0.0, 99.0);
            }
            snapshot.progress.percent = progress.percent;
            snapshot.migration_progress = Some(progress);
        })
    }

    pub fn complete(&self, id: &str, mut result: ModuleAnalysis) -> Option<JobSnapshot> {
        let source_path = self
            .entries
            .lock()
            .expect("job registry poisoned")
            .get(id)?
            .snapshot
            .source_path
            .clone();
        let mut baselines = self
            .dependency_baselines
            .lock()
            .expect("dependency baseline registry poisoned");
        if let Some(previous) = baselines.get(&source_path) {
            compare_dependency_reports(&mut result.dependency_report, Some(previous));
        }
        baselines.insert(source_path, result.dependency_report.clone());
        drop(baselines);

        self.update(id, |snapshot| {
            snapshot.state = JobState::Completed;
            snapshot.progress.percent = 100.0;
            snapshot.result = Some(result);
        })
    }

    pub fn complete_migration(
        &self,
        id: &str,
        result: AreaMigrationExportResult,
    ) -> Option<JobSnapshot> {
        self.update(id, |snapshot| {
            snapshot.state = JobState::Completed;
            snapshot.progress.percent = 100.0;
            if let Some(progress) = &mut snapshot.migration_progress {
                progress.percent = 100.0;
            }
            snapshot.migration_result = Some(result);
        })
    }

    pub fn fail(&self, id: &str, error: AppError) -> Option<JobSnapshot> {
        self.update(id, |snapshot| {
            snapshot.state = if error.code == "JOB_CANCELLED" {
                JobState::Cancelled
            } else {
                JobState::Failed
            };
            snapshot.error = Some(error);
        })
    }

    pub fn cancel(&self, id: &str) -> AppResult<JobSnapshot> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get_mut(id).ok_or_else(|| job_not_found(id))?;

        if matches!(entry.snapshot.state, JobState::Queued | JobState::Running) {
            entry.cancellation.store(true, Ordering::Relaxed);
            entry.snapshot.state = JobState::Cancelling;
        }

        Ok(entry.snapshot.transport_clone())
    }

    fn update(&self, id: &str, update: impl FnOnce(&mut JobSnapshot)) -> Option<JobSnapshot> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get_mut(id)?;
        update(&mut entry.snapshot);
        Some(entry.snapshot.transport_clone())
    }

    fn allocate_order(&self) -> u64 {
        let mut order = self.next_order.lock().expect("job order registry poisoned");
        *order = order.saturating_add(1);
        *order
    }
}

impl JobSnapshot {
    fn transport_clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            kind: self.kind.clone(),
            state: self.state,
            source_path: self.source_path.clone(),
            progress: self.progress,
            result: self.result.as_ref().map(|analysis| ModuleAnalysis {
                fingerprint: analysis.fingerprint.clone(),
                inventory: analysis.inventory.clone(),
                module_info: analysis.module_info.clone(),
                dependency_report: analysis.dependency_report.clone(),
                resource_catalog: Default::default(),
                resource_catalog_summary: analysis.resource_catalog_summary.clone(),
                resource_catalog_cache: analysis.resource_catalog_cache.clone(),
                structured_summary: analysis.structured_summary.clone(),
                script_index: Default::default(),
                script_index_summary: analysis.script_index_summary.clone(),
                dialogue_index: Default::default(),
                dialogue_index_summary: analysis.dialogue_index_summary.clone(),
                world_index: Default::default(),
                world_summary: analysis.world_summary.clone(),
            }),
            migration_progress: self.migration_progress.clone(),
            migration_result: self.migration_result.clone(),
            migration_analysis_job_id: self.migration_analysis_job_id.clone(),
            migration_area_resref: self.migration_area_resref.clone(),
            migration_destination: self.migration_destination.clone(),
            error: self.error.clone(),
        }
    }
}

fn job_not_found(id: &str) -> AppError {
    AppError::new(
        "JOB_NOT_FOUND",
        "L'opération demandée n'existe plus.",
        format!("No job exists with id {id}"),
        ErrorSeverity::Warning,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cancelling_a_running_job_sets_the_flag_and_state() {
        let registry = JobRegistry::default();
        let (job, flag) = registry.create_analysis_job("fixture.mod".to_owned());
        registry.set_running(&job.id);

        let cancelled = registry.cancel(&job.id).expect("job exists");

        assert_eq!(cancelled.state, JobState::Cancelling);
        assert!(flag.load(Ordering::Relaxed));
    }

    #[test]
    fn migration_jobs_keep_dedicated_progress_and_share_safe_cancellation() {
        let registry = JobRegistry::default();
        let (job, flag) = registry.create_migration_job("synthetic.mod".to_owned());
        registry.set_running(&job.id);
        let running = registry
            .set_migration_progress(
                &job.id,
                MigrationProgress {
                    phase: aurora_migration::MigrationPhase::Models,
                    percent: 52.0,
                    current: Some("tile_a".to_owned()),
                },
            )
            .expect("migration job");
        assert_eq!(running.kind, "area_migration_export");
        assert_eq!(running.progress.percent, 52.0);
        assert_eq!(
            running
                .migration_progress
                .as_ref()
                .map(|progress| progress.current.as_deref()),
            Some(Some("tile_a"))
        );

        let cancelling = registry.cancel(&job.id).expect("cancellable migration");
        assert_eq!(cancelling.state, JobState::Cancelling);
        assert!(flag.load(Ordering::Relaxed));
    }

    #[test]
    fn reserves_one_hundred_percent_for_a_completed_job() {
        let registry = JobRegistry::default();
        let (job, _) = registry.create_analysis_job("fixture.mod".to_owned());
        registry.set_running(&job.id);

        let running = registry
            .set_progress(
                &job.id,
                HashProgress {
                    bytes_read: 512,
                    total_bytes: 512,
                    percent: 100.0,
                    phase: AnalysisPhase::ResourceCatalog,
                },
            )
            .expect("job exists");

        assert_eq!(running.state, JobState::Running);
        assert_eq!(running.progress.percent, 99.0);

        let completed = registry
            .complete(&job.id, analysis_with_dependency_hash("AAAA"))
            .expect("job exists");
        assert_eq!(completed.state, JobState::Completed);
        assert_eq!(completed.progress.percent, 100.0);
    }

    #[test]
    fn compares_dependencies_with_the_last_successful_analysis() {
        let registry = JobRegistry::default();
        let (first, _) = registry.create_analysis_job("fixture.mod".to_owned());
        registry.complete(&first.id, analysis_with_dependency_hash("AAAA"));
        let (second, _) = registry.create_analysis_job("fixture.mod".to_owned());

        let completed = registry
            .complete(&second.id, analysis_with_dependency_hash("BBBB"))
            .expect("second job exists");
        let report = &completed.result.expect("analysis result").dependency_report;

        assert_eq!(report.changed_count, 1);
        assert_eq!(
            report.dependencies[0].change,
            aurora_project::ModuleDependencyChange::ContentChanged
        );
    }

    #[test]
    fn area_migration_jobs_are_recoverable_and_duplicate_active_exports_are_rejected() {
        let registry = JobRegistry::default();
        let (analysis, _) = registry.create_analysis_job("fixture.mod".to_owned());
        registry.complete(&analysis.id, analysis_with_dependency_hash("AAAA"));
        let (job, _) = registry
            .create_area_migration_job(
                &analysis.id,
                "forest01".to_owned(),
                PathBuf::from("C:/exports/forest01.area-migration-v1"),
            )
            .expect("migration job");
        let recovered = registry
            .find_area_migration_job(&analysis.id, "forest01")
            .expect("recoverable job");
        assert_eq!(recovered.id, job.id);
        assert_eq!(
            recovered.migration_destination.as_deref(),
            Some("C:/exports/forest01.area-migration-v1")
        );
        let duplicate = registry
            .create_area_migration_job(
                &analysis.id,
                "forest01".to_owned(),
                PathBuf::from("C:/exports/other.area-migration-v1"),
            )
            .expect_err("duplicate active export");
        assert_eq!(duplicate.code, "MIGRATION_JOB_ALREADY_ACTIVE");
        let analysis_source = registry
            .migration_source(&analysis.id)
            .expect("analysis source");
        let migration_source = registry
            .migration_source(&job.id)
            .expect("migration source");
        assert!(Arc::ptr_eq(
            &analysis_source.source_snapshot,
            &migration_source.source_snapshot
        ));
    }

    fn analysis_with_dependency_hash(sha256: &str) -> ModuleAnalysis {
        serde_json::from_value(json!({
            "fingerprint": { "sha256": "MODULE", "sizeBytes": 1 },
            "inventory": {
                "fileType": "MOD ",
                "fileVersion": "V1.0",
                "buildYear": 2026,
                "buildDay": 215,
                "resourceCount": 0,
                "resources": [],
                "typeSummaries": []
            },
            "moduleInfo": {
                "name": { "stringRef": null, "values": [] },
                "description": { "stringRef": null, "values": [] },
                "tag": "MODULE",
                "minimumGameVersion": null,
                "customTlk": null,
                "entryArea": "start",
                "hakFiles": ["shared"]
            },
            "dependencyReport": {
                "dependencies": [{
                    "kind": "hak",
                    "logicalName": "shared",
                    "state": "resolved",
                    "selectedPath": "C:/game/data/hk/shared.hak",
                    "shadowedPaths": [],
                    "searchedPaths": ["C:/game/data/hk/shared.hak"],
                    "fingerprint": { "sha256": sha256, "sizeBytes": 4 },
                    "change": "first_seen"
                }],
                "resolvedCount": 1,
                "missingCount": 0,
                "uncheckedCount": 0,
                "invalidCount": 0,
                "changedCount": 0
            }
        }))
        .expect("valid analysis fixture")
    }
}
