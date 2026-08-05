mod blueprint;
mod policy;
mod provider;
mod registry;
mod run;
mod store;

pub use blueprint::{
    AreaBlueprint, BlueprintDiagnostic, BlueprintExecutionPlan, BlueprintTask, BlueprintValidation,
    DialogueBlueprint, ModuleBlueprint, ModuleRequirement, ScriptBlueprint,
    compile_module_blueprint, validate_module_blueprint,
};
pub use policy::{
    AgentLimits, AgentPolicy, ApprovalMode, CapabilityAccess, CapabilityOverride, ContextPolicy,
    EffectiveCapability, PolicyDecision, ScopeGrants, SecurityLevel, ToolRuntimePolicy, ToolScope,
    built_in_policy, context_allows_capability, evaluate_capability, sanitize_context_value,
    validate_agent_policy, validate_tool_scope,
};
pub use provider::{
    ProviderRequestContext, ProviderStep, ProviderToolCall, build_provider_request,
    decode_provider_response, provider_tool_name,
};
pub use registry::{
    CapabilityDescriptor, CapabilityRegistry, CapabilityRisk, CapabilitySideEffect,
};
pub use run::{
    AgentEvent, AgentEventKind, AgentRun, AgentRunStatus, ApprovalRequest, ApprovalStatus,
    ProviderConversationState, ProviderKind, ProviderProfile, ProviderToolOutput, ToolCallRecord,
    ToolCallStatus,
};
pub use store::AgentWorkspaceStore;

pub const AGENT_POLICY_SCHEMA_VERSION: u32 = 2;
pub const AGENT_RUN_SCHEMA_VERSION: u32 = 1;
pub const MODULE_BLUEPRINT_SCHEMA_VERSION: u32 = 1;
