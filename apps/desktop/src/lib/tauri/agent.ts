import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type {
  AgentPolicy,
  AgentProviderTestReport,
  AgentRun,
  AgentStudioState,
  AiApplyReport,
  AiChangeSet,
  AiChangeSetPreview,
  AiConsent,
  AiProviderProposal,
  ProviderProfile,
  ResourceKey,
} from "./types";

export async function previewAiChangeSet(request: { jobId: string; workspaceId: string; changeSet: AiChangeSet }): Promise<AiChangeSetPreview> {
  requireTauri();
  return invoke<AiChangeSetPreview>("preview_ai_change_set", { request });
}

export async function requestAiChangeSet(request: {
  jobId: string; workspaceId: string; endpoint: string; model: string; apiKey?: string;
  prompt: string; selectedResources: ResourceKey[]; consent: AiConsent;
}): Promise<AiProviderProposal> {
  requireTauri();
  return invoke<AiProviderProposal>("request_ai_change_set", { request });
}

export async function applyAiChangeSet(request: {
  jobId: string; workspaceId: string; proposalSha256: string; changeSet: AiChangeSet; confirmed: boolean;
}): Promise<AiApplyReport> {
  requireTauri();
  return invoke<AiApplyReport>("apply_ai_change_set", { request });
}

export async function getAgentStudioState(workspaceId: string): Promise<AgentStudioState> {
  requireTauri();
  return invoke<AgentStudioState>("get_agent_studio_state", { request: { workspaceId } });
}

export async function saveAgentPolicy(workspaceId: string, policy: AgentPolicy): Promise<AgentStudioState> {
  requireTauri();
  return invoke<AgentStudioState>("save_agent_policy", { request: { workspaceId, policy } });
}

export async function createAgentRun(request: {
  jobId: string; workspaceId: string; objective: string; provider: ProviderProfile; policy?: AgentPolicy; blueprint?: unknown;
}): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("create_agent_run", { request });
}

export async function advanceAgentRun(request: { workspaceId: string; runId: string; apiKey?: string }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("advance_agent_run", { request });
}

export async function testAgentProvider(request: {
  provider: ProviderProfile; policy: AgentPolicy; apiKey?: string;
}): Promise<AgentProviderTestReport> {
  requireTauri();
  return invoke<AgentProviderTestReport>("test_agent_provider", { request });
}

export async function resolveAgentApproval(request: { workspaceId: string; runId: string; approvalId: string; approved: boolean }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("resolve_agent_approval", { request });
}

export async function cancelAgentRun(request: { workspaceId: string; runId: string }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("cancel_agent_run", { request });
}
