use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_project::{
    HashProgress, ModuleAnalysis, ModuleDependencyReport, compare_dependency_reports,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub error: Option<AppError>,
}

struct JobEntry {
    cancellation: Arc<AtomicBool>,
    snapshot: JobSnapshot,
}

#[derive(Default)]
pub struct JobRegistry {
    entries: Mutex<HashMap<String, JobEntry>>,
    dependency_baselines: Mutex<HashMap<String, ModuleDependencyReport>>,
}

impl JobRegistry {
    pub fn create_analysis_job(&self, source_path: String) -> (JobSnapshot, Arc<AtomicBool>) {
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
            },
            result: None,
            error: None,
        };

        self.entries.lock().expect("job registry poisoned").insert(
            id,
            JobEntry {
                cancellation: Arc::clone(&cancellation),
                snapshot: snapshot.clone(),
            },
        );
        (snapshot, cancellation)
    }

    pub fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.entries
            .lock()
            .expect("job registry poisoned")
            .get(id)
            .map(|entry| entry.snapshot.transport_clone())
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

    pub fn set_running(&self, id: &str) -> Option<JobSnapshot> {
        self.update(id, |snapshot| snapshot.state = JobState::Running)
    }

    pub fn set_progress(&self, id: &str, progress: HashProgress) -> Option<JobSnapshot> {
        self.update(id, |snapshot| snapshot.progress = progress)
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
                structured_summary: analysis.structured_summary.clone(),
                script_index: Default::default(),
                script_index_summary: analysis.script_index_summary.clone(),
                dialogue_index: Default::default(),
                dialogue_index_summary: analysis.dialogue_index_summary.clone(),
                world_index: Default::default(),
                world_summary: analysis.world_summary.clone(),
            }),
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
