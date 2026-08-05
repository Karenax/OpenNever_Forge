use crate::{AGENT_RUN_SCHEMA_VERSION, AgentPolicy, ModuleBlueprint};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAiResponses,
    OpenAiChatCompletions,
    Ollama,
    Compatible,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub temperature_milli: Option<u16>,
    pub supports_tools: bool,
    pub supports_parallel_tools: bool,
    pub supports_structured_output: bool,
    #[serde(default)]
    pub store_responses: bool,
    #[serde(default)]
    pub input_cost_micro_usd_per_million_tokens: u64,
    #[serde(default)]
    pub output_cost_micro_usd_per_million_tokens: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Planned,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Proposed,
    WaitingApproval,
    Running,
    Completed,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallRecord {
    pub id: String,
    pub capability_id: String,
    pub arguments: Value,
    pub arguments_sha256: String,
    pub status: ToolCallStatus,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub started_unix_ms: Option<u64>,
    pub completed_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderToolOutput {
    pub call_id: String,
    pub output: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConversationState {
    pub previous_response_id: Option<String>,
    pub pending_tool_outputs: Vec<ProviderToolOutput>,
    pub replay_items: Vec<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_call_id: String,
    #[serde(default)]
    pub tool_call_ids: Vec<String>,
    pub capability_id: String,
    pub summary: String,
    pub status: ApprovalStatus,
    pub created_unix_ms: u64,
    pub resolved_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKind {
    Created,
    Started,
    ModelRequest,
    ModelResponse,
    ToolProposed,
    ApprovalRequested,
    ApprovalResolved,
    ToolCompleted,
    Checkpoint,
    Validation,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub sequence: u64,
    pub unix_ms: u64,
    pub kind: AgentEventKind,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub schema_version: u32,
    pub id: String,
    pub job_id: String,
    pub workspace_id: String,
    pub objective: String,
    pub status: AgentRunStatus,
    pub provider: ProviderProfile,
    pub policy: AgentPolicy,
    pub blueprint: Option<ModuleBlueprint>,
    pub current_turn: u32,
    pub estimated_cost_micro_usd: u64,
    pub tool_calls: Vec<ToolCallRecord>,
    pub approvals: Vec<ApprovalRequest>,
    #[serde(default)]
    pub provider_conversation: ProviderConversationState,
    pub events: Vec<AgentEvent>,
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
}

impl AgentRun {
    pub fn new(
        job_id: impl Into<String>,
        workspace_id: impl Into<String>,
        objective: impl Into<String>,
        provider: ProviderProfile,
        policy: AgentPolicy,
        unix_ms: u64,
    ) -> Self {
        let mut run = Self {
            schema_version: AGENT_RUN_SCHEMA_VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.into(),
            workspace_id: workspace_id.into(),
            objective: objective.into(),
            status: AgentRunStatus::Planned,
            provider,
            policy,
            blueprint: None,
            current_turn: 0,
            estimated_cost_micro_usd: 0,
            tool_calls: Vec::new(),
            approvals: Vec::new(),
            provider_conversation: ProviderConversationState::default(),
            events: Vec::new(),
            created_unix_ms: unix_ms,
            updated_unix_ms: unix_ms,
        };
        run.push_event(unix_ms, AgentEventKind::Created, "Exécution créée.", None);
        run
    }

    pub fn push_event(
        &mut self,
        unix_ms: u64,
        kind: AgentEventKind,
        message: impl Into<String>,
        data: Option<Value>,
    ) {
        self.updated_unix_ms = unix_ms;
        self.events.push(AgentEvent {
            sequence: self.events.len() as u64 + 1,
            unix_ms,
            kind,
            message: message.into(),
            data,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SecurityLevel, built_in_policy};

    #[test]
    fn run_events_are_monotonic() {
        let provider = ProviderProfile {
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
        };
        let mut run = AgentRun::new(
            "job",
            "workspace",
            "Créer un module",
            provider,
            built_in_policy(SecurityLevel::Advisor),
            10,
        );
        run.push_event(11, AgentEventKind::Started, "Début", None);
        assert_eq!(run.events[0].sequence, 1);
        assert_eq!(run.events[1].sequence, 2);
        assert_eq!(run.updated_unix_ms, 11);
    }
}
