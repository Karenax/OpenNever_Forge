use aurora_core::{AppError, AppResult, ResourceKey};
use aurora_erf::{
    ContainerReader, ErfReader, ErfResourceInput, ErfResourceSource, ErfResourceStreamInput,
    write_erf_streaming, write_erf_streaming_with_metadata,
};
use aurora_gff::read_module_info;
use aurora_nwscript::parse_nss;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use super::JournalEvent;
use super::{
    AURORA_SYNC_SCHEMA_VERSION, AiApplyReport, AiChangeSet, AiChangeSetPreview, AuroraSyncBaseline,
    CommandPreview, DevelopmentCleanupReport, DevelopmentDeployment, DevelopmentFile,
    EDIT_WORKSPACE_SCHEMA_VERSION, EditCommand, ModifiedResource, ModuleBuildProfile,
    ModuleBuildReport, NwnLaunchProfile, NwnLaunchReport, PersistedWorkspace,
    ReproducibleBuildVerification, ResourceContentDigest, ResourceRevision, SourceFingerprint,
    WorkspaceExportManifest, WorkspaceMigrationRecord, WorkspaceSnapshot, ai_change_set_sha256,
    atomic_copy, atomic_write, canonical_toolset_root, controlled_ai_resource, edit_error,
    edit_gff_field, ensure_output_is_not_source, ensure_safe_workspace_root, is_sha256,
    safe_profile_name, sha256_bytes, sha256_file, validate_build_profile,
    validate_controlled_ai_change_set, validate_launch_profile, verify_source,
};

#[derive(Debug)]
pub struct EditWorkspace {
    pub(crate) root: PathBuf,
    pub(crate) state: PersistedWorkspace,
}

impl EditWorkspace {
    pub fn create(
        root: impl Into<PathBuf>,
        source_path: &Path,
        expected_sha256: &str,
        expected_size: u64,
    ) -> AppResult<Self> {
        let root = root.into();
        verify_source(source_path, expected_sha256, expected_size)?;
        ensure_safe_workspace_root(&root, source_path)?;
        fs::create_dir_all(root.join("resources")).map_err(|error| {
            Box::new(AppError::io(
                "create edit workspace",
                root.display().to_string(),
                &error,
            ))
        })?;
        let workspace_identity_path = if root.is_absolute() {
            root.clone()
        } else {
            std::env::current_dir()
                .map_err(|error| {
                    Box::new(AppError::io(
                        "resolve edit workspace root",
                        root.display().to_string(),
                        &error,
                    ))
                })?
                .join(&root)
        };
        let mut workspace_identity = expected_sha256.to_ascii_lowercase();
        workspace_identity.push('\0');
        workspace_identity.push_str(
            &workspace_identity_path
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase(),
        );
        let workspace_id = sha256_bytes(workspace_identity.as_bytes())
            .chars()
            .take(16)
            .collect::<String>();
        let source = SourceFingerprint {
            path: source_path.display().to_string(),
            sha256: expected_sha256.to_ascii_lowercase(),
            size_bytes: expected_size,
        };
        let mut workspace = Self {
            root,
            state: PersistedWorkspace {
                schema_version: EDIT_WORKSPACE_SCHEMA_VERSION,
                workspace_id,
                source,
                values: BTreeMap::new(),
                timeline: Vec::new(),
                cursor: 0,
                modified_resources: BTreeMap::new(),
                deleted_resources: BTreeMap::new(),
                resource_revisions: Vec::new(),
                pending_revision: None,
                next_event_sequence: 1,
                migration_history: Vec::new(),
            },
        };
        workspace.persist()?;
        workspace.append_event("workspace_created", 0, 0, None)?;
        Ok(workspace)
    }

