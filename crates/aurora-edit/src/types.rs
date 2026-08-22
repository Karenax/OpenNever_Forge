use aurora_core::ResourceKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{EditCommand, ModifiedResource, SourceFingerprint, WorkspaceMigrationRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreview {
    pub command: EditCommand,
    pub target: String,
    pub current: Value,
    pub resulting: Value,
    pub valid: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub schema_version: u32,
    pub workspace_id: String,
    pub root: String,
    pub source: SourceFingerprint,
    pub source_intact: bool,
    pub command_count: usize,
    pub cursor: usize,
    pub can_undo: bool,
    pub can_redo: bool,
    pub modified_resources: Vec<ModifiedResource>,
    pub deleted_resources: Vec<ResourceKey>,
    pub journal_events: u64,
    pub values: BTreeMap<String, Value>,
    pub migration_history: Vec<WorkspaceMigrationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleBuildReport {
    pub output_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub resource_count: usize,
    pub modified_resources: usize,
    pub deleted_resources: usize,
    pub source_intact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentDeployment {
    pub workspace_id: String,
    pub development_path: String,
    pub files: Vec<DevelopmentFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentFile {
    pub name: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentCleanupReport {
    pub removed: Vec<String>,
    pub preserved_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewModuleDefinition {
    pub name: String,
    pub tag: String,
    pub entry_area: String,
    pub tileset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteManifest {
    pub schema_version: u32,
    pub categories: Vec<PaletteCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteCategory {
    pub id: String,
    pub label: String,
    pub resource_types: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleBuildProfile {
    pub name: String,
    pub output_name: String,
    pub block_on_warnings: bool,
    pub deploy_development: bool,
    pub hak_files: Vec<String>,
    pub custom_tlk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReproducibleBuildVerification {
    pub profile: ModuleBuildProfile,
    pub first_sha256: String,
    pub second_sha256: String,
    pub identical: bool,
    pub resource_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitWorkspaceStatus {
    pub root: String,
    pub branch: String,
    pub head: Option<String>,
    pub clean: bool,
    pub files: Vec<GitFileStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NwnLaunchMode {
    Client,
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NwnLaunchProfile {
    pub name: String,
    pub mode: NwnLaunchMode,
    pub executable_path: String,
    pub working_directory: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NwnLaunchReport {
    pub profile: NwnLaunchProfile,
    pub process_id: u32,
    pub log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceExportManifest {
    pub schema_version: u32,
    pub workspace_id: String,
    pub source_sha256: String,
    pub files: Vec<DevelopmentFile>,
    pub deleted_resources: Vec<ResourceKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiChangeSet {
    pub summary: String,
    pub commands: Vec<EditCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiChangeSetPreview {
    pub summary: String,
    pub proposal_sha256: String,
    pub all_valid: bool,
    pub previews: Vec<CommandPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiApplyReport {
    pub proposal_sha256: String,
    pub applied_commands: usize,
    pub workspace: WorkspaceSnapshot,
}
