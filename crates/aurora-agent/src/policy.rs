use crate::AGENT_POLICY_SCHEMA_VERSION;
use crate::registry::{CapabilityDescriptor, CapabilityRisk, CapabilitySideEffect};
use aurora_core::ResourceKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SecurityLevel {
    Observer,
    Advisor,
    Assisted,
    Supervised,
    Autonomous,
    Operator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAccess {
    Deny,
    Read,
    Preview,
    Execute,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    Always,
    PerBatch,
    AboveRisk,
    Never,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolScope {
    SelectedResource,
    Area,
    Module,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentLimits {
    pub max_turns: u32,
    pub max_tool_calls: u32,
    pub max_parallel_calls: u32,
    pub max_retries: u32,
    pub max_prompt_bytes: usize,
    pub max_context_resources: usize,
    pub max_context_resource_bytes: usize,
    pub max_response_bytes: usize,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    pub max_duration_seconds: u64,
    pub max_cost_micro_usd: u64,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_turns: 12,
            max_tool_calls: 48,
            max_parallel_calls: 4,
            max_retries: 2,
            max_prompt_bytes: 128 * 1024,
            max_context_resources: 16,
            max_context_resource_bytes: 256 * 1024,
            max_response_bytes: 2 * 1024 * 1024,
            max_output_tokens: default_max_output_tokens(),
            max_duration_seconds: 15 * 60,
            max_cost_micro_usd: 5_000_000,
        }
    }
}

const fn default_max_output_tokens() -> u32 {
    8_192
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextPolicy {
    pub allow_network: bool,
    pub include_module_metadata: bool,
    pub include_resource_contents: bool,
    pub include_diagnostics: bool,
    pub include_architecture_graph: bool,
    #[serde(default)]
    pub include_local_paths: bool,
    pub retain_conversation: bool,
    pub retention_days: u16,
    #[serde(default = "default_allow_insecure_local_http")]
    pub allow_insecure_local_http: bool,
    #[serde(default = "default_provider_hosts")]
    pub allowed_provider_hosts: Vec<String>,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            include_module_metadata: false,
            include_resource_contents: false,
            include_diagnostics: true,
            include_architecture_graph: false,
            include_local_paths: false,
            retain_conversation: true,
            retention_days: 30,
            allow_insecure_local_http: default_allow_insecure_local_http(),
            allowed_provider_hosts: default_provider_hosts(),
        }
    }
}

fn default_allow_insecure_local_http() -> bool {
    true
}

