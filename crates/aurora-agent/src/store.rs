use crate::{AGENT_POLICY_SCHEMA_VERSION, AGENT_RUN_SCHEMA_VERSION, AgentPolicy, AgentRun};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use serde::Serialize;
use sha2::Digest;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AgentWorkspaceStore {
    root: PathBuf,
}

impl AgentWorkspaceStore {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            root: workspace_root.into().join("agent"),
        }
    }

    pub fn policy_path(&self) -> PathBuf {
        self.root.join("policy.json")
    }

    pub fn run_path(&self, run_id: &str) -> PathBuf {
        self.root.join("runs").join(format!("{run_id}.json"))
    }

    pub fn save_policy(&self, policy: &AgentPolicy) -> AppResult<()> {
        if policy.schema_version != AGENT_POLICY_SCHEMA_VERSION {
            return Err(agent_error(
                "AGENT_POLICY_SCHEMA_UNSUPPORTED",
                "Le profil de sécurité utilise une version non prise en charge.",
                format!("unsupported policy schema {}", policy.schema_version),
            ));
        }
        save_json(&self.policy_path(), policy)
    }

    pub fn load_policy(&self) -> AppResult<Option<AgentPolicy>> {
        load_json(&self.policy_path())
            .and_then(|policy: Option<AgentPolicy>| policy.map(migrate_policy).transpose())
    }

    pub fn save_run(&self, run: &AgentRun) -> AppResult<()> {
        if run.schema_version != AGENT_RUN_SCHEMA_VERSION {
            return Err(agent_error(
                "AGENT_RUN_SCHEMA_UNSUPPORTED",
                "L’exécution IA utilise une version non prise en charge.",
                format!("unsupported run schema {}", run.schema_version),
            ));
        }
        let persisted = persistable_run(run);
        save_json(&self.run_path(&run.id), &persisted)?;
        self.append_missing_audit_events(&persisted)
    }

    pub fn load_run(&self, id: &str) -> AppResult<Option<AgentRun>> {
        if !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(agent_error(
                "AGENT_RUN_ID_INVALID",
                "L’identifiant d’exécution IA n’est pas valide.",
                format!("invalid run id {id}"),
            ));
        }
        load_json(&self.run_path(id))
            .and_then(|run: Option<AgentRun>| run.map(migrate_run).transpose())
    }

    pub fn list_runs(&self) -> AppResult<Vec<AgentRun>> {
        let directory = self.root.join("runs");
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut runs = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|error| {
            Box::new(AppError::io(
                "read agent runs",
                directory.display().to_string(),
                &error,
            ))
        })? {
            let entry = entry.map_err(|error| {
                Box::new(AppError::io(
                    "read agent run entry",
                    directory.display().to_string(),
                    &error,
                ))
            })?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Some(run) = load_json::<AgentRun>(&path)? {
                runs.push(migrate_run(run)?);
            }
        }
        runs.sort_by_key(|run| std::cmp::Reverse(run.updated_unix_ms));
        Ok(runs)
    }

    pub fn audit_path(&self, run_id: &str) -> PathBuf {
        self.root.join("audit").join(format!("{run_id}.jsonl"))
    }

    fn append_missing_audit_events(&self, run: &AgentRun) -> AppResult<()> {
        let path = self.audit_path(&run.id);
        let last_sequence = if path.is_file() {
            fs::read_to_string(&path)
                .map_err(|error| {
                    Box::new(AppError::io(
                        "read agent audit",
                        path.display().to_string(),
                        &error,
                    ))
                })?
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter_map(|value| value.get("sequence").and_then(serde_json::Value::as_u64))
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let missing = run
            .events
            .iter()
            .filter(|event| event.sequence > last_sequence)
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(());
        }
        let parent = path.parent().expect("audit path has a parent");
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create agent audit directory",
                parent.display().to_string(),
                &error,
            ))
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                Box::new(AppError::io(
                    "open agent audit",
                    path.display().to_string(),
                    &error,
                ))
            })?;
        for event in missing {
            serde_json::to_writer(&mut file, event).map_err(|error| {
                agent_error(
                    "AGENT_AUDIT_SERIALIZE_FAILED",
                    "Le journal d’audit IA n’a pas pu être écrit.",
                    error.to_string(),
                )
            })?;
            file.write_all(b"\n").map_err(|error| {
                Box::new(AppError::io(
                    "append agent audit",
                    path.display().to_string(),
                    &error,
                ))
            })?;
        }
        file.sync_all().map_err(|error| {
            Box::new(AppError::io(
                "sync agent audit",
                path.display().to_string(),
                &error,
            ))
        })?;
        Ok(())
    }
}

