use crate::{DevelopmentFile, WorkspaceSnapshot, edit_error, sha256_bytes};
use aurora_core::{AppResult, ResourceKey, resource_type_for_extension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};

pub const AURORA_SYNC_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncManifest {
    pub schema_version: u32,
    pub root: String,
    pub files: Vec<DevelopmentFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncWorkspaceFile {
    pub resource: ResourceKey,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncBaselineEntry {
    pub resource: ResourceKey,
    pub relative_path: String,
    pub toolset_sha256: Option<String>,
    pub workspace_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncBaseline {
    pub schema_version: u32,
    pub root: String,
    pub entries: Vec<AuroraSyncBaselineEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuroraSyncState {
    Identical,
    ToolsetOnly,
    WorkspaceOnly,
    ToolsetChanged,
    WorkspaceChanged,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncEntry {
    pub resource: ResourceKey,
    pub relative_path: String,
    pub toolset_sha256: Option<String>,
    pub workspace_sha256: Option<String>,
    pub baseline_toolset_sha256: Option<String>,
    pub baseline_workspace_sha256: Option<String>,
    pub state: AuroraSyncState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncPlan {
    pub schema_version: u32,
    pub root: String,
    pub baseline_found: bool,
    pub entries: Vec<AuroraSyncEntry>,
    pub identical_count: usize,
    pub incoming_count: usize,
    pub outgoing_count: usize,
    pub conflict_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuroraSyncDirection {
    PullFromToolset,
    PushToToolset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncAction {
    pub resource: ResourceKey,
    pub direction: AuroraSyncDirection,
    pub expected_toolset_sha256: Option<String>,
    pub expected_workspace_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncAppliedFile {
    pub resource: ResourceKey,
    pub direction: AuroraSyncDirection,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncReport {
    pub schema_version: u32,
    pub root: String,
    pub applied: Vec<AuroraSyncAppliedFile>,
    pub backups: Vec<String>,
    pub plan: AuroraSyncPlan,
    pub workspace: WorkspaceSnapshot,
}

pub fn resource_key_from_aurora_path(relative_path: &str) -> AppResult<ResourceKey> {
    let path = Path::new(relative_path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_PATH_INVALID",
            format!("{relative_path:?} is not a safe Toolset-relative path"),
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| edit_error("EDIT_AURORA_SYNC_EXTENSION_INVALID", relative_path))?;
    let resource_type = resource_type_for_extension(extension).ok_or_else(|| {
        edit_error(
            "EDIT_AURORA_SYNC_EXTENSION_INVALID",
            format!(".{extension} is not a supported NWN resource extension"),
        )
    })?;
    let resref = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| edit_error("EDIT_AURORA_SYNC_RESREF_INVALID", relative_path))?;
    if resref.is_empty() || resref.len() > 16 || !resref.is_ascii() {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_RESREF_INVALID",
            format!("{resref:?} is outside the NWN ResRef limit"),
        ));
    }
    Ok(ResourceKey::new(resref, resource_type))
}

pub fn compare_aurora_sync(
    toolset: &AuroraSyncManifest,
    workspace_files: &[AuroraSyncWorkspaceFile],
    baseline: Option<&AuroraSyncBaseline>,
) -> AppResult<AuroraSyncPlan> {
    let mut toolset_by_key = BTreeMap::<ResourceKey, (&DevelopmentFile, String)>::new();
    for file in &toolset.files {
        let key = resource_key_from_aurora_path(&file.name)?;
        if toolset_by_key
            .insert(key.clone(), (file, file.name.clone()))
            .is_some()
        {
            return Err(edit_error(
                "EDIT_AURORA_SYNC_DUPLICATE_RESOURCE",
                format!("the Toolset workspace exposes {key} more than once"),
            ));
        }
    }
    let workspace_by_key = workspace_files
        .iter()
        .map(|file| (file.resource.clone(), file))
        .collect::<BTreeMap<_, _>>();
    let baseline_by_key = baseline
        .map(|value| {
            value
                .entries
                .iter()
                .map(|entry| (entry.resource.clone(), entry))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut keys = toolset_by_key.keys().cloned().collect::<Vec<_>>();
    keys.extend(workspace_by_key.keys().cloned());
    keys.extend(baseline_by_key.keys().cloned());
    keys.sort();
    keys.dedup();

    let mut entries = Vec::with_capacity(keys.len());
    for resource in keys {
        let toolset_file = toolset_by_key.get(&resource);
        let workspace_file = workspace_by_key.get(&resource);
        let previous = baseline_by_key.get(&resource);
        let toolset_sha256 = toolset_file.map(|(file, _)| file.sha256.clone());
        let workspace_sha256 = workspace_file.and_then(|file| file.sha256.clone());
        let baseline_toolset_sha256 = previous.and_then(|entry| entry.toolset_sha256.clone());
        let baseline_workspace_sha256 = previous.and_then(|entry| entry.workspace_sha256.clone());
        let state = classify_sync_state(
            toolset_sha256.as_deref(),
            workspace_sha256.as_deref(),
            previous.map(|_| baseline_toolset_sha256.as_deref()),
            previous.map(|_| baseline_workspace_sha256.as_deref()),
        );
        entries.push(AuroraSyncEntry {
            relative_path: toolset_file
                .map(|(_, path)| path.clone())
                .or_else(|| previous.map(|entry| entry.relative_path.clone()))
                .unwrap_or_else(|| resource.file_name()),
            resource,
            toolset_sha256,
            workspace_sha256,
            baseline_toolset_sha256,
            baseline_workspace_sha256,
            state,
        });
    }
    let identical_count = entries
        .iter()
        .filter(|entry| entry.state == AuroraSyncState::Identical)
        .count();
    let incoming_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.state,
                AuroraSyncState::ToolsetOnly | AuroraSyncState::ToolsetChanged
            )
        })
        .count();
    let outgoing_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.state,
                AuroraSyncState::WorkspaceOnly | AuroraSyncState::WorkspaceChanged
            )
        })
        .count();
    let conflict_count = entries
        .iter()
        .filter(|entry| entry.state == AuroraSyncState::Conflict)
        .count();
    Ok(AuroraSyncPlan {
        schema_version: AURORA_SYNC_SCHEMA_VERSION,
        root: toolset.root.clone(),
        baseline_found: baseline.is_some(),
        entries,
        identical_count,
        incoming_count,
        outgoing_count,
        conflict_count,
    })
}

pub fn baseline_from_plan(plan: &AuroraSyncPlan) -> AuroraSyncBaseline {
    AuroraSyncBaseline {
        schema_version: AURORA_SYNC_SCHEMA_VERSION,
        root: plan.root.clone(),
        entries: plan
            .entries
            .iter()
            .map(|entry| AuroraSyncBaselineEntry {
                resource: entry.resource.clone(),
                relative_path: entry.relative_path.clone(),
                toolset_sha256: entry.toolset_sha256.clone(),
                workspace_sha256: entry.workspace_sha256.clone(),
            })
            .collect(),
    }
}

fn classify_sync_state(
    toolset: Option<&str>,
    workspace: Option<&str>,
    baseline_toolset: Option<Option<&str>>,
    baseline_workspace: Option<Option<&str>>,
) -> AuroraSyncState {
    if toolset == workspace {
        return AuroraSyncState::Identical;
    }
    let (Some(baseline_toolset), Some(baseline_workspace)) = (baseline_toolset, baseline_workspace)
    else {
        return match (toolset, workspace) {
            (Some(_), None) => AuroraSyncState::ToolsetOnly,
            (None, Some(_)) => AuroraSyncState::WorkspaceOnly,
            _ => AuroraSyncState::Conflict,
        };
    };
    let toolset_changed = toolset != baseline_toolset;
    let workspace_changed = workspace != baseline_workspace;
    match (toolset_changed, workspace_changed) {
        (true, false) => AuroraSyncState::ToolsetChanged,
        (false, true) => AuroraSyncState::WorkspaceChanged,
        (true, true) => AuroraSyncState::Conflict,
        (false, false) => AuroraSyncState::Conflict,
    }
}

pub fn verify_sync_action(entry: &AuroraSyncEntry, action: &AuroraSyncAction) -> AppResult<()> {
    if entry.resource != action.resource
        || entry.toolset_sha256 != action.expected_toolset_sha256
        || entry.workspace_sha256 != action.expected_workspace_sha256
    {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_PRECONDITION_FAILED",
            format!(
                "{} changed after the synchronization preview",
                action.resource
            ),
        ));
    }
    Ok(())
}

pub fn content_digest(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(sha256_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, sha256: &str) -> DevelopmentFile {
        DevelopmentFile {
            name: name.to_owned(),
            sha256: sha256.to_owned(),
            size_bytes: 1,
        }
    }

    #[test]
    fn detects_initial_conflicts_and_one_sided_files() {
        let toolset = AuroraSyncManifest {
            schema_version: 1,
            root: "C:/toolset".to_owned(),
            files: vec![file("start.nss", "toolset"), file("only.dlg", "incoming")],
        };
        let workspace = vec![
            AuroraSyncWorkspaceFile {
                resource: ResourceKey::new("start", 2009),
                sha256: Some("workspace".to_owned()),
                size_bytes: Some(1),
            },
            AuroraSyncWorkspaceFile {
                resource: ResourceKey::new("out", 2014),
                sha256: Some("outgoing".to_owned()),
                size_bytes: Some(1),
            },
        ];
        let plan = compare_aurora_sync(&toolset, &workspace, None).expect("plan");
        assert_eq!(plan.conflict_count, 1);
        assert_eq!(plan.incoming_count, 1);
        assert_eq!(plan.outgoing_count, 1);
    }

    #[test]
    fn uses_the_baseline_for_three_way_change_detection() {
        let toolset = AuroraSyncManifest {
            schema_version: 1,
            root: "C:/toolset".to_owned(),
            files: vec![file("start.nss", "incoming")],
        };
        let workspace = vec![AuroraSyncWorkspaceFile {
            resource: ResourceKey::new("start", 2009),
            sha256: Some("old".to_owned()),
            size_bytes: Some(1),
        }];
        let baseline = AuroraSyncBaseline {
            schema_version: AURORA_SYNC_SCHEMA_VERSION,
            root: toolset.root.clone(),
            entries: vec![AuroraSyncBaselineEntry {
                resource: ResourceKey::new("start", 2009),
                relative_path: "start.nss".to_owned(),
                toolset_sha256: Some("old".to_owned()),
                workspace_sha256: Some("old".to_owned()),
            }],
        };
        let plan = compare_aurora_sync(&toolset, &workspace, Some(&baseline)).expect("plan");
        assert_eq!(plan.entries[0].state, AuroraSyncState::ToolsetChanged);
    }

    #[test]
    fn rejects_duplicate_resource_names_in_nested_folders() {
        let toolset = AuroraSyncManifest {
            schema_version: 1,
            root: "C:/toolset".to_owned(),
            files: vec![file("one/start.nss", "a"), file("two/start.nss", "b")],
        };
        let error = compare_aurora_sync(&toolset, &[], None).expect_err("duplicate");
        assert_eq!(error.code, "EDIT_AURORA_SYNC_DUPLICATE_RESOURCE");
    }
}