fn default_provider_hosts() -> Vec<String> {
    vec![
        "api.openai.com".to_owned(),
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolRuntimePolicy {
    pub compiler_path: String,
    pub game_install_path: String,
    pub include_paths: Vec<String>,
    pub development_path: String,
    pub toolset_temp_path: String,
    pub allowed_output_roots: Vec<String>,
    pub nwn_executable_path: String,
    pub nwn_working_directory: String,
    pub nwn_arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityOverride {
    pub access: CapabilityAccess,
    pub approval: ApprovalMode,
    pub scope: ToolScope,
    pub max_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScopeGrants {
    pub selected_resources: Vec<ResourceKey>,
    pub areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPolicy {
    pub schema_version: u32,
    pub name: String,
    pub level: SecurityLevel,
    pub context: ContextPolicy,
    pub limits: AgentLimits,
    #[serde(default)]
    pub tool_runtime: ToolRuntimePolicy,
    pub capability_overrides: BTreeMap<String, CapabilityOverride>,
    #[serde(default)]
    pub scope_grants: ScopeGrants,
    pub allow_development_deploy: bool,
    pub allow_toolset_sync: bool,
    pub allow_process_launch: bool,
    pub stop_on_validation_error: bool,
    pub checkpoint_before_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveCapability {
    pub id: String,
    pub access: CapabilityAccess,
    pub approval: ApprovalMode,
    pub scope: ToolScope,
    pub max_calls: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    Denied { reason: String },
    ApprovalRequired { reason: String },
    Allowed,
}

pub fn built_in_policy(level: SecurityLevel) -> AgentPolicy {
    let (name, default_access, default_approval) = match level {
        SecurityLevel::Observer => ("Observateur", CapabilityAccess::Read, ApprovalMode::Always),
        SecurityLevel::Advisor => (
            "Conseiller",
            CapabilityAccess::Preview,
            ApprovalMode::Always,
        ),
        SecurityLevel::Assisted => (
            "Assistant",
            CapabilityAccess::Execute,
            ApprovalMode::PerBatch,
        ),
        SecurityLevel::Supervised => (
            "Agent supervisé",
            CapabilityAccess::Execute,
            ApprovalMode::AboveRisk,
        ),
        SecurityLevel::Autonomous => (
            "Constructeur autonome",
            CapabilityAccess::Execute,
            ApprovalMode::AboveRisk,
        ),
        SecurityLevel::Operator => (
            "Opérateur expert",
            CapabilityAccess::Execute,
            ApprovalMode::AboveRisk,
        ),
    };
    let mut policy = AgentPolicy {
        schema_version: AGENT_POLICY_SCHEMA_VERSION,
        name: name.to_owned(),
        level,
        context: ContextPolicy::default(),
        limits: AgentLimits::default(),
        tool_runtime: ToolRuntimePolicy::default(),
        capability_overrides: BTreeMap::new(),
        scope_grants: ScopeGrants::default(),
        allow_development_deploy: level >= SecurityLevel::Operator,
        allow_toolset_sync: level >= SecurityLevel::Operator,
        allow_process_launch: level >= SecurityLevel::Operator,
        stop_on_validation_error: true,
        checkpoint_before_write: true,
    };
    if level >= SecurityLevel::Assisted {
        policy.context.include_module_metadata = true;
    }
    if level >= SecurityLevel::Supervised {
        policy.context.include_resource_contents = true;
        policy.limits.max_turns = 24;
        policy.limits.max_tool_calls = 128;
    }
    if level >= SecurityLevel::Autonomous {
        policy.limits.max_turns = 64;
        policy.limits.max_tool_calls = 512;
        policy.limits.max_duration_seconds = 60 * 60;
    }
    policy.capability_overrides.insert(
        "*".to_owned(),
        CapabilityOverride {
            access: default_access,
            approval: default_approval,
            scope: ToolScope::Workspace,
            max_calls: policy.limits.max_tool_calls,
        },
    );
    policy
}

pub fn validate_agent_policy(policy: &AgentPolicy) -> Result<(), String> {
    if policy.schema_version != AGENT_POLICY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported agent policy schema {}",
            policy.schema_version
        ));
    }
    if policy.name.trim().is_empty() || policy.name.len() > 128 {
        return Err("policy name must contain between 1 and 128 bytes".to_owned());
    }
    if policy.limits.max_turns == 0 || policy.limits.max_turns > 1_024 {
        return Err("max turns must be between 1 and 1024".to_owned());
    }
    if policy.limits.max_tool_calls == 0 || policy.limits.max_tool_calls > 10_000 {
        return Err("max tool calls must be between 1 and 10000".to_owned());
    }
    if policy.limits.max_parallel_calls == 0 || policy.limits.max_parallel_calls > 64 {
        return Err("max parallel calls must be between 1 and 64".to_owned());
    }
    if policy.limits.max_retries > 20 {
        return Err("max retries cannot exceed 20".to_owned());
    }
    if policy.limits.max_prompt_bytes < 1_024 || policy.limits.max_prompt_bytes > 4 * 1024 * 1024 {
        return Err("max prompt bytes must be between 1 KiB and 4 MiB".to_owned());
    }
    if policy.limits.max_context_resources > 1_024 {
        return Err("max context resources cannot exceed 1024".to_owned());
    }
    if policy.limits.max_context_resource_bytes == 0
        || policy.limits.max_context_resource_bytes > 16 * 1024 * 1024
    {
        return Err("max context resource bytes must be between 1 and 16 MiB".to_owned());
    }
    if policy.limits.max_response_bytes == 0 || policy.limits.max_response_bytes > 32 * 1024 * 1024
    {
        return Err("max response bytes must be between 1 and 32 MiB".to_owned());
    }
    if policy.limits.max_output_tokens == 0 || policy.limits.max_output_tokens > 1_000_000 {
        return Err("max output tokens must be between 1 and 1000000".to_owned());
    }
    if policy.limits.max_duration_seconds == 0
        || policy.limits.max_duration_seconds > 7 * 24 * 60 * 60
    {
        return Err("max duration must be between 1 second and 7 days".to_owned());
    }
    if policy.context.retention_days > 3_650 {
        return Err("conversation retention cannot exceed 3650 days".to_owned());
    }
    if policy.context.allowed_provider_hosts.len() > 128 {
        return Err("provider host allowlist cannot exceed 128 entries".to_owned());
    }
    for host in &policy.context.allowed_provider_hosts {
        if host.trim().is_empty() || host.len() > 253 || host.contains('/') || host.contains('@') {
            return Err(format!("invalid provider host allowlist entry {host:?}"));
        }
    }
    if policy.tool_runtime.include_paths.len() > 128
        || policy.tool_runtime.allowed_output_roots.len() > 128
        || policy.tool_runtime.nwn_arguments.len() > 128
    {
        return Err("tool runtime path lists cannot exceed 128 entries".to_owned());
    }
    let runtime_paths = [
        policy.tool_runtime.compiler_path.as_str(),
        policy.tool_runtime.game_install_path.as_str(),
        policy.tool_runtime.development_path.as_str(),
        policy.tool_runtime.toolset_temp_path.as_str(),
        policy.tool_runtime.nwn_executable_path.as_str(),
        policy.tool_runtime.nwn_working_directory.as_str(),
    ]
    .into_iter()
    .chain(policy.tool_runtime.include_paths.iter().map(String::as_str))
    .chain(
        policy
            .tool_runtime
            .allowed_output_roots
            .iter()
            .map(String::as_str),
    );
    if runtime_paths.into_iter().any(|path| path.len() > 32 * 1024) {
        return Err("tool runtime paths cannot exceed 32 KiB".to_owned());
    }
    if policy
        .tool_runtime
        .nwn_arguments
        .iter()
        .any(|argument| argument.len() > 4 * 1024)
    {
        return Err("NWN arguments cannot exceed 4 KiB each".to_owned());
    }
    if policy.scope_grants.selected_resources.len() > 1_024
        || policy.scope_grants.areas.len() > 1_024
    {
        return Err("scope grants cannot exceed 1024 entries".to_owned());
    }
    for resource in &policy.scope_grants.selected_resources {
        if resource.resref.is_empty()
            || resource.resref.len() > 16
            || !resource
                .resref
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(format!("invalid scoped resource {}", resource));
        }
    }
    for area in &policy.scope_grants.areas {
        if area.is_empty()
            || area.len() > 16
            || !area
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(format!("invalid scoped area {area:?}"));
        }
    }
    if policy.capability_overrides.len() > 1_024 {
        return Err("capability overrides cannot exceed 1024 entries".to_owned());
    }
    for (id, rule) in &policy.capability_overrides {
        if id.trim().is_empty() || id.len() > 128 {
            return Err(format!("invalid capability override id {id:?}"));
        }
        if rule.max_calls > policy.limits.max_tool_calls {
            return Err(format!("capability {id} max calls exceeds the task limit"));
        }
    }
    Ok(())
}

pub fn evaluate_capability(
    policy: &AgentPolicy,
    descriptor: &CapabilityDescriptor,
    calls_so_far: u32,
) -> (EffectiveCapability, PolicyDecision) {
    let rule = policy
        .capability_overrides
        .get(&descriptor.id)
        .or_else(|| policy.capability_overrides.get("*"))
        .cloned()
        .unwrap_or(CapabilityOverride {
            access: CapabilityAccess::Deny,
            approval: ApprovalMode::Always,
            scope: ToolScope::SelectedResource,
            max_calls: 0,
        });
    let mut effective = EffectiveCapability {
        id: descriptor.id.clone(),
        access: rule.access,
        approval: rule.approval,
        scope: rule.scope,
        max_calls: rule.max_calls,
        reason: "Règle du profil appliquée.".to_owned(),
    };

    if descriptor.side_effect != CapabilitySideEffect::None
        && effective.access < CapabilityAccess::Execute
    {
        return (
            effective,
            PolicyDecision::Denied {
                reason: "Cette capacité modifie un état mais le profil n’autorise pas l’exécution."
                    .to_owned(),
            },
        );
    }
    if descriptor.side_effect == CapabilitySideEffect::External
        && ((descriptor.id == "development.deploy" && !policy.allow_development_deploy)
            || (descriptor.id.starts_with("toolset.") && !policy.allow_toolset_sync)
            || (descriptor.id.starts_with("nwn.") && !policy.allow_process_launch))
    {
        effective.reason = "Action externe désactivée par la politique.".to_owned();
        return (
            effective,
            PolicyDecision::Denied {
                reason: "Cette action externe n’est pas autorisée par le profil.".to_owned(),
            },
        );
    }
    if calls_so_far >= effective.max_calls || calls_so_far >= policy.limits.max_tool_calls {
        return (
            effective,
            PolicyDecision::Denied {
                reason: "Le plafond d’appels de cet outil ou de la tâche est atteint.".to_owned(),
            },
        );
    }
    let approval = match effective.approval {
        ApprovalMode::Always | ApprovalMode::PerBatch => true,
        ApprovalMode::AboveRisk => descriptor.risk >= CapabilityRisk::High,
        ApprovalMode::Never => false,
    };
    if approval {
        (
            effective,
            PolicyDecision::ApprovalRequired {
                reason: "La politique exige une confirmation avant cet appel.".to_owned(),
            },
        )
    } else {
        (effective, PolicyDecision::Allowed)
    }
}

pub fn context_allows_capability(policy: &AgentPolicy, capability_id: &str) -> bool {
    match capability_id {
        "module.inspect" | "resource.search" => policy.context.include_module_metadata,
        "resource.read" => policy.context.include_resource_contents,
        "diagnostics.run" | "module.validate" => policy.context.include_diagnostics,
        "architecture.query" => policy.context.include_architecture_graph,
        _ => true,
    }
}

pub fn sanitize_context_value(value: &Value, include_local_paths: bool) -> Value {
    if include_local_paths {
        return value.clone();
    }
    match value {
        Value::String(text) if Path::new(text).is_absolute() => {
            Value::String("[local path redacted]".to_owned())
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| sanitize_context_value(value, false))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    let normalized = key.to_ascii_lowercase();
                    let sensitive_key = normalized == "root"
                        || normalized.ends_with("path")
                        || normalized.ends_with("paths")
                        || normalized.ends_with("directory");
                    (
                        key.clone(),
                        if sensitive_key {
                            Value::String("[local path redacted]".to_owned())
                        } else {
                            sanitize_context_value(value, false)
                        },
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub fn validate_tool_scope(
    policy: &AgentPolicy,
    effective: &EffectiveCapability,
    descriptor: &CapabilityDescriptor,
    arguments: &Value,
) -> Result<(), String> {
    match effective.scope {
        ToolScope::Workspace => Ok(()),
        ToolScope::Module => {
            if descriptor.side_effect == CapabilitySideEffect::External {
                Err("Le périmètre module interdit les actions externes au workspace.".to_owned())
            } else {
                Ok(())
            }
        }
        ToolScope::Area => {
            let area = arguments
                .get("area")
                .or_else(|| arguments.get("resref"))
                .and_then(Value::as_str)
                .ok_or_else(|| "Cet appel ne désigne aucune zone vérifiable.".to_owned())?;
            if policy
                .scope_grants
                .areas
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(area))
            {
                Ok(())
            } else {
                Err(format!(
                    "La zone {area} n’appartient pas au périmètre autorisé."
                ))
            }
        }
        ToolScope::SelectedResource => {
            let resource = arguments
                .get("resource")
                .cloned()
                .and_then(|value| serde_json::from_value::<ResourceKey>(value).ok())
                .or_else(|| inferred_resource(descriptor, arguments))
                .ok_or_else(|| "Cet appel ne désigne aucune ressource vérifiable.".to_owned())?;
            if policy
                .scope_grants
                .selected_resources
                .iter()
                .any(|allowed| allowed == &resource)
            {
                Ok(())
            } else {
                Err(format!("La ressource {resource} n’est pas sélectionnée."))
            }
        }
    }
}

fn inferred_resource(descriptor: &CapabilityDescriptor, arguments: &Value) -> Option<ResourceKey> {
    let resref = arguments.get("resref")?.as_str()?;
    let resource_type = match descriptor.id.as_str() {
        "script.replace" | "script.create" => 2009,
        "script.compile" => 2009,
        "dialogue.edit" | "dialogue.create" => 2029,
        _ => return None,
    };
    Some(ResourceKey::new(resref, resource_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityRegistry;

    #[test]
    fn observer_cannot_execute_mutating_tools() {
        let policy = built_in_policy(SecurityLevel::Observer);
        let registry = CapabilityRegistry::standard();
        let tool = registry.get("resource.set_field").expect("tool");
        let (_, decision) = evaluate_capability(&policy, tool, 0);
        assert!(matches!(decision, PolicyDecision::Denied { .. }));
    }

    #[test]
    fn operator_still_requires_approval_for_external_actions() {
        let policy = built_in_policy(SecurityLevel::Operator);
        let registry = CapabilityRegistry::standard();
        let tool = registry.get("development.deploy").expect("tool");
        let (_, decision) = evaluate_capability(&policy, tool, 0);
        assert!(matches!(decision, PolicyDecision::ApprovalRequired { .. }));
    }

    #[test]
    fn rejects_an_unbounded_policy() {
        let mut policy = built_in_policy(SecurityLevel::Autonomous);
        policy.limits.max_tool_calls = 0;
        assert!(validate_agent_policy(&policy).is_err());
    }

    #[test]
    fn context_policy_hides_sensitive_capabilities_independently_of_access() {
        let mut policy = built_in_policy(SecurityLevel::Operator);
        policy.context.include_resource_contents = false;
        policy.context.include_diagnostics = false;
        assert!(!context_allows_capability(&policy, "resource.read"));
        assert!(!context_allows_capability(&policy, "module.validate"));
        policy.context.include_module_metadata = false;
        assert!(!context_allows_capability(&policy, "resource.search"));
        assert!(context_allows_capability(&policy, "area.create"));
    }

    #[test]
    fn selected_resource_scope_is_enforced_against_arguments() {
        let mut policy = built_in_policy(SecurityLevel::Operator);
        policy.scope_grants.selected_resources = vec![ResourceKey::new("allowed", 2009)];
        let registry = CapabilityRegistry::standard();
        let descriptor = registry.get("script.replace").expect("script tool");
        let (mut effective, _) = evaluate_capability(&policy, descriptor, 0);
        effective.scope = ToolScope::SelectedResource;
        assert!(
            validate_tool_scope(
                &policy,
                &effective,
                descriptor,
                &serde_json::json!({"resource":{"resref":"allowed","resourceType":2009}}),
            )
            .is_ok()
        );
        assert!(
            validate_tool_scope(
                &policy,
                &effective,
                descriptor,
                &serde_json::json!({"resource":{"resref":"denied","resourceType":2009}}),
            )
            .is_err()
        );
    }
}