fn persistable_run(run: &AgentRun) -> AgentRun {
    if run.policy.context.retain_conversation && run.policy.context.retention_days > 0 {
        return run.clone();
    }
    let mut persisted = run.clone();
    let terminal = matches!(
        persisted.status,
        crate::AgentRunStatus::Completed
            | crate::AgentRunStatus::Failed
            | crate::AgentRunStatus::Cancelled
    );
    if terminal {
        persisted.objective = "[redacted by retention policy]".to_owned();
        persisted.blueprint = None;
        persisted.provider_conversation.previous_response_id = None;
        persisted.provider_conversation.pending_tool_outputs.clear();
        persisted.provider_conversation.replay_items.clear();
        for call in &mut persisted.tool_calls {
            call.arguments = serde_json::json!({
                "redacted": true,
                "sha256": call.arguments_sha256,
            });
            if let Some(result) = call.result.take() {
                let bytes = serde_json::to_vec(&result).unwrap_or_default();
                call.result = Some(serde_json::json!({
                    "redacted": true,
                    "sizeBytes": bytes.len(),
                    "sha256": hex::encode(sha2::Sha256::digest(bytes)),
                }));
            }
        }
    }
    for event in &mut persisted.events {
        if matches!(
            event.kind,
            crate::AgentEventKind::ModelRequest | crate::AgentEventKind::ModelResponse
        ) {
            event.message = "Réponse du fournisseur non conservée par la politique.".to_owned();
            event.data = None;
        }
    }
    persisted
}

fn migrate_policy(mut policy: AgentPolicy) -> AppResult<AgentPolicy> {
    if policy.schema_version == 1 {
        policy.schema_version = AGENT_POLICY_SCHEMA_VERSION;
    }
    if policy.schema_version != AGENT_POLICY_SCHEMA_VERSION {
        return Err(agent_error(
            "AGENT_POLICY_SCHEMA_UNSUPPORTED",
            "Le profil de sécurité utilise une version non prise en charge.",
            format!("unsupported policy schema {}", policy.schema_version),
        ));
    }
    Ok(policy)
}

fn migrate_run(mut run: AgentRun) -> AppResult<AgentRun> {
    run.policy = migrate_policy(run.policy)?;
    Ok(run)
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> AppResult<Option<T>> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| {
        Box::new(AppError::io(
            "read agent state",
            path.display().to_string(),
            &error,
        ))
    })?;
    serde_json::from_slice(&bytes).map(Some).map_err(|error| {
        agent_error(
            "AGENT_STATE_INVALID",
            "L’état IA enregistré est invalide.",
            format!("cannot decode {}: {error}", path.display()),
        )
    })
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        agent_error(
            "AGENT_STATE_SERIALIZE_FAILED",
            "L’état IA n’a pas pu être enregistré.",
            error.to_string(),
        )
    })?;
    let parent = path.parent().ok_or_else(|| {
        agent_error(
            "AGENT_STATE_PATH_INVALID",
            "Le chemin de l’état IA n’est pas valide.",
            path.display().to_string(),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        Box::new(AppError::io(
            "create agent state directory",
            parent.display().to_string(),
            &error,
        ))
    })?;
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| {
            Box::new(AppError::io(
                "create agent state",
                temporary.display().to_string(),
                &error,
            ))
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            Box::new(AppError::io(
                "write agent state",
                temporary.display().to_string(),
                &error,
            ))
        })?;
    fs::rename(&temporary, path).map_err(|error| {
        Box::new(AppError::io(
            "replace agent state",
            path.display().to_string(),
            &error,
        ))
    })?;
    Ok(())
}

