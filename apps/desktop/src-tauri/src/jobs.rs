use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_project::{HashProgress, ModuleAnalysis};
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
            .map(|entry| entry.snapshot.clone())
    }

    pub fn set_running(&self, id: &str) -> Option<JobSnapshot> {
        self.update(id, |snapshot| snapshot.state = JobState::Running)
    }

    pub fn set_progress(&self, id: &str, progress: HashProgress) -> Option<JobSnapshot> {
        self.update(id, |snapshot| snapshot.progress = progress)
    }

    pub fn complete(&self, id: &str, result: ModuleAnalysis) -> Option<JobSnapshot> {
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

        Ok(entry.snapshot.clone())
    }

    fn update(&self, id: &str, update: impl FnOnce(&mut JobSnapshot)) -> Option<JobSnapshot> {
        let mut entries = self.entries.lock().expect("job registry poisoned");
        let entry = entries.get_mut(id)?;
        update(&mut entry.snapshot);
        Some(entry.snapshot.clone())
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

    #[test]
    fn cancelling_a_running_job_sets_the_flag_and_state() {
        let registry = JobRegistry::default();
        let (job, flag) = registry.create_analysis_job("fixture.mod".to_owned());
        registry.set_running(&job.id);

        let cancelled = registry.cancel(&job.id).expect("job exists");

        assert_eq!(cancelled.state, JobState::Cancelling);
        assert!(flag.load(Ordering::Relaxed));
    }
}