    pub fn open(root: impl Into<PathBuf>) -> AppResult<Self> {
        let root = root.into();
        let path = root.join("workspace.json");
        let bytes = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "open edit workspace",
                path.display().to_string(),
                &error,
            ))
        })?;
        let mut state = serde_json::from_slice::<PersistedWorkspace>(&bytes).map_err(|error| {
            edit_error(
                "EDIT_WORKSPACE_INVALID",
                format!("cannot decode {}: {error}", path.display()),
            )
        })?;
        if state.schema_version == 0 || state.schema_version > EDIT_WORKSPACE_SCHEMA_VERSION {
            return Err(edit_error(
                "EDIT_WORKSPACE_VERSION_UNSUPPORTED",
                format!(
                    "workspace schema {} is not supported; expected {EDIT_WORKSPACE_SCHEMA_VERSION}",
                    state.schema_version
                ),
            ));
        }
        if state.cursor > state.timeline.len() {
            return Err(edit_error(
                "EDIT_WORKSPACE_CURSOR_INVALID",
                "workspace cursor is outside the command timeline",
            ));
        }
        let previous_schema = state.schema_version;
        let migration = if previous_schema < EDIT_WORKSPACE_SCHEMA_VERSION {
            let backup_path = root.join(format!("workspace.json.v{previous_schema}.bak"));
            atomic_write(&backup_path, &bytes)?;
            let mut steps = Vec::new();
            if previous_schema < 2 {
                steps.push(
                    "initialisation des révisions de ressources et des suppressions atomiques"
                        .to_owned(),
                );
            }
            if previous_schema < 3 {
                steps.push(
                    "activation des baselines Toolset et de l’historique de migration".to_owned(),
                );
            }
            Some(WorkspaceMigrationRecord {
                from_version: previous_schema,
                to_version: EDIT_WORKSPACE_SCHEMA_VERSION,
                backup_path: backup_path.display().to_string(),
                steps,
            })
        } else {
            None
        };
        state.schema_version = EDIT_WORKSPACE_SCHEMA_VERSION;
        if let Some(migration) = migration.clone() {
            state.migration_history.push(migration);
        }
        let mut workspace = Self { root, state };
        if migration.is_some() {
            workspace.persist()?;
            workspace.append_event(
                "migrate_workspace",
                workspace.state.cursor,
                workspace.state.cursor,
                None,
            )?;
        }
        if workspace.state.pending_revision.is_some() {
            workspace.restore_pending_revision()?;
            workspace.persist()?;
            workspace.append_event(
                "recover_incomplete_transaction",
                workspace.state.cursor,
                workspace.state.cursor,
                None,
            )?;
        }
        Ok(workspace)
    }

    pub fn preview(&self, command: EditCommand) -> CommandPreview {
        Self::preview_against(&self.state.values, command)
    }

    fn preview_against(values: &BTreeMap<String, Value>, command: EditCommand) -> CommandPreview {
        let target = command.target();
        let (before, after) = command.values();
        let current = if matches!(command, EditCommand::TransformResource { .. }) {
            // Structural transforms carry their byte precondition. The staged revision is
            // verified against both hashes during apply, so unrelated field commands on the
            // same resource must not create a false logical-value conflict here.
            before.clone()
        } else {
            values
                .get(&target)
                .cloned()
                .unwrap_or_else(|| before.clone())
        };
        let valid = current == before && command.validate().is_ok();
        let diagnostic = if current != before {
            Some(
                "La valeur actuelle ne correspond pas à la précondition de la commande.".to_owned(),
            )
        } else {
            command.validate().err()
        };
        CommandPreview {
            command,
            target,
            current,
            resulting: after,
            valid,
            diagnostic,
        }
    }

    pub fn apply(&mut self, command: EditCommand) -> AppResult<WorkspaceSnapshot> {
        let preview = self.preview(command.clone());
        if !preview.valid {
            self.restore_pending_revision()?;
            self.persist()?;
            return Err(edit_error(
                "EDIT_PRECONDITION_FAILED",
                preview
                    .diagnostic
                    .unwrap_or_else(|| "command preview rejected the change".to_owned()),
            ));
        }
        let cursor_before = self.state.cursor;
        let revision = self.state.pending_revision.take();
        let expected_resources = command.affected_resources();
        let staged_resources = revision
            .as_ref()
            .map(ResourceRevision::resources)
            .unwrap_or_default();
        if expected_resources.is_empty() || staged_resources != expected_resources {
            self.state.pending_revision = revision;
            self.restore_pending_revision()?;
            self.persist()?;
            return Err(edit_error(
                "EDIT_RESOURCE_TRANSACTION_REQUIRED",
                "every edit command must be committed with exactly its staged resource bytes",
            ));
        }
        if let EditCommand::TransformResource {
            before_sha256,
            after_sha256,
            ..
        } = &command
        {
            let revision = revision.as_ref().expect("staged resources are non-empty");
            let staged_before = revision
                .before_modified
                .as_ref()
                .map(|modified| modified.output_sha256.as_str())
                .or_else(|| {
                    revision
                        .after_modified
                        .as_ref()
                        .and_then(|modified| modified.source_sha256.as_deref())
                });
            let staged_after = revision
                .after_modified
                .as_ref()
                .map(|modified| modified.output_sha256.as_str());
            if staged_before != Some(before_sha256.as_str())
                || staged_after != Some(after_sha256.as_str())
            {
                self.state.pending_revision = Some(revision.clone());
                self.restore_pending_revision()?;
                self.persist()?;
                return Err(edit_error(
                    "EDIT_RESOURCE_HASH_TRANSACTION_MISMATCH",
                    "structural transform hashes do not match the staged resource revision",
                ));
            }
        }
        // Preserve the redo branch until every fallible transaction check has
        // succeeded. A rejected command must never destroy valid history.
        self.state.timeline.truncate(self.state.cursor);
        self.state.resource_revisions.truncate(self.state.cursor);
        self.state
            .resource_revisions
            .resize(self.state.cursor, None);
        self.state.values.insert(preview.target, preview.resulting);
        self.state.timeline.push(command);
        self.state.resource_revisions.push(revision);
        self.state.cursor = self.state.timeline.len();
        self.persist()?;
        let event_command = self.state.timeline.last().cloned();
        self.append_event(
            "apply",
            cursor_before,
            self.state.cursor,
            event_command.as_ref(),
        )?;
        self.snapshot()
    }

    pub fn undo(&mut self) -> AppResult<WorkspaceSnapshot> {
        if self.state.cursor == 0 {
            return Err(edit_error(
                "EDIT_NOTHING_TO_UNDO",
                "the command cursor is zero",
            ));
        }
        let cursor_before = self.state.cursor;
        let command = self.state.timeline[self.state.cursor - 1].clone();
        let (before, after) = command.values();
        let target = command.target();
        let current = self.state.values.get(&target).cloned().unwrap_or(after);
        if current != command.values().1 {
            return Err(edit_error(
                "EDIT_UNDO_CONFLICT",
                format!("current value for {target} differs from the command result"),
            ));
        }
        if let Some(Some(revision)) = self
            .state
            .resource_revisions
            .get(self.state.cursor - 1)
            .cloned()
        {
            self.restore_revision_tree_before(&revision)?;
        }
        self.state.values.insert(target, before);
        self.state.cursor -= 1;
        self.persist()?;
        self.append_event("undo", cursor_before, self.state.cursor, Some(&command))?;
        self.snapshot()
    }

    pub fn redo(&mut self) -> AppResult<WorkspaceSnapshot> {
        if self.state.cursor >= self.state.timeline.len() {
            return Err(edit_error(
                "EDIT_NOTHING_TO_REDO",
                "the command cursor is at the end",
            ));
        }
        let cursor_before = self.state.cursor;
        let command = self.state.timeline[self.state.cursor].clone();
        let (before, after) = command.values();
        let target = command.target();
        let current = self.state.values.get(&target).cloned().unwrap_or(before);
        if current != command.values().0 {
            return Err(edit_error(
                "EDIT_REDO_CONFLICT",
                format!("current value for {target} differs from the command precondition"),
            ));
        }
        if let Some(Some(revision)) = self
            .state
            .resource_revisions
            .get(self.state.cursor)
            .cloned()
        {
            self.restore_revision_tree_after(&revision)?;
        }
        self.state.values.insert(target, after);
        self.state.cursor += 1;
        self.persist()?;
        self.append_event("redo", cursor_before, self.state.cursor, Some(&command))?;
        self.snapshot()
    }

    pub fn stage_resource(
        &mut self,
        resource: ResourceKey,
        source_bytes: Option<&[u8]>,
        output_bytes: &[u8],
    ) -> AppResult<ModifiedResource> {
        self.ensure_no_pending_transaction()?;
        let (revision, modified) =
            self.prepare_modified_revision(resource, source_bytes, output_bytes)?;
        self.state.pending_revision = Some(revision.clone());
        self.persist()?;
        self.restore_revision_after(&revision)?;
        self.persist()?;
        self.append_event("stage_resource", self.state.cursor, self.state.cursor, None)?;
        Ok(modified)
    }

    fn prepare_modified_revision(
        &self,
        resource: ResourceKey,
        source_bytes: Option<&[u8]>,
        output_bytes: &[u8],
    ) -> AppResult<(ResourceRevision, ModifiedResource)> {
        let key = resource.to_string();
        let before_modified = self.state.modified_resources.get(&key).cloned();
        let before_deleted = self.state.deleted_resources.get(&key).cloned();
        let before_blob = if before_modified.is_some() {
            let bytes = self
                .staged_resource_bytes(&resource)?
                .ok_or_else(|| edit_error("EDIT_STAGED_RESOURCE_MISSING", &key))?;
            Some(self.store_history_blob(&bytes)?)
        } else {
            None
        };
        let relative_path = format!("resources/{}", resource.file_name());
        let modified = ModifiedResource {
            resource: resource.clone(),
            source_sha256: source_bytes.map(sha256_bytes),
            output_sha256: sha256_bytes(output_bytes),
            size_bytes: output_bytes.len() as u64,
            relative_path: relative_path.replace('\\', "/"),
        };
        let after_blob = self.store_history_blob(output_bytes)?;
        Ok((
            ResourceRevision {
                resource,
                before_blob,
                before_modified,
                before_deleted,
                after_blob: Some(after_blob),
                after_modified: Some(modified.clone()),
                after_deleted: None,
                related: Vec::new(),
            },
            modified,
        ))
    }

    pub fn create_resource(
        &mut self,
        resource: ResourceKey,
        output_bytes: &[u8],
    ) -> AppResult<WorkspaceSnapshot> {
        if self
            .state
            .modified_resources
            .contains_key(&resource.to_string())
        {
            return Err(edit_error(
                "EDIT_RESOURCE_ALREADY_EXISTS",
                format!("{resource} already has a workspace state"),
            ));
        }
        let content_sha256 = sha256_bytes(output_bytes);
        self.stage_resource(resource.clone(), None, output_bytes)?;
        self.apply(EditCommand::CreateResource {
            resource,
            content_sha256,
        })
    }

    pub fn create_resources_atomic(
        &mut self,
        resources: &[ErfResourceInput],
    ) -> AppResult<WorkspaceSnapshot> {
        self.ensure_no_pending_transaction()?;
        if resources.is_empty() {
            return Err(edit_error(
                "EDIT_RESOURCE_SET_EMPTY",
                "an atomic resource set must contain at least one resource",
            ));
        }
        let mut keys = resources
            .iter()
            .map(|resource| resource.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(edit_error(
                "EDIT_RESOURCE_SET_DUPLICATE",
                "an atomic resource set cannot contain duplicate keys",
            ));
        }
        for key in &keys {
            if self.state.modified_resources.contains_key(&key.to_string())
                || self.state.deleted_resources.contains_key(&key.to_string())
            {
                return Err(edit_error(
                    "EDIT_RESOURCE_ALREADY_EXISTS",
                    format!("{key} already has a workspace state"),
                ));
            }
        }
        let mut revisions = Vec::with_capacity(resources.len());
        let mut digests = Vec::with_capacity(resources.len());
        for resource in resources {
            let (revision, _) =
                self.prepare_modified_revision(resource.key.clone(), None, &resource.bytes)?;
            revisions.push(revision);
            digests.push(ResourceContentDigest {
                resource: resource.key.clone(),
                content_sha256: sha256_bytes(&resource.bytes),
            });
        }
        revisions.sort_by(|left, right| left.resource.cmp(&right.resource));
        digests.sort_by(|left, right| left.resource.cmp(&right.resource));
        let mut revision = revisions.remove(0);
        revision.related = revisions;
        self.state.pending_revision = Some(revision.clone());
        self.persist()?;
        if let Err(error) = self.restore_revision_tree_after(&revision) {
            let _ = self.restore_pending_revision();
            let _ = self.persist();
            return Err(error);
        }
        self.persist()?;
        self.append_event(
            "stage_resource_set",
            self.state.cursor,
            self.state.cursor,
            None,
        )?;
        self.apply(EditCommand::CreateResourceSet { resources: digests })
    }

    pub fn delete_resource(
        &mut self,
        resource: ResourceKey,
        source_bytes: Option<&[u8]>,
    ) -> AppResult<WorkspaceSnapshot> {
        self.ensure_no_pending_transaction()?;
        let (revision, content_sha256) =
            self.prepare_deleted_revision(resource.clone(), source_bytes)?;
        self.state.pending_revision = Some(revision.clone());
        self.persist()?;
        self.restore_revision_after(&revision)?;
        self.persist()?;
        self.apply(EditCommand::DeleteResource {
            resource,
            content_sha256,
        })
    }

    fn prepare_deleted_revision(
        &self,
        resource: ResourceKey,
        source_bytes: Option<&[u8]>,
    ) -> AppResult<(ResourceRevision, String)> {
        let key = resource.to_string();
        let before_modified = self.state.modified_resources.get(&key).cloned();
        let before_deleted = self.state.deleted_resources.get(&key).cloned();
        if before_deleted.is_some() {
            return Err(edit_error(
                "EDIT_RESOURCE_ALREADY_DELETED",
                format!("{resource} is already deleted in the workspace"),
            ));
        }
        let current_bytes = if before_modified.is_some() {
            self.staged_resource_bytes(&resource)?
                .ok_or_else(|| edit_error("EDIT_STAGED_RESOURCE_MISSING", resource.to_string()))?
        } else {
            source_bytes
                .map(ToOwned::to_owned)
                .ok_or_else(|| edit_error("EDIT_DELETE_SOURCE_MISSING", resource.to_string()))?
        };
        let before_blob = if before_modified.is_some() {
            Some(self.store_history_blob(&current_bytes)?)
        } else {
            None
        };
        let content_sha256 = sha256_bytes(&current_bytes);
        Ok((
            ResourceRevision {
                resource: resource.clone(),
                before_blob,
                before_modified,
                before_deleted,
                after_blob: None,
                after_modified: None,
                after_deleted: Some(resource),
                related: Vec::new(),
            },
            content_sha256,
        ))
    }

    pub fn delete_resources_atomic(
        &mut self,
        resources: &[ErfResourceInput],
    ) -> AppResult<WorkspaceSnapshot> {
        self.ensure_no_pending_transaction()?;
        if resources.is_empty() {
            return Err(edit_error(
                "EDIT_RESOURCE_SET_EMPTY",
                "an atomic resource set must contain at least one resource",
            ));
        }
        let mut revisions = Vec::with_capacity(resources.len());
        let mut digests = Vec::with_capacity(resources.len());
        for resource in resources {
            let (revision, content_sha256) =
                self.prepare_deleted_revision(resource.key.clone(), Some(&resource.bytes))?;
            revisions.push(revision);
            digests.push(ResourceContentDigest {
                resource: resource.key.clone(),
                content_sha256,
            });
        }
        revisions.sort_by(|left, right| left.resource.cmp(&right.resource));
        if revisions
            .windows(2)
            .any(|pair| pair[0].resource == pair[1].resource)
        {
            return Err(edit_error(
                "EDIT_RESOURCE_SET_DUPLICATE",
                "an atomic resource set cannot contain duplicate keys",
            ));
        }
        digests.sort_by(|left, right| left.resource.cmp(&right.resource));
        let mut revision = revisions.remove(0);
        revision.related = revisions;
        self.state.pending_revision = Some(revision.clone());
        self.persist()?;
        if let Err(error) = self.restore_revision_tree_after(&revision) {
            let _ = self.restore_pending_revision();
            let _ = self.persist();
            return Err(error);
        }
        self.persist()?;
        self.append_event(
            "stage_resource_deletion_set",
            self.state.cursor,
            self.state.cursor,
            None,
        )?;
        self.apply(EditCommand::DeleteResourceSet { resources: digests })
    }

    fn ensure_no_pending_transaction(&self) -> AppResult<()> {
        if self.state.pending_revision.is_some() {
            return Err(edit_error(
                "EDIT_TRANSACTION_ALREADY_PENDING",
                "an edit transaction is already staged and must be committed or recovered",
            ));
        }
        Ok(())
    }

    pub fn staged_resource_bytes(&self, resource: &ResourceKey) -> AppResult<Option<Vec<u8>>> {
        let Some(modified) = self.state.modified_resources.get(&resource.to_string()) else {
            return Ok(None);
        };
        let path = self.root.join(&modified.relative_path);
        fs::read(&path).map(Some).map_err(|error| {
            Box::new(AppError::io(
                "read staged resource",
                path.display().to_string(),
                &error,
            ))
        })
    }

    fn staged_resource_path(&self, resource: &ResourceKey) -> AppResult<Option<PathBuf>> {
        let Some(modified) = self.state.modified_resources.get(&resource.to_string()) else {
            return Ok(None);
        };
        let path = self.root.join(&modified.relative_path);
        if !path.is_file() {
            return Err(edit_error(
                "EDIT_STAGED_RESOURCE_MISSING",
                format!("{} is recorded but {} is absent", resource, path.display()),
            ));
        }
        Ok(Some(path))
    }

    pub fn snapshot(&self) -> AppResult<WorkspaceSnapshot> {
        let source_intact = verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        )
        .is_ok();
        Ok(WorkspaceSnapshot {
            schema_version: self.state.schema_version,
            workspace_id: self.state.workspace_id.clone(),
            root: self.root.display().to_string(),
            source: self.state.source.clone(),
            source_intact,
            command_count: self.state.timeline.len(),
            cursor: self.state.cursor,
            can_undo: self.state.cursor > 0,
            can_redo: self.state.cursor < self.state.timeline.len(),
            modified_resources: self.state.modified_resources.values().cloned().collect(),
            deleted_resources: self.state.deleted_resources.values().cloned().collect(),
            journal_events: self.state.next_event_sequence.saturating_sub(1),
            values: self.state.values.clone(),
            migration_history: self.state.migration_history.clone(),
        })
    }

    pub fn modified_resources(&self) -> Vec<ModifiedResource> {
        self.state.modified_resources.values().cloned().collect()
    }

    fn store_history_blob(&self, bytes: &[u8]) -> AppResult<String> {
        let sha256 = sha256_bytes(bytes);
        let relative = format!("history/blobs/{sha256}.bin");
        let path = self.root.join(&relative);
        if !path.is_file() {
            atomic_write(&path, bytes)?;
        }
        Ok(relative)
    }

    fn restore_pending_revision(&mut self) -> AppResult<()> {
        if let Some(revision) = self.state.pending_revision.take() {
            self.restore_revision_tree_before(&revision)?;
        }
        Ok(())
    }

    fn restore_revision_tree_before(&mut self, revision: &ResourceRevision) -> AppResult<()> {
        for related in revision.related.iter().rev() {
            self.restore_revision_before(related)?;
        }
        self.restore_revision_before(revision)
    }

    fn restore_revision_tree_after(&mut self, revision: &ResourceRevision) -> AppResult<()> {
        self.restore_revision_after(revision)?;
        for related in &revision.related {
            self.restore_revision_after(related)?;
        }
        Ok(())
    }

    fn restore_revision_before(&mut self, revision: &ResourceRevision) -> AppResult<()> {
        let key = revision.resource.to_string();
        self.state.deleted_resources.remove(&key);
        if let (Some(blob), Some(modified)) = (&revision.before_blob, &revision.before_modified) {
            let bytes = fs::read(self.root.join(blob))
                .map_err(|error| Box::new(AppError::io("read edit history blob", blob, &error)))?;
            atomic_write(&self.root.join(&modified.relative_path), &bytes)?;
            self.state
                .modified_resources
                .insert(key.clone(), modified.clone());
        } else {
            if let Some(after_modified) = &revision.after_modified {
                let path = self.root.join(&after_modified.relative_path);
                if path.is_file() {
                    fs::remove_file(&path).map_err(|error| {
                        Box::new(AppError::io(
                            "remove reverted staged resource",
                            path.display().to_string(),
                            &error,
                        ))
                    })?;
                }
            }
            self.state.modified_resources.remove(&key);
        }
        if let Some(deleted) = &revision.before_deleted {
            self.state.deleted_resources.insert(key, deleted.clone());
        }
        Ok(())
    }

    fn restore_revision_after(&mut self, revision: &ResourceRevision) -> AppResult<()> {
        let key = revision.resource.to_string();
        self.state.deleted_resources.remove(&key);
        if let (Some(blob), Some(modified)) = (&revision.after_blob, &revision.after_modified) {
            let bytes = fs::read(self.root.join(blob))
                .map_err(|error| Box::new(AppError::io("read edit history blob", blob, &error)))?;
            atomic_write(&self.root.join(&modified.relative_path), &bytes)?;
            self.state
                .modified_resources
                .insert(key.clone(), modified.clone());
        } else {
            if let Some(before_modified) = &revision.before_modified {
                let path = self.root.join(&before_modified.relative_path);
                if path.is_file() {
                    fs::remove_file(&path).map_err(|error| {
                        Box::new(AppError::io(
                            "remove deleted staged resource",
                            path.display().to_string(),
                            &error,
                        ))
                    })?;
                }
            }
            self.state.modified_resources.remove(&key);
        }
        if let Some(deleted) = &revision.after_deleted {
            self.state.deleted_resources.insert(key, deleted.clone());
        }
        Ok(())
    }

    pub fn build_module(&self, output_path: &Path) -> AppResult<ModuleBuildReport> {
        verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        )?;
        ensure_output_is_not_source(output_path, Path::new(&self.state.source.path))?;
        if !output_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("mod"))
        {
            return Err(edit_error(
                "EDIT_BUILD_EXTENSION_INVALID",
                "module build output must use the .mod extension",
            ));
        }
        self.validate_compiled_scripts()?;
        let reader = ErfReader::default();
        let cancelled = AtomicBool::new(false);
        let source_path = Path::new(&self.state.source.path);
        let inventory = reader.read_inventory(source_path, &cancelled)?;
        let archive_metadata = reader.read_archive_metadata(source_path)?;
        let mut resources = Vec::with_capacity(
            inventory
                .resources
                .len()
                .saturating_add(self.state.modified_resources.len()),
        );
        let mut seen = std::collections::BTreeSet::new();
        for resource in &inventory.resources {
            if self
                .state
                .deleted_resources
                .contains_key(&resource.key.to_string())
            {
                continue;
            }
            let source = match self.staged_resource_path(&resource.key)? {
                Some(path) => ErfResourceSource::File(path),
                None => ErfResourceSource::Range {
                    path: source_path.to_path_buf(),
                    offset: resource.offset,
                    size: resource.size,
                },
            };
            seen.insert(resource.key.clone());
            resources.push(ErfResourceStreamInput {
                key: resource.key.clone(),
                source,
            });
        }
        for modified in self.state.modified_resources.values() {
            if seen.contains(&modified.resource) {
                continue;
            }
            let path = self
                .staged_resource_path(&modified.resource)?
                .ok_or_else(|| {
                    edit_error(
                        "EDIT_STAGED_RESOURCE_MISSING",
                        format!("{} is recorded but absent", modified.resource),
                    )
                })?;
            resources.push(ErfResourceStreamInput {
                key: modified.resource.clone(),
                source: ErfResourceSource::File(path),
            });
        }
        write_erf_streaming_with_metadata(output_path, "MOD ", &resources, &archive_metadata)?;
        if let Err(error) = verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        ) {
            let _ = fs::remove_file(output_path);
            return Err(error);
        }
        let reopened = reader.read_inventory(output_path, &cancelled)?;
        if reopened.resource_count as usize != resources.len() {
            return Err(edit_error(
                "EDIT_BUILD_REOPEN_FAILED",
                format!(
                    "rebuilt MOD exposes {} resources, expected {}",
                    reopened.resource_count,
                    resources.len()
                ),
            ));
        }
        Ok(ModuleBuildReport {
            output_path: output_path.display().to_string(),
            sha256: sha256_file(output_path)?,
            size_bytes: fs::metadata(output_path)
                .map_err(|error| {
                    Box::new(AppError::io(
                        "inspect rebuilt module",
                        output_path.display().to_string(),
                        &error,
                    ))
                })?
                .len(),
            resource_count: resources.len(),
            modified_resources: self.state.modified_resources.len(),
            deleted_resources: self.state.deleted_resources.len(),
            source_intact: true,
        })
    }

    pub fn build_hak(&self, output_path: &Path) -> AppResult<ModuleBuildReport> {
        verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        )?;
        ensure_output_is_not_source(output_path, Path::new(&self.state.source.path))?;
        self.validate_compiled_scripts()?;
        if !output_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("hak"))
        {
            return Err(edit_error(
                "EDIT_HAK_EXTENSION_INVALID",
                "custom content output must use the .hak extension",
            ));
        }
        let mut resources = Vec::with_capacity(self.state.modified_resources.len());
        for modified in self.state.modified_resources.values() {
            let path = self
                .staged_resource_path(&modified.resource)?
                .ok_or_else(|| {
                    edit_error(
                        "EDIT_STAGED_RESOURCE_MISSING",
                        modified.resource.to_string(),
                    )
                })?;
            resources.push(ErfResourceStreamInput {
                key: modified.resource.clone(),
                source: ErfResourceSource::File(path),
            });
        }
        if resources.is_empty() {
            return Err(edit_error(
                "EDIT_HAK_EMPTY",
                "at least one modified resource is required to build a HAK",
            ));
        }
        write_erf_streaming(output_path, "HAK ", &resources)?;
        if let Err(error) = verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        ) {
            let _ = fs::remove_file(output_path);
            return Err(error);
        }
        let inventory =
            ErfReader::default().read_inventory(output_path, &AtomicBool::new(false))?;
        Ok(ModuleBuildReport {
            output_path: output_path.display().to_string(),
            sha256: sha256_file(output_path)?,
            size_bytes: fs::metadata(output_path)
                .map_err(|error| {
                    Box::new(AppError::io(
                        "inspect rebuilt HAK",
                        output_path.display().to_string(),
                        &error,
                    ))
                })?
                .len(),
            resource_count: inventory.resource_count as usize,
            modified_resources: resources.len(),
            deleted_resources: 0,
            source_intact: true,
        })
    }

    pub fn deploy_development(&self, user_data_path: &Path) -> AppResult<DevelopmentDeployment> {
        if !user_data_path.is_dir() {
            return Err(edit_error(
                "EDIT_DEVELOPMENT_ROOT_INVALID",
                format!("{} is not a directory", user_data_path.display()),
            ));
        }
        self.validate_compiled_scripts()?;
        let development = user_data_path.join("development");
        fs::create_dir_all(&development).map_err(|error| {
            Box::new(AppError::io(
                "create NWN development directory",
                development.display().to_string(),
                &error,
            ))
        })?;
        let mut files = Vec::new();
        for modified in self.state.modified_resources.values() {
            let name = modified.resource.file_name();
            files.push(DevelopmentFile {
                name,
                sha256: modified.output_sha256.clone(),
                size_bytes: modified.size_bytes,
            });
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        self.prepare_development_deployment(&development, &files)?;
        for modified in self.state.modified_resources.values() {
            let source = self
                .staged_resource_path(&modified.resource)?
                .ok_or_else(|| {
                    edit_error(
                        "EDIT_STAGED_RESOURCE_MISSING",
                        modified.resource.to_string(),
                    )
                })?;
            atomic_copy(&source, &development.join(modified.resource.file_name()))?;
        }
        let deployment = DevelopmentDeployment {
            workspace_id: self.state.workspace_id.clone(),
            development_path: development.display().to_string(),
            files,
        };
        let manifest = serde_json::to_vec_pretty(&deployment).map_err(|error| {
            edit_error(
                "EDIT_DEPLOYMENT_SERIALIZE_FAILED",
                format!("cannot serialize development deployment: {error}"),
            )
        })?;
        atomic_write(&self.deployment_manifest_path(&development), &manifest)?;
        Ok(deployment)
    }

    fn prepare_development_deployment(
        &self,
        development: &Path,
        new_files: &[DevelopmentFile],
    ) -> AppResult<()> {
        let new_names = new_files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for entry in fs::read_dir(development).map_err(|error| {
            Box::new(AppError::io(
                "scan development deployment manifests",
                development.display().to_string(),
                &error,
            ))
        })? {
            let entry = entry.map_err(|error| {
                Box::new(AppError::io(
                    "read development deployment manifest entry",
                    development.display().to_string(),
                    &error,
                ))
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(".opennever-deployment-") || !name.ends_with(".json") {
                continue;
            }
            let bytes = fs::read(entry.path()).map_err(|error| {
                Box::new(AppError::io(
                    "read development deployment manifest",
                    entry.path().display().to_string(),
                    &error,
                ))
            })?;
            let manifest =
                serde_json::from_slice::<DevelopmentDeployment>(&bytes).map_err(|error| {
                    edit_error(
                        "EDIT_DEPLOYMENT_MANIFEST_INVALID",
                        format!("cannot decode {}: {error}", entry.path().display()),
                    )
                })?;
            if manifest.workspace_id != self.state.workspace_id {
                if let Some(conflict) = manifest
                    .files
                    .iter()
                    .find(|file| new_names.contains(file.name.as_str()))
                {
                    return Err(edit_error(
                        "EDIT_DEVELOPMENT_OWNERSHIP_CONFLICT",
                        format!(
                            "{} is already owned by workspace {}",
                            conflict.name, manifest.workspace_id
                        ),
                    ));
                }
                continue;
            }
            for old in manifest
                .files
                .iter()
                .filter(|file| !new_names.contains(file.name.as_str()))
            {
                let path = development.join(&old.name);
                if !path.is_file() {
                    continue;
                }
                if sha256_file(&path)? != old.sha256 {
                    return Err(edit_error(
                        "EDIT_DEVELOPMENT_CHANGED_FILE_CONFLICT",
                        format!("{} changed after the previous deployment", old.name),
                    ));
                }
                fs::remove_file(&path).map_err(|error| {
                    Box::new(AppError::io(
                        "remove obsolete development deployment file",
                        path.display().to_string(),
                        &error,
                    ))
                })?;
            }
        }
        Ok(())
    }

    pub fn clean_development(&self, user_data_path: &Path) -> AppResult<DevelopmentCleanupReport> {
        let development = user_data_path.join("development");
        let manifest_path = self.deployment_manifest_path(&development);
        let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
            Box::new(AppError::io(
                "read development deployment manifest",
                manifest_path.display().to_string(),
                &error,
            ))
        })?;
        let deployment =
            serde_json::from_slice::<DevelopmentDeployment>(&manifest_bytes).map_err(|error| {
                edit_error(
                    "EDIT_DEPLOYMENT_MANIFEST_INVALID",
                    format!("cannot decode {}: {error}", manifest_path.display()),
                )
            })?;
        if deployment.workspace_id != self.state.workspace_id {
            return Err(edit_error(
                "EDIT_DEPLOYMENT_WORKSPACE_MISMATCH",
                "deployment manifest belongs to another workspace",
            ));
        }
        let mut removed = Vec::new();
        let mut preserved_changed = Vec::new();
        for file in deployment.files {
            let path = development.join(&file.name);
            if !path.is_file() {
                continue;
            }
            let current = sha256_file(&path)?;
            if current == file.sha256 {
                fs::remove_file(&path).map_err(|error| {
                    Box::new(AppError::io(
                        "remove deployed development file",
                        path.display().to_string(),
                        &error,
                    ))
                })?;
                removed.push(file.name);
            } else {
                preserved_changed.push(file.name);
            }
        }
        fs::remove_file(&manifest_path).map_err(|error| {
            Box::new(AppError::io(
                "remove development deployment manifest",
                manifest_path.display().to_string(),
                &error,
            ))
        })?;
        Ok(DevelopmentCleanupReport {
            removed,
            preserved_changed,
        })
    }

    pub fn validate_compiled_scripts(&self) -> AppResult<()> {
        for modified in self.state.modified_resources.values() {
            if modified.resource.resource_type != 2009 {
                continue;
            }
            let ncs = ResourceKey::new(&modified.resource.resref, 2010);
            let current_nss_sha256 = modified.output_sha256.as_str();
            let compilation = self.state.timeline[..self.state.cursor]
                .iter()
                .rev()
                .find_map(|command| match command {
                    EditCommand::CompileScript {
                        resource,
                        inputs,
                        compiler_sha256,
                        after_sha256,
                        ..
                    } if resource == &ncs => Some((inputs, compiler_sha256, after_sha256)),
                    _ => None,
                });
            let valid = compilation.is_some_and(|(inputs, compiler_sha256, after_sha256)| {
                is_sha256(compiler_sha256)
                    && self
                        .state
                        .modified_resources
                        .get(&ncs.to_string())
                        .is_some_and(|compiled| compiled.output_sha256 == *after_sha256)
                    && inputs.iter().any(|input| {
                        input.resource == modified.resource
                            && input.content_sha256 == current_nss_sha256
                    })
                    && inputs.iter().all(|input| {
                        self.state
                            .modified_resources
                            .get(&input.resource.to_string())
                            .is_none_or(|current| current.output_sha256 == input.content_sha256)
                    })
            });
            if !valid {
                return Err(edit_error(
                    "EDIT_NSS_COMPILATION_STALE",
                    format!(
                        "{} was modified after the last exact compilation of {}",
                        modified.resource, ncs
                    ),
                ));
            }
        }
        Ok(())
    }

    fn deployment_manifest_path(&self, development: &Path) -> PathBuf {
        development.join(format!(
            ".opennever-deployment-{}.json",
            self.state.workspace_id
        ))
    }

    pub fn id(&self) -> &str {
        &self.state.workspace_id
    }

    pub fn preview_ai_change_set(&self, change_set: &AiChangeSet) -> AiChangeSetPreview {
        let mut values = self.state.values.clone();
        let mut previews = Vec::with_capacity(change_set.commands.len());
        for command in change_set.commands.iter().cloned() {
            let preview = Self::preview_against(&values, command);
            if preview.valid {
                values.insert(preview.target.clone(), preview.resulting.clone());
            }
            previews.push(preview);
        }
        AiChangeSetPreview {
            summary: change_set.summary.clone(),
            proposal_sha256: hex::encode(Sha256::digest(
                serde_json::to_vec(change_set).expect("AI change set serializes"),
            )),
            all_valid: !previews.is_empty() && previews.iter().all(|preview| preview.valid),
            previews,
        }
    }

    pub fn preview_controlled_ai_change_set(
        &self,
        change_set: &AiChangeSet,
        source_resources: &BTreeMap<String, Vec<u8>>,
    ) -> AppResult<AiChangeSetPreview> {
        validate_controlled_ai_change_set(change_set)?;
        let mut preview = self.preview_ai_change_set(change_set);
        let mut resource_states = BTreeMap::<String, Vec<u8>>::new();

        for (index, command) in change_set.commands.iter().enumerate() {
            if !preview.previews[index].valid {
                continue;
            }
            let resource = controlled_ai_resource(command);
            let key = resource.to_string();
            if !resource_states.contains_key(&key) {
                let bytes = self
                    .staged_resource_bytes(resource)?
                    .or_else(|| source_resources.get(&key).cloned())
                    .ok_or_else(|| {
                        edit_error(
                            "EDIT_AI_RESOURCE_MISSING",
                            format!("no source bytes are available for {resource}"),
                        )
                    })?;
                resource_states.insert(key.clone(), bytes);
            }
            let current = resource_states
                .get(&key)
                .expect("controlled resource state was inserted")
                .clone();
            let transformed = match command {
                EditCommand::SetField {
                    path,
                    before,
                    after,
                    ..
                } => edit_gff_field(
                    &current,
                    &format!("ai-preview::{}", resource.file_name()),
                    path,
                    before,
                    after,
                )
                .map(|(bytes, _)| bytes),
                EditCommand::ReplaceText { before, after, .. } => {
                    let current_text = String::from_utf8_lossy(&current).into_owned();
                    if current_text != *before {
                        Err(edit_error(
                            "EDIT_AI_TEXT_PRECONDITION_FAILED",
                            format!("current text for {resource} differs from the proposal"),
                        ))
                    } else {
                        parse_nss(after.as_bytes(), &format!("ai-preview::{resource}"))?;
                        Ok(after.as_bytes().to_vec())
                    }
                }
                _ => unreachable!("controlled AI validation rejects unsupported commands"),
            };
            match transformed {
                Ok(bytes) => {
                    resource_states.insert(key, bytes);
                }
                Err(error) => {
                    preview.previews[index].valid = false;
                    preview.previews[index].diagnostic = Some(error.user_message.clone());
                }
            }
        }
        preview.all_valid =
            !preview.previews.is_empty() && preview.previews.iter().all(|command| command.valid);
        Ok(preview)
    }

    pub fn apply_controlled_ai_change_set(
        &mut self,
        change_set: &AiChangeSet,
        expected_proposal_sha256: &str,
        source_resources: &BTreeMap<String, Vec<u8>>,
    ) -> AppResult<AiApplyReport> {
        let proposal_sha256 = ai_change_set_sha256(change_set)?;
        if proposal_sha256 != expected_proposal_sha256 {
            return Err(edit_error(
                "EDIT_AI_PROPOSAL_CHANGED",
                "the confirmed AI proposal does not match its preview digest",
            ));
        }
        let preview = self.preview_controlled_ai_change_set(change_set, source_resources)?;
        if !preview.all_valid {
            return Err(edit_error(
                "EDIT_AI_PREVIEW_REJECTED",
                "at least one AI operation failed its current byte or schema precondition",
            ));
        }

        let cursor_before = self.state.cursor;
        for command in change_set.commands.iter().cloned() {
            let resource = controlled_ai_resource(&command).clone();
            let source_bytes = source_resources.get(&resource.to_string()).ok_or_else(|| {
                edit_error(
                    "EDIT_AI_RESOURCE_MISSING",
                    format!("no immutable source bytes are available for {resource}"),
                )
            })?;
            let current = self
                .staged_resource_bytes(&resource)?
                .unwrap_or_else(|| source_bytes.clone());
            let output = match &command {
                EditCommand::SetField {
                    path,
                    before,
                    after,
                    ..
                } => edit_gff_field(
                    &current,
                    &format!("ai-apply::{}", resource.file_name()),
                    path,
                    before,
                    after,
                )
                .map(|(bytes, _)| bytes),
                EditCommand::ReplaceText { before, after, .. } => {
                    let current_text = String::from_utf8_lossy(&current).into_owned();
                    if current_text != *before {
                        Err(edit_error(
                            "EDIT_AI_TEXT_PRECONDITION_FAILED",
                            format!("current text for {resource} differs from the proposal"),
                        ))
                    } else {
                        parse_nss(after.as_bytes(), &format!("ai-apply::{resource}"))?;
                        Ok(after.as_bytes().to_vec())
                    }
                }
                _ => unreachable!("controlled AI validation rejects unsupported commands"),
            };
            let result = output.and_then(|bytes| {
                self.stage_resource(resource, Some(source_bytes), &bytes)?;
                self.apply(command)
            });
            if let Err(error) = result {
                self.rollback_ai_batch(cursor_before)?;
                return Err(error);
            }
        }

        Ok(AiApplyReport {
            proposal_sha256,
            applied_commands: change_set.commands.len(),
            workspace: self.snapshot()?,
        })
    }

    fn rollback_ai_batch(&mut self, cursor: usize) -> AppResult<()> {
        while self.state.cursor > cursor {
            self.undo()?;
        }
        self.state.timeline.truncate(cursor);
        self.state.resource_revisions.truncate(cursor);
        self.persist()?;
        self.append_event("rollback_ai_change_set", cursor, cursor, None)
    }

    pub fn export_reproducible_sources(
        &self,
        destination: &Path,
    ) -> AppResult<WorkspaceExportManifest> {
        verify_source(
            Path::new(&self.state.source.path),
            &self.state.source.sha256,
            self.state.source.size_bytes,
        )?;
        if destination.as_os_str().is_empty() || destination == Path::new(&self.state.source.path) {
            return Err(edit_error(
                "EDIT_EXPORT_PATH_INVALID",
                "export destination must be separate from the source module",
            ));
        }
        let resource_root = destination.join("resources");
        fs::create_dir_all(&resource_root).map_err(|error| {
            Box::new(AppError::io(
                "create reproducible export directory",
                resource_root.display().to_string(),
                &error,
            ))
        })?;
        let mut files = Vec::new();
        for modified in self.state.modified_resources.values() {
            let bytes = self
                .staged_resource_bytes(&modified.resource)?
                .ok_or_else(|| {
                    edit_error(
                        "EDIT_STAGED_RESOURCE_MISSING",
                        modified.resource.to_string(),
                    )
                })?;
            let name = modified.resource.file_name();
            atomic_write(&resource_root.join(&name), &bytes)?;
            files.push(DevelopmentFile {
                name,
                sha256: sha256_bytes(&bytes),
                size_bytes: bytes.len() as u64,
            });
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        let manifest = WorkspaceExportManifest {
            schema_version: 1,
            workspace_id: self.state.workspace_id.clone(),
            source_sha256: self.state.source.sha256.clone(),
            files,
            deleted_resources: self.state.deleted_resources.values().cloned().collect(),
        };
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| edit_error("EDIT_EXPORT_SERIALIZE_FAILED", error.to_string()))?;
        atomic_write(&destination.join("opennever-export.json"), &bytes)?;
        Ok(manifest)
    }

    pub fn load_aurora_sync_baseline(
        &self,
        toolset_root: &Path,
    ) -> AppResult<Option<AuroraSyncBaseline>> {
        let path = self.aurora_sync_baseline_path(toolset_root)?;
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "read Aurora synchronization baseline",
                path.display().to_string(),
                &error,
            ))
        })?;
        let mut baseline =
            serde_json::from_slice::<AuroraSyncBaseline>(&bytes).map_err(|error| {
                edit_error(
                    "EDIT_AURORA_SYNC_BASELINE_INVALID",
                    format!("cannot decode {}: {error}", path.display()),
                )
            })?;
        if baseline.schema_version == 0 || baseline.schema_version > AURORA_SYNC_SCHEMA_VERSION {
            return Err(edit_error(
                "EDIT_AURORA_SYNC_BASELINE_VERSION_UNSUPPORTED",
                format!(
                    "baseline schema {} is not supported",
                    baseline.schema_version
                ),
            ));
        }
        baseline.schema_version = AURORA_SYNC_SCHEMA_VERSION;
        Ok(Some(baseline))
    }

    pub fn save_aurora_sync_baseline(
        &self,
        toolset_root: &Path,
        baseline: &AuroraSyncBaseline,
    ) -> AppResult<String> {
        let path = self.aurora_sync_baseline_path(toolset_root)?;
        let mut normalized = baseline.clone();
        normalized.schema_version = AURORA_SYNC_SCHEMA_VERSION;
        normalized.root = canonical_toolset_root(toolset_root)?;
        normalized
            .entries
            .sort_by(|left, right| left.resource.cmp(&right.resource));
        let bytes = serde_json::to_vec_pretty(&normalized).map_err(|error| {
            edit_error(
                "EDIT_AURORA_SYNC_BASELINE_SERIALIZE_FAILED",
                error.to_string(),
            )
        })?;
        atomic_write(&path, &bytes)?;
        Ok(path.display().to_string())
    }

    fn aurora_sync_baseline_path(&self, toolset_root: &Path) -> AppResult<PathBuf> {
        let root = canonical_toolset_root(toolset_root)?;
        let identity = sha256_bytes(root.to_ascii_lowercase().as_bytes());
        Ok(self
            .root
            .join("aurora-sync")
            .join(format!("{identity}.json")))
    }

    pub fn list_build_profiles(&self) -> AppResult<Vec<ModuleBuildProfile>> {
        let path = self.root.join("build-profiles.json");
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "read build profiles",
                path.display().to_string(),
                &error,
            ))
        })?;
        let mut profiles =
            serde_json::from_slice::<Vec<ModuleBuildProfile>>(&bytes).map_err(|error| {
                edit_error(
                    "EDIT_BUILD_PROFILES_INVALID",
                    format!("cannot decode {}: {error}", path.display()),
                )
            })?;
        for profile in &profiles {
            validate_build_profile(profile)?;
        }
        profiles.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
        Ok(profiles)
    }

    pub fn save_build_profile(
        &self,
        profile: ModuleBuildProfile,
    ) -> AppResult<Vec<ModuleBuildProfile>> {
        validate_build_profile(&profile)?;
        let mut profiles = self.list_build_profiles()?;
        profiles.retain(|candidate| !candidate.name.eq_ignore_ascii_case(&profile.name));
        profiles.push(profile);
        profiles.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
        let bytes = serde_json::to_vec_pretty(&profiles).map_err(|error| {
            edit_error("EDIT_BUILD_PROFILES_SERIALIZE_FAILED", error.to_string())
        })?;
        atomic_write(&self.root.join("build-profiles.json"), &bytes)?;
        Ok(profiles)
    }

    pub fn verify_reproducible_build(
        &self,
        profile: &ModuleBuildProfile,
    ) -> AppResult<ReproducibleBuildVerification> {
        let warnings = self.validate_build_profile_context(profile)?;
        let temp = tempfile::tempdir().map_err(|error| {
            Box::new(AppError::io(
                "create build verification directory",
                "temporary",
                &error,
            ))
        })?;
        let first = temp.path().join("first.mod");
        let second = temp.path().join("second.mod");
        let first_report = self.build_module(&first)?;
        let second_report = self.build_module(&second)?;
        Ok(ReproducibleBuildVerification {
            profile: profile.clone(),
            first_sha256: first_report.sha256.clone(),
            second_sha256: second_report.sha256.clone(),
            identical: first_report.sha256 == second_report.sha256,
            resource_count: first_report.resource_count,
            warnings,
        })
    }

    pub fn validate_build_profile_context(
        &self,
        profile: &ModuleBuildProfile,
    ) -> AppResult<Vec<String>> {
        validate_build_profile(profile)?;
        let key = ResourceKey::new("module", 2014);
        let bytes = if let Some(bytes) = self.staged_resource_bytes(&key)? {
            bytes
        } else {
            let source = Path::new(&self.state.source.path);
            let reader = ErfReader::default();
            let inventory = reader.read_inventory(source, &AtomicBool::new(false))?;
            let resource = inventory
                .resources
                .iter()
                .find(|resource| resource.key == key)
                .ok_or_else(|| {
                    edit_error(
                        "EDIT_MODULE_INFO_MISSING",
                        "source module has no module.ifo",
                    )
                })?;
            reader.read_resource(source, resource, &AtomicBool::new(false))?
        };
        let info = read_module_info(&bytes, "profile::module.ifo")?;
        let mut warnings = Vec::new();
        if info.hak_files != profile.hak_files {
            warnings.push(format!(
                "profile HAK list {:?} differs from module.ifo {:?}",
                profile.hak_files, info.hak_files
            ));
        }
        if info.custom_tlk != profile.custom_tlk {
            warnings.push(format!(
                "profile custom TLK {:?} differs from module.ifo {:?}",
                profile.custom_tlk, info.custom_tlk
            ));
        }
        if profile.block_on_warnings && !warnings.is_empty() {
            return Err(edit_error(
                "EDIT_BUILD_PROFILE_WARNINGS_BLOCKED",
                warnings.join("; "),
            ));
        }
        Ok(warnings)
    }

    pub fn list_launch_profiles(&self) -> AppResult<Vec<NwnLaunchProfile>> {
        let path = self.root.join("launch-profiles.json");
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "read launch profiles",
                path.display().to_string(),
                &error,
            ))
        })?;
        let mut profiles = serde_json::from_slice::<Vec<NwnLaunchProfile>>(&bytes)
            .map_err(|error| edit_error("EDIT_LAUNCH_PROFILES_INVALID", error.to_string()))?;
        for profile in &profiles {
            validate_launch_profile(profile)?;
        }
        profiles.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
        Ok(profiles)
    }

    pub fn save_launch_profile(
        &self,
        profile: NwnLaunchProfile,
    ) -> AppResult<Vec<NwnLaunchProfile>> {
        validate_launch_profile(&profile)?;
        let mut profiles = self.list_launch_profiles()?;
        profiles.retain(|candidate| !candidate.name.eq_ignore_ascii_case(&profile.name));
        profiles.push(profile);
        profiles.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
        let bytes = serde_json::to_vec_pretty(&profiles).map_err(|error| {
            edit_error("EDIT_LAUNCH_PROFILES_SERIALIZE_FAILED", error.to_string())
        })?;
        atomic_write(&self.root.join("launch-profiles.json"), &bytes)?;
        Ok(profiles)
    }

    pub fn launch_nwn_profile(&self, profile: &NwnLaunchProfile) -> AppResult<NwnLaunchReport> {
        validate_launch_profile(profile)?;
        let log_root = self.root.join("launch-logs");
        fs::create_dir_all(&log_root).map_err(|error| {
            Box::new(AppError::io(
                "create launch log directory",
                log_root.display().to_string(),
                &error,
            ))
        })?;
        let log_path = log_root.join(format!("{}.log", safe_profile_name(&profile.name)));
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|error| {
                Box::new(AppError::io(
                    "open launch log",
                    log_path.display().to_string(),
                    &error,
                ))
            })?;
        let stderr = log.try_clone().map_err(|error| {
            Box::new(AppError::io(
                "clone launch log",
                log_path.display().to_string(),
                &error,
            ))
        })?;
        let child = Command::new(&profile.executable_path)
            .current_dir(&profile.working_directory)
            .args(&profile.arguments)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| {
                Box::new(AppError::io(
                    "launch NWN test profile",
                    profile.executable_path.clone(),
                    &error,
                ))
            })?;
        Ok(NwnLaunchReport {
            profile: profile.clone(),
            process_id: child.id(),
            log_path: log_path.display().to_string(),
        })
    }

    fn persist(&self) -> AppResult<()> {
        let bytes = serde_json::to_vec_pretty(&self.state).map_err(|error| {
            edit_error(
                "EDIT_WORKSPACE_SERIALIZE_FAILED",
                format!("cannot serialize workspace state: {error}"),
            )
        })?;
        atomic_write(&self.root.join("workspace.json"), &bytes)
    }

    fn append_event(
        &mut self,
        action: &str,
        cursor_before: usize,
        cursor_after: usize,
        command: Option<&EditCommand>,
    ) -> AppResult<()> {
        let event = JournalEvent {
            sequence: self.state.next_event_sequence,
            action,
            cursor_before,
            cursor_after,
            command,
        };
        let mut bytes = serde_json::to_vec(&event).map_err(|error| {
            edit_error(
                "EDIT_JOURNAL_SERIALIZE_FAILED",
                format!("cannot serialize journal event: {error}"),
            )
        })?;
        bytes.push(b'\n');
        let path = self.root.join("journal.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                Box::new(AppError::io(
                    "open edit journal",
                    path.display().to_string(),
                    &error,
                ))
            })?;
        file.write_all(&bytes).map_err(|error| {
            Box::new(AppError::io(
                "append edit journal",
                path.display().to_string(),
                &error,
            ))
        })?;
        file.sync_data().map_err(|error| {
            Box::new(AppError::io(
                "flush edit journal",
                path.display().to_string(),
                &error,
            ))
        })?;
        self.state.next_event_sequence += 1;
        self.persist()
    }
}