fn agent_error(
    code: impl Into<String>,
    user_message: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(code, user_message, technical_message, ErrorSeverity::Error)
            .with_import_stage("agent_runtime"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentEventKind, AgentRunStatus, ProviderKind, ProviderProfile, SecurityLevel,
        ToolCallRecord, ToolCallStatus, built_in_policy,
    };

    fn manual_provider() -> ProviderProfile {
        ProviderProfile {
            id: "manual".to_owned(),
            name: "Manuel".to_owned(),
            kind: ProviderKind::Manual,
            endpoint: String::new(),
            model: String::new(),
            reasoning_effort: None,
            temperature_milli: None,
            supports_tools: false,
            supports_parallel_tools: false,
            supports_structured_output: true,
            store_responses: false,
            input_cost_micro_usd_per_million_tokens: 0,
            output_cost_micro_usd_per_million_tokens: 0,
        }
    }

    #[test]
    fn persists_policy_and_run_atomically() {
        let temporary = tempfile::tempdir().expect("temporary");
        let store = AgentWorkspaceStore::new(temporary.path());
        let policy = built_in_policy(SecurityLevel::Supervised);
        store.save_policy(&policy).expect("save policy");
        assert_eq!(
            store.load_policy().expect("load policy"),
            Some(policy.clone())
        );

        let run = AgentRun::new("job", "workspace", "Objectif", manual_provider(), policy, 1);
        store.save_run(&run).expect("save run");
        assert_eq!(
            store.load_run(&run.id).expect("load run"),
            Some(run.clone())
        );
        assert_eq!(store.list_runs().expect("list runs").len(), 1);
        let first_audit = fs::read_to_string(store.audit_path(&run.id)).expect("audit");
        assert_eq!(first_audit.lines().count(), 1);
        store.save_run(&run).expect("save run again");
        let second_audit = fs::read_to_string(store.audit_path(&run.id)).expect("audit");
        assert_eq!(second_audit.lines().count(), 1);
    }

    #[test]
    fn redacts_completed_content_when_conversation_retention_is_disabled() {
        let temporary = tempfile::tempdir().expect("temporary");
        let store = AgentWorkspaceStore::new(temporary.path());
        let mut policy = built_in_policy(SecurityLevel::Supervised);
        policy.context.retain_conversation = false;
        policy.context.retention_days = 0;
        let mut run = AgentRun::new(
            "job",
            "workspace",
            "Objectif confidentiel",
            manual_provider(),
            policy,
            1,
        );
        run.tool_calls.push(ToolCallRecord {
            id: "call-1".to_owned(),
            capability_id: "resource.read".to_owned(),
            arguments: serde_json::json!({ "resref": "secret", "resourceType": 2009 }),
            arguments_sha256: "ARGUMENT-DIGEST".to_owned(),
            status: ToolCallStatus::Completed,
            result: Some(serde_json::json!({ "source": "private source" })),
            error: None,
            started_unix_ms: Some(2),
            completed_unix_ms: Some(3),
        });
        run.push_event(
            4,
            AgentEventKind::ModelResponse,
            "private provider response",
            None,
        );
        run.provider_conversation.replay_items =
            vec![serde_json::json!({"role":"user","content":"private replay"})];
        run.status = AgentRunStatus::Completed;

        store.save_run(&run).expect("save redacted run");
        let loaded = store
            .load_run(&run.id)
            .expect("load redacted run")
            .expect("persisted run");
        assert_eq!(loaded.tool_calls[0].arguments["redacted"], true);
        assert_eq!(loaded.tool_calls[0].arguments["sha256"], "ARGUMENT-DIGEST");
        assert_eq!(
            loaded.tool_calls[0].result.as_ref().unwrap()["redacted"],
            true
        );
        assert!(
            loaded
                .events
                .iter()
                .find(|event| event.kind == AgentEventKind::ModelResponse)
                .expect("model event")
                .message
                .contains("conserv")
        );
        let persisted_text = fs::read_to_string(store.run_path(&run.id)).expect("run json");
        assert!(!persisted_text.contains("private source"));
        assert!(!persisted_text.contains("private provider response"));
        assert!(!persisted_text.contains("Objectif confidentiel"));
        assert!(!persisted_text.contains("private replay"));
    }

    #[test]
    fn keeps_active_tool_state_recoverable_without_logging_provider_content() {
        let temporary = tempfile::tempdir().expect("temporary");
        let store = AgentWorkspaceStore::new(temporary.path());
        let mut policy = built_in_policy(SecurityLevel::Supervised);
        policy.context.retain_conversation = false;
        let mut run = AgentRun::new(
            "job",
            "workspace",
            "Objectif à reprendre",
            manual_provider(),
            policy,
            1,
        );
        run.status = AgentRunStatus::WaitingApproval;
        run.tool_calls.push(ToolCallRecord {
            id: "call-1".to_owned(),
            capability_id: "script.create".to_owned(),
            arguments: serde_json::json!({ "resref": "resume_me", "source": "void main() {}" }),
            arguments_sha256: "ARGUMENT-DIGEST".to_owned(),
            status: ToolCallStatus::WaitingApproval,
            result: None,
            error: None,
            started_unix_ms: None,
            completed_unix_ms: None,
        });
        run.push_event(
            2,
            AgentEventKind::ModelRequest,
            "private request",
            Some(serde_json::json!({ "private": true })),
        );
        run.provider_conversation.replay_items =
            vec![serde_json::json!({"role":"user","content":"active replay"})];

        store.save_run(&run).expect("save recoverable run");
        let loaded = store
            .load_run(&run.id)
            .expect("load recoverable run")
            .expect("persisted run");
        assert_eq!(loaded.objective, "Objectif à reprendre");
        assert_eq!(loaded.tool_calls[0].arguments["resref"], "resume_me");
        let model_event = loaded
            .events
            .iter()
            .find(|event| event.kind == AgentEventKind::ModelRequest)
            .expect("model request");
        assert!(model_event.message.contains("conserv"));
        assert!(model_event.data.is_none());
        assert_eq!(loaded.provider_conversation.replay_items.len(), 1);
        let audit = fs::read_to_string(store.audit_path(&run.id)).expect("audit");
        assert!(!audit.contains("private request"));
    }

    #[test]
    fn migrates_a_version_one_policy_with_safe_runtime_defaults() {
        let temporary = tempfile::tempdir().expect("temporary");
        let store = AgentWorkspaceStore::new(temporary.path());
        let mut value = serde_json::to_value(built_in_policy(SecurityLevel::Advisor))
            .expect("serialize policy");
        value["schemaVersion"] = serde_json::json!(1);
        value.as_object_mut().expect("object").remove("toolRuntime");
        value["context"]
            .as_object_mut()
            .expect("context")
            .remove("allowedProviderHosts");
        fs::create_dir_all(store.policy_path().parent().expect("parent")).expect("directory");
        fs::write(
            store.policy_path(),
            serde_json::to_vec_pretty(&value).expect("encode legacy policy"),
        )
        .expect("legacy policy");
        let migrated = store.load_policy().expect("load policy").expect("policy");
        assert_eq!(migrated.schema_version, AGENT_POLICY_SCHEMA_VERSION);
        assert!(migrated.tool_runtime.compiler_path.is_empty());
        assert!(
            migrated
                .context
                .allowed_provider_hosts
                .contains(&"localhost".to_owned())
        );
    }
}
