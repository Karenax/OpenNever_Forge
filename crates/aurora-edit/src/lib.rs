mod mapgen;
mod sync;

pub use mapgen::{
    MAP_GENERATION_SCHEMA_VERSION, MAP_MAX_BLUEPRINTS_PER_RULE, MAP_MAX_DENSITY_RULES,
    MAP_MAX_HEIGHT, MAP_MAX_PLACEMENTS, MAP_MAX_TILES, MAP_MAX_WIDTH, MapCompatibilityReport,
    MapDensityRule, MapGenerationMetrics, MapGenerationPlan, MapGenerationSpec, MapTilePlan,
    generate_map_plan, generate_map_plan_with_compatibility,
};

pub use sync::{
    AURORA_SYNC_SCHEMA_VERSION, AuroraSyncAction, AuroraSyncAppliedFile, AuroraSyncBaseline,
    AuroraSyncBaselineEntry, AuroraSyncDirection, AuroraSyncEntry, AuroraSyncManifest,
    AuroraSyncPlan, AuroraSyncReport, AuroraSyncState, AuroraSyncWorkspaceFile, baseline_from_plan,
    compare_aurora_sync, content_digest, resource_key_from_aurora_path, verify_sync_action,
};

use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey, resource_type_for_extension};
use aurora_erf::{
    ContainerReader, ErfReader, ErfResourceInput, ErfResourceSource, ErfResourceStreamInput,
    write_erf, write_erf_streaming, write_erf_streaming_with_metadata,
};
use aurora_gff::{
    GenericField, GenericGff, GenericStruct, GenericValue, LocalizedString, LocalizedValue,
    parse_gff, read_module_info, write_gff,
};
use aurora_mdl::{MdlFormat, parse_mdl};
use aurora_nwscript::parse_nss;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

pub const EDIT_WORKSPACE_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceFingerprint {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EditCommand {
    SetField {
        resource: ResourceKey,
        path: String,
        before: Value,
        after: Value,
    },
    TransformResource {
        resource: ResourceKey,
        operation: String,
        before_sha256: String,
        after_sha256: String,
    },
    ReplaceText {
        resource: ResourceKey,
        before: String,
        after: String,
    },
    CompileScript {
        resource: ResourceKey,
        inputs: Vec<ResourceContentDigest>,
        compiler_sha256: String,
        before_sha256: Option<String>,
        after_sha256: String,
    },
    MoveInstance {
        area: String,
        instance_id: String,
        before: Transform,
        after: Transform,
    },
    SetTile {
        area: String,
        x: u32,
        y: u32,
        before: TileState,
        after: TileState,
    },
    AddInstance {
        area: String,
        instance_id: String,
        placement: InstancePlacement,
    },
    RemoveInstance {
        area: String,
        instance_id: String,
    },
    CreateResource {
        resource: ResourceKey,
        content_sha256: String,
    },
    DeleteResource {
        resource: ResourceKey,
        content_sha256: String,
    },
    CreateResourceSet {
        resources: Vec<ResourceContentDigest>,
    },
    DeleteResourceSet {
        resources: Vec<ResourceContentDigest>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContentDigest {
    pub resource: ResourceKey,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub bearing: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TileState {
    pub tile_id: u32,
    pub orientation: u8,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstancePlacement {
    pub category: String,
    pub template_resref: String,
    pub tag: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub bearing: f64,
    pub linked_to: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaSpawnPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub orientation: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaEnvironmentPatch {
    pub tag: Option<String>,
    pub comments: Option<String>,
    pub day_night_cycle: Option<bool>,
    pub is_night: Option<bool>,
    pub no_rest: Option<bool>,
    pub player_vs_player: Option<u8>,
    pub chance_rain: Option<u8>,
    pub chance_snow: Option<u8>,
    pub chance_lightning: Option<u8>,
    pub wind_power: Option<u8>,
    pub fog_clip_distance: Option<f32>,
    pub sky_box: Option<u8>,
    pub load_screen_id: Option<u16>,
    pub lighting_scheme: Option<u8>,
    pub shadow_opacity: Option<u8>,
    pub sun_ambient_color: Option<u32>,
    pub sun_diffuse_color: Option<u32>,
    pub sun_fog_color: Option<u32>,
    pub sun_fog_amount: Option<u8>,
    pub sun_shadows: Option<bool>,
    pub moon_ambient_color: Option<u32>,
    pub moon_diffuse_color: Option<u32>,
    pub moon_fog_color: Option<u32>,
    pub moon_fog_amount: Option<u8>,
    pub moon_shadows: Option<bool>,
    pub on_enter: Option<String>,
    pub on_exit: Option<String>,
    pub on_heartbeat: Option<String>,
    pub on_user_defined: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaAudioPatch {
    pub ambient_sound_day: Option<i32>,
    pub ambient_sound_night: Option<i32>,
    pub ambient_sound_day_volume: Option<u8>,
    pub ambient_sound_night_volume: Option<u8>,
    pub environment_audio: Option<i32>,
    pub music_battle: Option<i32>,
    pub music_day: Option<i32>,
    pub music_night: Option<i32>,
    pub music_delay: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AreaStructureAction {
    SetGeometry {
        instance_id: String,
        points: Vec<AreaPoint>,
    },
    SetSpawnPoints {
        instance_id: String,
        points: Vec<AreaSpawnPoint>,
    },
    SetTransition {
        instance_id: String,
        destination: String,
        flags: u8,
        load_screen_id: u16,
    },
    AddInventoryItem {
        instance_id: String,
        resref: String,
        stack_size: u16,
        x: u16,
        y: u16,
        infinite: bool,
        category_index: Option<usize>,
    },
    RemoveInventoryItem {
        instance_id: String,
        item_index: usize,
        category_index: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DialogueNodeKind {
    Entry,
    Reply,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueNodeRef {
    pub kind: DialogueNodeKind,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DialogueStructureAction {
    AddNode {
        node_kind: DialogueNodeKind,
    },
    RemoveNode {
        node: DialogueNodeRef,
    },
    AddLink {
        source: Option<DialogueNodeRef>,
        target: DialogueNodeRef,
    },
    SetLinkScripts {
        source: Option<DialogueNodeRef>,
        position: usize,
        condition_script: Option<String>,
        action_script: Option<String>,
    },
    RemoveLink {
        source: Option<DialogueNodeRef>,
        position: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum JournalStructureAction {
    AddCategory {
        tag: String,
    },
    RemoveCategory {
        category_index: usize,
    },
    AddEntry {
        category_index: usize,
    },
    RemoveEntry {
        category_index: usize,
        entry_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum FactionStructureAction {
    AddFaction {
        name: String,
        parent_id: Option<u32>,
    },
    RemoveFaction {
        faction_index: usize,
    },
    AddReputation {
        source_id: u32,
        target_id: u32,
        value: u32,
    },
    RemoveReputation {
        reputation_index: usize,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintListKind {
    Feat,
    SpecialAbility,
    Class,
    EquippedItem,
    ItemProperty,
    Sound,
    EncounterCreature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BlueprintStructureAction {
    AddFeat {
        feat_id: u16,
    },
    AddSpecialAbility {
        spell_id: u16,
        caster_level: u8,
        flags: u8,
    },
    AddClass {
        class_id: u32,
        class_level: u16,
    },
    AddEquippedItem {
        resref: String,
        slot: u32,
    },
    AddItemProperty {
        property_name: u16,
        subtype: u16,
        cost_table: u8,
        cost_value: u16,
        param1: u8,
        param1_value: u8,
        chance_appear: u8,
    },
    AddSound {
        resref: String,
    },
    AddEncounterCreature {
        resref: String,
        appearance: u32,
        challenge_rating: f32,
        single_spawn: bool,
    },
    RemoveEntry {
        list_kind: BlueprintListKind,
        entry_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModifiedResource {
    pub resource: ResourceKey,
    pub source_sha256: Option<String>,
    pub output_sha256: String,
    pub size_bytes: u64,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMigrationRecord {
    pub from_version: u32,
    pub to_version: u32,
    pub backup_path: String,
    pub steps: Vec<String>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshDraft {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    #[serde(default)]
    pub surface_ids: Vec<i32>,
    #[serde(default)]
    pub variants: Vec<WalkmeshVariantDraft>,
    #[serde(default)]
    pub hooks: Vec<WalkmeshHookDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshVariantDraft {
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    #[serde(default)]
    pub surface_ids: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshHookDraft {
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalkmeshKind {
    Wok,
    Pwk,
    Dwk,
}

impl WalkmeshKind {
    pub fn resource_type(self) -> u16 {
        match self {
            Self::Wok => 2016,
            Self::Dwk => 2052,
            Self::Pwk => 2053,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WalkmeshOperation {
    SplitFace {
        face_index: usize,
    },
    RemoveFace {
        face_index: usize,
    },
    WeldVertices {
        tolerance: f32,
    },
    ExtrudeFace {
        face_index: usize,
        distance: f32,
    },
    MoveVertex {
        vertex_index: usize,
        position: [f32; 3],
    },
    SetSurface {
        face_index: usize,
        surface_id: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshDocument {
    pub resref: String,
    pub kind: WalkmeshKind,
    pub source_format: String,
    pub draft: WalkmeshDraft,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshValidation {
    pub valid: bool,
    pub diagnostics: Vec<String>,
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

pub fn ai_change_set_sha256(change_set: &AiChangeSet) -> AppResult<String> {
    validate_controlled_ai_change_set(change_set)?;
    let encoded = serde_json::to_vec(change_set).map_err(|error| {
        edit_error(
            "EDIT_AI_PROPOSAL_SERIALIZE_FAILED",
            format!("cannot serialize AI proposal: {error}"),
        )
    })?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub fn validate_controlled_ai_change_set(change_set: &AiChangeSet) -> AppResult<()> {
    if change_set.summary.trim().is_empty() || change_set.summary.len() > 512 {
        return Err(edit_error(
            "EDIT_AI_SUMMARY_INVALID",
            "AI proposals require a non-empty summary of at most 512 bytes",
        ));
    }
    if change_set.commands.is_empty() || change_set.commands.len() > 32 {
        return Err(edit_error(
            "EDIT_AI_COMMAND_COUNT_INVALID",
            "AI proposals must contain between 1 and 32 operations",
        ));
    }
    let encoded_size = serde_json::to_vec(change_set)
        .map_err(|error| {
            edit_error(
                "EDIT_AI_PROPOSAL_SERIALIZE_FAILED",
                format!("cannot serialize AI proposal: {error}"),
            )
        })?
        .len();
    if encoded_size > 1024 * 1024 {
        return Err(edit_error(
            "EDIT_AI_PROPOSAL_TOO_LARGE",
            "AI proposal JSON exceeds the 1 MiB safety limit",
        ));
    }
    for command in &change_set.commands {
        command
            .validate()
            .map_err(|message| edit_error("EDIT_AI_COMMAND_INVALID", message))?;
        match command {
            EditCommand::SetField {
                resource,
                path,
                before,
                after,
            } => {
                validate_resref(&resource.resref)?;
                let extension = aurora_core::resource_extension(resource.resource_type);
                if !matches!(
                    extension,
                    Some(
                        "are"
                            | "ifo"
                            | "bic"
                            | "git"
                            | "uti"
                            | "utc"
                            | "dlg"
                            | "itp"
                            | "utt"
                            | "uts"
                            | "gff"
                            | "fac"
                            | "ute"
                            | "utd"
                            | "utp"
                            | "gic"
                            | "gui"
                            | "utm"
                            | "jrl"
                            | "utw"
                    )
                ) {
                    return Err(edit_error(
                        "EDIT_AI_RESOURCE_TYPE_UNSUPPORTED",
                        format!("{resource} is not a supported GFF resource"),
                    ));
                }
                if path.len() > 256
                    || serde_json::to_vec(before).is_ok_and(|value| value.len() > 64 * 1024)
                    || serde_json::to_vec(after).is_ok_and(|value| value.len() > 64 * 1024)
                {
                    return Err(edit_error(
                        "EDIT_AI_FIELD_OPERATION_TOO_LARGE",
                        format!("field operation for {resource} exceeds its safety limit"),
                    ));
                }
            }
            EditCommand::ReplaceText {
                resource,
                before,
                after,
            } => {
                validate_resref(&resource.resref)?;
                if resource.resource_type != 2009 {
                    return Err(edit_error(
                        "EDIT_AI_RESOURCE_TYPE_UNSUPPORTED",
                        format!("AI text replacement is limited to NSS, not {resource}"),
                    ));
                }
                if before.len() > 256 * 1024 || after.len() > 256 * 1024 {
                    return Err(edit_error(
                        "EDIT_AI_SCRIPT_TOO_LARGE",
                        format!("AI script operation for {resource} exceeds 256 KiB"),
                    ));
                }
            }
            _ => {
                return Err(edit_error(
                    "EDIT_AI_COMMAND_UNSUPPORTED",
                    "AI output may only contain set_field and replace_text operations",
                ));
            }
        }
    }
    Ok(())
}

fn controlled_ai_resource(command: &EditCommand) -> &ResourceKey {
    match command {
        EditCommand::SetField { resource, .. } | EditCommand::ReplaceText { resource, .. } => {
            resource
        }
        _ => unreachable!("controlled AI validation rejects unsupported commands"),
    }
}

impl PaletteManifest {
    pub fn standard() -> Self {
        Self {
            schema_version: 1,
            categories: vec![
                PaletteCategory {
                    id: "creatures".to_owned(),
                    label: "Créatures".to_owned(),
                    resource_types: vec![2027],
                },
                PaletteCategory {
                    id: "doors".to_owned(),
                    label: "Portes".to_owned(),
                    resource_types: vec![2042],
                },
                PaletteCategory {
                    id: "encounters".to_owned(),
                    label: "Rencontres".to_owned(),
                    resource_types: vec![2040],
                },
                PaletteCategory {
                    id: "items".to_owned(),
                    label: "Objets".to_owned(),
                    resource_types: vec![2025],
                },
                PaletteCategory {
                    id: "placeables".to_owned(),
                    label: "Plaçables".to_owned(),
                    resource_types: vec![2044],
                },
                PaletteCategory {
                    id: "sounds".to_owned(),
                    label: "Sons".to_owned(),
                    resource_types: vec![2035],
                },
                PaletteCategory {
                    id: "stores".to_owned(),
                    label: "Marchands".to_owned(),
                    resource_types: vec![2051],
                },
                PaletteCategory {
                    id: "triggers".to_owned(),
                    label: "Déclencheurs".to_owned(),
                    resource_types: vec![2032],
                },
                PaletteCategory {
                    id: "waypoints".to_owned(),
                    label: "Points de passage".to_owned(),
                    resource_types: vec![2058],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorkspace {
    schema_version: u32,
    workspace_id: String,
    source: SourceFingerprint,
    values: BTreeMap<String, Value>,
    timeline: Vec<EditCommand>,
    cursor: usize,
    modified_resources: BTreeMap<String, ModifiedResource>,
    #[serde(default)]
    deleted_resources: BTreeMap<String, ResourceKey>,
    #[serde(default)]
    resource_revisions: Vec<Option<ResourceRevision>>,
    #[serde(default)]
    pending_revision: Option<ResourceRevision>,
    next_event_sequence: u64,
    #[serde(default)]
    migration_history: Vec<WorkspaceMigrationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceRevision {
    resource: ResourceKey,
    before_blob: Option<String>,
    before_modified: Option<ModifiedResource>,
    #[serde(default)]
    before_deleted: Option<ResourceKey>,
    after_blob: Option<String>,
    after_modified: Option<ModifiedResource>,
    #[serde(default)]
    after_deleted: Option<ResourceKey>,
    #[serde(default)]
    related: Vec<ResourceRevision>,
}

impl ResourceRevision {
    fn resources(&self) -> Vec<ResourceKey> {
        let mut resources = Vec::with_capacity(1 + self.related.len());
        resources.push(self.resource.clone());
        resources.extend(
            self.related
                .iter()
                .map(|revision| revision.resource.clone()),
        );
        resources.sort();
        resources
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalEvent<'a> {
    sequence: u64,
    action: &'a str,
    cursor_before: usize,
    cursor_after: usize,
    command: Option<&'a EditCommand>,
}

#[derive(Debug)]
pub struct EditWorkspace {
    root: PathBuf,
    state: PersistedWorkspace,
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

pub fn create_empty_module(
    output_path: &Path,
    definition: &NewModuleDefinition,
) -> AppResult<ModuleBuildReport> {
    if output_path.exists() {
        return Err(edit_error(
            "EDIT_NEW_MODULE_OUTPUT_EXISTS",
            format!("{} already exists", output_path.display()),
        ));
    }
    if !output_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("mod"))
    {
        return Err(edit_error(
            "EDIT_BUILD_EXTENSION_INVALID",
            "new module output must use the .mod extension",
        ));
    }
    validate_resref(&definition.entry_area)?;
    if definition.name.trim().is_empty() || definition.tag.trim().is_empty() {
        return Err(edit_error(
            "EDIT_NEW_MODULE_IDENTITY_INVALID",
            "module name and tag cannot be empty",
        ));
    }
    let resources = new_module_resources(definition)?;
    let output = write_erf("MOD ", &resources)?;
    atomic_write(output_path, &output)?;
    let inventory = ErfReader::default().read_inventory(output_path, &AtomicBool::new(false))?;
    if inventory.resource_count as usize != resources.len() {
        return Err(edit_error(
            "EDIT_BUILD_REOPEN_FAILED",
            "new module resource count changed after reopen",
        ));
    }
    Ok(ModuleBuildReport {
        output_path: output_path.display().to_string(),
        sha256: sha256_bytes(&output),
        size_bytes: output.len() as u64,
        resource_count: resources.len(),
        modified_resources: resources.len(),
        deleted_resources: 0,
        source_intact: true,
    })
}

pub fn validate_walkmesh(draft: &WalkmeshDraft) -> WalkmeshValidation {
    validate_walkmesh_document(draft, false)
}

pub fn validate_walkmesh_for_kind(draft: &WalkmeshDraft, kind: WalkmeshKind) -> WalkmeshValidation {
    let hook_only = !matches!(kind, WalkmeshKind::Wok)
        && draft.vertices.is_empty()
        && draft.faces.is_empty()
        && !draft.hooks.is_empty();
    validate_walkmesh_document(draft, hook_only)
}

fn validate_walkmesh_document(
    draft: &WalkmeshDraft,
    allow_empty_primary: bool,
) -> WalkmeshValidation {
    let mut diagnostics = Vec::new();
    validate_walkmesh_geometry(
        "principal",
        &draft.vertices,
        &draft.faces,
        &draft.surface_ids,
        allow_empty_primary,
        &mut diagnostics,
    );
    for variant in &draft.variants {
        if variant.name.trim().is_empty() || variant.name.len() > 63 {
            diagnostics.push("Une variante possède un nom vide ou trop long.".to_owned());
        }
        if !variant.position.iter().all(|value| value.is_finite())
            || !variant.rotation.iter().all(|value| value.is_finite())
        {
            diagnostics.push(format!(
                "Variante {} avec transformation non finie.",
                variant.name
            ));
        }
        validate_walkmesh_geometry(
            &variant.name,
            &variant.vertices,
            &variant.faces,
            &variant.surface_ids,
            false,
            &mut diagnostics,
        );
    }
    for hook in &draft.hooks {
        if hook.name.trim().is_empty() || hook.name.len() > 63 {
            diagnostics.push("Un point d'usage possède un nom vide ou trop long.".to_owned());
        }
        if !hook.position.iter().all(|value| value.is_finite())
            || !hook.rotation.iter().all(|value| value.is_finite())
        {
            diagnostics.push(format!("Point d'usage {} non fini.", hook.name));
        }
    }
    WalkmeshValidation {
        valid: diagnostics.is_empty(),
        diagnostics,
    }
}

fn validate_walkmesh_geometry(
    label: &str,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    surface_ids: &[i32],
    allow_empty: bool,
    diagnostics: &mut Vec<String>,
) {
    if (vertices.is_empty() || faces.is_empty())
        && !(allow_empty && vertices.is_empty() && faces.is_empty())
    {
        diagnostics.push(format!("Le walkmesh {label} ne contient pas de géométrie."));
    }
    if vertices.len() > 1_000_000 || faces.len() > 2_000_000 {
        diagnostics.push(format!(
            "Le walkmesh {label} dépasse les limites de sécurité."
        ));
    }
    for (index, vertex) in vertices.iter().enumerate() {
        if !vertex.iter().all(|value| value.is_finite()) {
            diagnostics.push(format!("Sommet {index} de {label} non fini."));
        }
    }
    let mut unique_faces = std::collections::BTreeSet::new();
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, bool)>>::new();
    for (index, face) in faces.iter().enumerate() {
        if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
            diagnostics.push(format!("Face {index} de {label} dégénérée."));
        }
        if face.iter().any(|vertex| *vertex as usize >= vertices.len()) {
            diagnostics.push(format!(
                "Face {index} de {label} référence un sommet absent."
            ));
            continue;
        }
        let mut canonical = *face;
        canonical.sort_unstable();
        if !unique_faces.insert(canonical) {
            diagnostics.push(format!("Face {index} de {label} dupliquée."));
        }
        let [a, b, c] = face.map(|vertex| vertices[vertex as usize]);
        let cross = triangle_cross(a, b, c);
        if vector_length_squared(cross) <= 1.0e-12 {
            diagnostics.push(format!(
                "Face {index} de {label} de surface géométrique nulle."
            ));
        }
        for (from, to) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = if from < to { (from, to) } else { (to, from) };
            edges.entry(edge).or_default().push((index, from < to));
        }
    }
    for (edge, owners) in edges {
        if owners.len() > 2 {
            diagnostics.push(format!(
                "Arête {}-{} non-manifold dans {label} ({} faces).",
                edge.0,
                edge.1,
                owners.len()
            ));
        } else if owners.len() == 2 && owners[0].1 == owners[1].1 {
            diagnostics.push(format!(
                "Orientation incohérente autour de l'arête {}-{} dans {label}.",
                edge.0, edge.1
            ));
        }
    }
    if !surface_ids.is_empty() && surface_ids.len() != faces.len() {
        diagnostics.push(format!(
            "Le nombre d'identifiants de surface de {label} doit correspondre au nombre de faces."
        ));
    }
}

/// Imports standalone ASCII WOK/PWK/DWK resources and regular MDL AABB data.
/// Every geometry state and use/door hook remains explicit in the draft.
pub fn inspect_walkmesh(
    resref: &str,
    kind: WalkmeshKind,
    bytes: &[u8],
) -> AppResult<WalkmeshDocument> {
    validate_walkmesh_resref(resref)?;
    let model = parse_mdl(bytes).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_WALKMESH_PARSE_FAILED",
                "Le walkmesh n'a pas pu etre lu.",
                error.to_string(),
                ErrorSeverity::Error,
            )
            .with_resource(format!("{resref}.{}", walkmesh_extension(kind)))
            .with_import_stage("walkmesh"),
        )
    })?;
    let mesh_nodes = model
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.mesh.as_ref().is_some_and(|mesh| match kind {
                WalkmeshKind::Wok => mesh.walkmesh,
                WalkmeshKind::Pwk | WalkmeshKind::Dwk => true,
            })
        })
        .collect::<Vec<_>>();
    let primary = mesh_nodes
        .iter()
        .position(|(_, node)| match kind {
            WalkmeshKind::Wok => node.mesh.as_ref().is_some_and(|mesh| mesh.walkmesh),
            WalkmeshKind::Pwk => node.name.to_ascii_lowercase().contains("nowalk"),
            WalkmeshKind::Dwk => node.name.to_ascii_lowercase().contains("wg_closed"),
        })
        .unwrap_or_default();
    let mut draft = WalkmeshDraft::default();
    for (position, (_, node)) in mesh_nodes.iter().enumerate() {
        let mesh = node.mesh.as_ref().expect("mesh node filtered above");
        let geometry = walkmesh_geometry_from_mesh(mesh);
        if position == primary {
            draft.vertices = geometry
                .0
                .into_iter()
                .map(|vertex| transform_point(vertex, node.translation, node.rotation))
                .collect();
            draft.faces = geometry.1;
            draft.surface_ids = geometry.2;
        } else {
            draft.variants.push(WalkmeshVariantDraft {
                name: node.name.clone(),
                position: node.translation,
                rotation: node.rotation,
                vertices: geometry.0,
                faces: geometry.1,
                surface_ids: geometry.2,
            });
        }
    }
    draft.hooks = model
        .nodes
        .iter()
        .filter(|node| node.mesh.is_none() && node.kinds.contains(&aurora_mdl::MdlNodeKind::Dummy))
        .map(|node| WalkmeshHookDraft {
            name: node.name.clone(),
            position: node.translation,
            rotation: node.rotation,
        })
        .collect();
    if draft.faces.is_empty() && (matches!(kind, WalkmeshKind::Wok) || draft.hooks.is_empty()) {
        return Err(edit_error(
            "EDIT_WALKMESH_EMPTY",
            format!(
                "{resref}.{} contains no walkmesh face",
                walkmesh_extension(kind)
            ),
        ));
    }
    Ok(WalkmeshDocument {
        resref: resref.to_owned(),
        kind,
        source_format: match model.format {
            MdlFormat::Ascii => "ascii",
            MdlFormat::Binary => "binary",
        }
        .to_owned(),
        draft,
        source_sha256: sha256_bytes(bytes),
    })
}

fn walkmesh_geometry_from_mesh(
    mesh: &aurora_mdl::MdlMesh,
) -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<i32>) {
    let faces = mesh
        .indices
        .chunks_exact(3)
        .map(|face| [face[0], face[1], face[2]])
        .collect::<Vec<_>>();
    let surfaces = (0..faces.len())
        .map(|index| mesh.surface_ids.get(index).copied().unwrap_or_default())
        .collect();
    (mesh.positions.clone(), faces, surfaces)
}

/// Serializes the standalone ASCII grammars used by NWN for WOK, PWK and DWK.
/// WOK resources include a deterministic AABB tree; PWK/DWK keep every
/// geometry state and interaction hook.
pub fn serialize_walkmesh_ascii(
    resref: &str,
    kind: WalkmeshKind,
    draft: &WalkmeshDraft,
) -> AppResult<Vec<u8>> {
    validate_walkmesh_resref(resref)?;
    let validation = validate_walkmesh_for_kind(draft, kind);
    if !validation.valid {
        return Err(edit_error(
            "EDIT_WALKMESH_INVALID",
            validation.diagnostics.join(" "),
        ));
    }
    validate_walkmesh_kind(kind, draft)?;
    let mut output = String::from("# Generated by OpenNever Forge (Apache-2.0)\n");
    match kind {
        WalkmeshKind::Wok => {
            output.push_str("#NWmax WALKMESH  ASCII\n");
            output.push_str(&format!("beginwalkmeshgeom {resref}\n"));
            output.push_str(&format!("node aabb {resref}\n  parent {resref}\n"));
            output.push_str("  position 0 0 0\n  orientation 1 0 0 0\n  render 0\n");
            output.push_str("  multimaterial 20\n");
            for surface in WALKMESH_SURFACES {
                output.push_str(&format!("    {surface}\n"));
            }
            write_walkmesh_geometry(
                &mut output,
                &draft.vertices,
                &draft.faces,
                &draft.surface_ids,
                true,
            );
            write_walkmesh_aabb_tree(&mut output, &draft.vertices, &draft.faces);
            output.push_str("endnode\n");
            output.push_str(&format!("endwalkmeshgeom {resref}\n"));
        }
        WalkmeshKind::Pwk => {
            output.push_str("#NWmax PWKMESH  ASCII\n");
            let parent = format!("{resref}_pwk");
            if !draft.faces.is_empty() {
                write_walkmesh_trimesh(
                    &mut output,
                    "NoWalk",
                    &parent,
                    WalkmeshTrimesh {
                        position: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        vertices: &draft.vertices,
                        faces: &draft.faces,
                        surfaces: &draft.surface_ids,
                    },
                );
            }
            for variant in &draft.variants {
                write_walkmesh_trimesh(
                    &mut output,
                    &variant.name,
                    &parent,
                    WalkmeshTrimesh {
                        position: variant.position,
                        rotation: variant.rotation,
                        vertices: &variant.vertices,
                        faces: &variant.faces,
                        surfaces: &variant.surface_ids,
                    },
                );
            }
            let hooks = if draft.hooks.is_empty() {
                default_pwk_hooks(resref, &draft.vertices)
            } else {
                draft.hooks.clone()
            };
            for hook in &hooks {
                write_walkmesh_hook(&mut output, hook, &parent);
            }
        }
        WalkmeshKind::Dwk => {
            output.push_str("#NWmax DWKMESH  ASCII\n");
            let parent = format!("{resref}_DWK");
            let hooks = if draft.hooks.is_empty() {
                default_dwk_hooks(resref, &draft.vertices)
            } else {
                draft.hooks.clone()
            };
            for hook in &hooks {
                write_walkmesh_hook(&mut output, hook, &parent);
            }
            if !draft.faces.is_empty() {
                write_walkmesh_trimesh(
                    &mut output,
                    &format!("{resref}_DWK_wg_closed"),
                    &parent,
                    WalkmeshTrimesh {
                        position: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        vertices: &draft.vertices,
                        faces: &draft.faces,
                        surfaces: &draft.surface_ids,
                    },
                );
                let variants = dwk_variants(resref, draft);
                for variant in &variants {
                    write_walkmesh_trimesh(
                        &mut output,
                        &variant.name,
                        &parent,
                        WalkmeshTrimesh {
                            position: variant.position,
                            rotation: variant.rotation,
                            vertices: &variant.vertices,
                            faces: &variant.faces,
                            surfaces: &variant.surface_ids,
                        },
                    );
                }
            }
        }
    }
    Ok(output.into_bytes())
}

pub fn split_walkmesh_face(draft: &mut WalkmeshDraft, face_index: usize) -> AppResult<()> {
    let face = *draft.faces.get(face_index).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_NOT_FOUND",
            format!("face {face_index} does not exist"),
        )
    })?;
    if face
        .iter()
        .any(|vertex| *vertex as usize >= draft.vertices.len())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} references a missing vertex"),
        ));
    }
    let [a, b, c] = face.map(|vertex| draft.vertices[vertex as usize]);
    let centroid = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    let center = draft.vertices.len() as u32;
    draft.vertices.push(centroid);
    draft.faces[face_index] = [face[0], face[1], center];
    draft.faces.push([face[1], face[2], center]);
    draft.faces.push([face[2], face[0], center]);
    if !draft.surface_ids.is_empty() {
        let surface = draft.surface_ids[face_index];
        draft.surface_ids.push(surface);
        draft.surface_ids.push(surface);
    }
    Ok(())
}

pub fn apply_walkmesh_operation(
    draft: &mut WalkmeshDraft,
    operation: &WalkmeshOperation,
) -> AppResult<WalkmeshValidation> {
    match operation {
        WalkmeshOperation::SplitFace { face_index } => split_walkmesh_face(draft, *face_index)?,
        WalkmeshOperation::RemoveFace { face_index } => {
            if *face_index >= draft.faces.len() {
                return Err(edit_error(
                    "EDIT_WALKMESH_FACE_NOT_FOUND",
                    format!("face {face_index} does not exist"),
                ));
            }
            draft.faces.remove(*face_index);
            if !draft.surface_ids.is_empty() && *face_index < draft.surface_ids.len() {
                draft.surface_ids.remove(*face_index);
            }
            compact_walkmesh_vertices(draft);
        }
        WalkmeshOperation::WeldVertices { tolerance } => {
            weld_walkmesh_vertices(draft, *tolerance)?;
        }
        WalkmeshOperation::ExtrudeFace {
            face_index,
            distance,
        } => extrude_walkmesh_face(draft, *face_index, *distance)?,
        WalkmeshOperation::MoveVertex {
            vertex_index,
            position,
        } => {
            if !position.iter().all(|value| value.is_finite()) {
                return Err(edit_error(
                    "EDIT_WALKMESH_VERTEX_INVALID",
                    "vertex position must contain finite values",
                ));
            }
            let vertex = draft.vertices.get_mut(*vertex_index).ok_or_else(|| {
                edit_error(
                    "EDIT_WALKMESH_VERTEX_NOT_FOUND",
                    format!("vertex {vertex_index} does not exist"),
                )
            })?;
            *vertex = *position;
        }
        WalkmeshOperation::SetSurface {
            face_index,
            surface_id,
        } => {
            if *face_index >= draft.faces.len() {
                return Err(edit_error(
                    "EDIT_WALKMESH_FACE_NOT_FOUND",
                    format!("face {face_index} does not exist"),
                ));
            }
            draft.surface_ids.resize(draft.faces.len(), 0);
            draft.surface_ids[*face_index] = *surface_id;
        }
    }
    Ok(validate_walkmesh(draft))
}

fn extrude_walkmesh_face(
    draft: &mut WalkmeshDraft,
    face_index: usize,
    distance: f32,
) -> AppResult<()> {
    if !distance.is_finite() || distance.abs() < 1.0e-5 || distance.abs() > 1_000.0 {
        return Err(edit_error(
            "EDIT_WALKMESH_EXTRUSION_INVALID",
            "extrusion distance must be finite and between 0.00001 and 1000",
        ));
    }
    let face = *draft.faces.get(face_index).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_NOT_FOUND",
            format!("face {face_index} does not exist"),
        )
    })?;
    if face
        .iter()
        .any(|vertex| *vertex as usize >= draft.vertices.len())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} references a missing vertex"),
        ));
    }
    let [a, b, c] = face.map(|vertex| draft.vertices[vertex as usize]);
    let normal = normalize_vector(triangle_cross(a, b, c)).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} has no geometric normal"),
        )
    })?;
    let offset = normal.map(|value| value * distance);
    let [d, e, f] = [a, b, c].map(|vertex| {
        [
            vertex[0] + offset[0],
            vertex[1] + offset[1],
            vertex[2] + offset[2],
        ]
    });
    let first = draft.vertices.len() as u32;
    draft.vertices.extend([d, e, f]);
    let [a, b, c] = face;
    let [d, e, f] = [first, first + 1, first + 2];
    draft.faces.extend([
        [f, e, d],
        [b, a, d],
        [b, d, e],
        [c, b, e],
        [c, e, f],
        [a, c, f],
        [a, f, d],
    ]);
    if !draft.surface_ids.is_empty() {
        let surface = draft
            .surface_ids
            .get(face_index)
            .copied()
            .unwrap_or_default();
        draft.surface_ids.extend([surface; 7]);
    }
    Ok(())
}

fn weld_walkmesh_vertices(draft: &mut WalkmeshDraft, tolerance: f32) -> AppResult<()> {
    if !tolerance.is_finite() || !(1.0e-6..=10.0).contains(&tolerance) {
        return Err(edit_error(
            "EDIT_WALKMESH_WELD_TOLERANCE_INVALID",
            "weld tolerance must be between 0.000001 and 10",
        ));
    }
    let mut cells = BTreeMap::<(i64, i64, i64), Vec<u32>>::new();
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut remap = Vec::with_capacity(draft.vertices.len());
    for vertex in &draft.vertices {
        let cell = (
            (vertex[0] / tolerance).floor() as i64,
            (vertex[1] / tolerance).floor() as i64,
            (vertex[2] / tolerance).floor() as i64,
        );
        let mut replacement = None;
        'neighbors: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(candidates) = cells.get(&(cell.0 + x, cell.1 + y, cell.2 + z)) {
                        for candidate in candidates {
                            let existing = vertices[*candidate as usize];
                            let delta = [
                                existing[0] - vertex[0],
                                existing[1] - vertex[1],
                                existing[2] - vertex[2],
                            ];
                            if vector_length_squared(delta) <= tolerance * tolerance {
                                replacement = Some(*candidate);
                                break 'neighbors;
                            }
                        }
                    }
                }
            }
        }
        let index = replacement.unwrap_or_else(|| {
            let index = vertices.len() as u32;
            vertices.push(*vertex);
            cells.entry(cell).or_default().push(index);
            index
        });
        remap.push(index);
    }
    let mut faces = Vec::with_capacity(draft.faces.len());
    let mut surfaces = Vec::with_capacity(draft.faces.len());
    for (index, face) in draft.faces.iter().enumerate() {
        if face.iter().any(|value| *value as usize >= remap.len()) {
            return Err(edit_error(
                "EDIT_WALKMESH_FACE_INVALID",
                format!("face {index} references a missing vertex"),
            ));
        }
        let face = face.map(|value| remap[value as usize]);
        if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
            continue;
        }
        faces.push(face);
        surfaces.push(draft.surface_ids.get(index).copied().unwrap_or_default());
    }
    draft.vertices = vertices;
    draft.faces = faces;
    draft.surface_ids = surfaces;
    compact_walkmesh_vertices(draft);
    Ok(())
}

fn compact_walkmesh_vertices(draft: &mut WalkmeshDraft) {
    let mut used = vec![false; draft.vertices.len()];
    for face in &draft.faces {
        for vertex in face {
            if let Some(value) = used.get_mut(*vertex as usize) {
                *value = true;
            }
        }
    }
    let mut remap = vec![0_u32; draft.vertices.len()];
    let mut vertices = Vec::with_capacity(draft.vertices.len());
    for (index, vertex) in draft.vertices.iter().enumerate() {
        if used[index] {
            remap[index] = vertices.len() as u32;
            vertices.push(*vertex);
        }
    }
    for face in &mut draft.faces {
        *face = face.map(|vertex| remap[vertex as usize]);
    }
    draft.vertices = vertices;
}

const WALKMESH_SURFACES: [&str; 20] = [
    "Dirt",
    "Obscuring",
    "Grass",
    "Stone",
    "Wood",
    "Water",
    "Nonwalk",
    "Transparent",
    "Carpet",
    "Metal",
    "Puddles",
    "Swamp",
    "Mud",
    "Leaves",
    "Lava",
    "BottomlessPit",
    "DeepWater",
    "Door",
    "Snow",
    "Sand",
];

fn validate_walkmesh_kind(kind: WalkmeshKind, draft: &WalkmeshDraft) -> AppResult<()> {
    if matches!(kind, WalkmeshKind::Wok)
        && draft
            .surface_ids
            .iter()
            .any(|surface| !(0..=19).contains(surface))
    {
        return Err(edit_error(
            "EDIT_WALKMESH_SURFACE_INVALID",
            "WOK surface identifiers must be between 0 and 19",
        ));
    }
    if matches!(kind, WalkmeshKind::Wok) && (!draft.variants.is_empty() || !draft.hooks.is_empty())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_WOK_STRUCTURE_INVALID",
            "WOK resources accept one AABB geometry and no PWK/DWK hooks",
        ));
    }
    Ok(())
}

fn write_walkmesh_geometry(
    output: &mut String,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    surfaces: &[i32],
    include_tverts: bool,
) {
    output.push_str(&format!("  verts {}\n", vertices.len()));
    for vertex in vertices {
        output.push_str(&format!(
            "    {} {} {}\n",
            format_walkmesh_float(vertex[0]),
            format_walkmesh_float(vertex[1]),
            format_walkmesh_float(vertex[2])
        ));
    }
    output.push_str(&format!("  faces {}\n", faces.len()));
    for (index, face) in faces.iter().enumerate() {
        let surface = surfaces.get(index).copied().unwrap_or_default();
        let texture = if include_tverts { *face } else { [0, 0, 0] };
        output.push_str(&format!(
            "    {} {} {}  {}  {} {} {}  {}\n",
            face[0],
            face[1],
            face[2],
            surface,
            texture[0],
            texture[1],
            texture[2],
            if include_tverts { 4 } else { 7 }
        ));
    }
    if include_tverts {
        let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
        let width = (maximum[0] - minimum[0]).abs().max(1.0);
        let height = (maximum[1] - minimum[1]).abs().max(1.0);
        output.push_str(&format!("  tverts {}\n", vertices.len()));
        for vertex in vertices {
            output.push_str(&format!(
                "    {} {} 0\n",
                format_walkmesh_float((vertex[0] - minimum[0]) / width),
                format_walkmesh_float((vertex[1] - minimum[1]) / height)
            ));
        }
    }
}

struct WalkmeshTrimesh<'a> {
    position: [f32; 3],
    rotation: [f32; 4],
    vertices: &'a [[f32; 3]],
    faces: &'a [[u32; 3]],
    surfaces: &'a [i32],
}

fn write_walkmesh_trimesh(
    output: &mut String,
    name: &str,
    parent: &str,
    geometry: WalkmeshTrimesh<'_>,
) {
    output.push_str(&format!("node trimesh {name}\n  parent {parent}\n"));
    write_walkmesh_transform(output, geometry.position, geometry.rotation);
    output.push_str("  bitmap NULL\n");
    write_walkmesh_geometry(
        output,
        geometry.vertices,
        geometry.faces,
        geometry.surfaces,
        false,
    );
    output.push_str("endnode\n");
}

fn write_walkmesh_hook(output: &mut String, hook: &WalkmeshHookDraft, parent: &str) {
    output.push_str(&format!("node dummy {}\n  parent {parent}\n", hook.name));
    write_walkmesh_transform(output, hook.position, hook.rotation);
    output.push_str("endnode\n");
}

fn write_walkmesh_transform(output: &mut String, position: [f32; 3], rotation: [f32; 4]) {
    let orientation = quaternion_axis_angle(rotation);
    output.push_str(&format!(
        "  position {} {} {}\n  orientation {} {} {} {}\n",
        format_walkmesh_float(position[0]),
        format_walkmesh_float(position[1]),
        format_walkmesh_float(position[2]),
        format_walkmesh_float(orientation[0]),
        format_walkmesh_float(orientation[1]),
        format_walkmesh_float(orientation[2]),
        format_walkmesh_float(orientation[3])
    ));
}

fn default_pwk_hooks(resref: &str, vertices: &[[f32; 3]]) -> Vec<WalkmeshHookDraft> {
    let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
    let x = (minimum[0] + maximum[0]) * 0.5;
    let z = maximum[2];
    vec![
        WalkmeshHookDraft {
            name: format!("{resref}_pwk_use01"),
            position: [x, maximum[1], z],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
        WalkmeshHookDraft {
            name: format!("{resref}_pwk_use02"),
            position: [x, minimum[1], z],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    ]
}

fn default_dwk_hooks(resref: &str, vertices: &[[f32; 3]]) -> Vec<WalkmeshHookDraft> {
    let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
    let x = (minimum[0] + maximum[0]) * 0.5;
    let y = (minimum[1] + maximum[1]) * 0.5;
    let z = minimum[2];
    [
        ("closed_01", [x, minimum[1], z]),
        ("closed_02", [x, maximum[1], z]),
        ("open1_01", [minimum[0], y, z]),
        ("open1_02", [minimum[0], maximum[1], z]),
        ("open2_01", [maximum[0], y, z]),
        ("open2_02", [maximum[0], minimum[1], z]),
    ]
    .into_iter()
    .map(|(suffix, position)| WalkmeshHookDraft {
        name: format!("{resref}_DWK_dp_{suffix}"),
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
    })
    .collect()
}

fn dwk_variants(resref: &str, draft: &WalkmeshDraft) -> Vec<WalkmeshVariantDraft> {
    ["open1", "open2"]
        .into_iter()
        .map(|state| {
            draft
                .variants
                .iter()
                .find(|variant| variant.name.to_ascii_lowercase().contains(state))
                .cloned()
                .unwrap_or_else(|| WalkmeshVariantDraft {
                    name: format!("{resref}_DWK_wg_{state}"),
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    vertices: draft.vertices.clone(),
                    faces: draft.faces.clone(),
                    surface_ids: draft.surface_ids.clone(),
                })
        })
        .collect()
}

fn write_walkmesh_aabb_tree(output: &mut String, vertices: &[[f32; 3]], faces: &[[u32; 3]]) {
    let indices = (0..faces.len()).collect::<Vec<_>>();
    write_aabb_branch(output, vertices, faces, &indices, 4);
}

fn write_aabb_branch(
    output: &mut String,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    indices: &[usize],
    indent: usize,
) {
    let points = indices.iter().flat_map(|index| {
        faces[*index]
            .into_iter()
            .map(|vertex| vertices[vertex as usize])
    });
    let (minimum, maximum) = walkmesh_bounds(points);
    output.push_str(&" ".repeat(indent));
    output.push_str(&format!(
        "aabb {} {} {} {} {} {} {}\n",
        format_walkmesh_float(minimum[0]),
        format_walkmesh_float(minimum[1]),
        format_walkmesh_float(minimum[2]),
        format_walkmesh_float(maximum[0]),
        format_walkmesh_float(maximum[1]),
        format_walkmesh_float(maximum[2]),
        indices
            .first()
            .copied()
            .filter(|_| indices.len() == 1)
            .map(|value| value as i64)
            .unwrap_or(-1)
    ));
    if indices.len() <= 1 {
        return;
    }
    let extent = [
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ];
    let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
        0
    } else if extent[1] >= extent[2] {
        1
    } else {
        2
    };
    let mut sorted = indices.to_vec();
    sorted.sort_by(|left, right| {
        let centroid = |index: usize| {
            let face = faces[index];
            face.into_iter()
                .map(|vertex| vertices[vertex as usize][axis])
                .sum::<f32>()
                / 3.0
        };
        centroid(*left)
            .total_cmp(&centroid(*right))
            .then_with(|| left.cmp(right))
    });
    let middle = sorted.len() / 2;
    write_aabb_branch(output, vertices, faces, &sorted[..middle], indent + 4);
    write_aabb_branch(output, vertices, faces, &sorted[middle..], indent + 4);
}

fn walkmesh_bounds(points: impl Iterator<Item = [f32; 3]>) -> ([f32; 3], [f32; 3]) {
    points.fold(
        ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]),
        |(mut minimum, mut maximum), point| {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(point[axis]);
                maximum[axis] = maximum[axis].max(point[axis]);
            }
            (minimum, maximum)
        },
    )
}

fn triangle_cross(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ]
}

fn vector_length_squared(value: [f32; 3]) -> f32 {
    value.iter().map(|component| component * component).sum()
}

fn normalize_vector(value: [f32; 3]) -> Option<[f32; 3]> {
    let length = vector_length_squared(value).sqrt();
    (length > 1.0e-6).then(|| value.map(|component| component / length))
}

fn transform_point(point: [f32; 3], translation: [f32; 3], rotation: [f32; 4]) -> [f32; 3] {
    let [x, y, z, w] = rotation;
    let dot_uv = x * point[0] + y * point[1] + z * point[2];
    let dot_uu = x * x + y * y + z * z;
    let cross = [
        y * point[2] - z * point[1],
        z * point[0] - x * point[2],
        x * point[1] - y * point[0],
    ];
    [
        2.0 * dot_uv * x + (w * w - dot_uu) * point[0] + 2.0 * w * cross[0] + translation[0],
        2.0 * dot_uv * y + (w * w - dot_uu) * point[1] + 2.0 * w * cross[1] + translation[1],
        2.0 * dot_uv * z + (w * w - dot_uu) * point[2] + 2.0 * w * cross[2] + translation[2],
    ]
}

fn quaternion_axis_angle(rotation: [f32; 4]) -> [f32; 4] {
    let length = rotation
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    let quaternion = if length > f32::EPSILON {
        rotation.map(|value| value / length)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    let angle = 2.0 * quaternion[3].clamp(-1.0, 1.0).acos();
    let sine = (1.0 - quaternion[3] * quaternion[3]).max(0.0).sqrt();
    if sine <= 1.0e-6 {
        [1.0, 0.0, 0.0, 0.0]
    } else {
        [
            quaternion[0] / sine,
            quaternion[1] / sine,
            quaternion[2] / sine,
            angle,
        ]
    }
}

fn validate_walkmesh_resref(resref: &str) -> AppResult<()> {
    if resref.is_empty()
        || resref.len() > 16
        || !resref
            .bytes()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_')
    {
        return Err(edit_error(
            "EDIT_WALKMESH_RESREF_INVALID",
            "walkmesh resref must contain 1-16 lowercase ASCII letters, digits, or underscores",
        ));
    }
    Ok(())
}

fn walkmesh_extension(kind: WalkmeshKind) -> &'static str {
    match kind {
        WalkmeshKind::Wok => "wok",
        WalkmeshKind::Pwk => "pwk",
        WalkmeshKind::Dwk => "dwk",
    }
}

fn format_walkmesh_float(value: f32) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let value = format!("{value:.6}");
    value.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub fn validate_build_profile(profile: &ModuleBuildProfile) -> AppResult<()> {
    let output = Path::new(&profile.output_name);
    if profile.name.trim().is_empty()
        || profile.name.len() > 80
        || output.components().count() != 1
        || !profile.output_name.to_ascii_lowercase().ends_with(".mod")
    {
        return Err(edit_error(
            "EDIT_BUILD_PROFILE_INVALID",
            "profile name is required and output name must end with .mod",
        ));
    }
    for hak in &profile.hak_files {
        validate_dependency_name(hak, "hak")?;
    }
    if let Some(tlk) = &profile.custom_tlk {
        validate_dependency_name(tlk, "tlk")?;
    }
    Ok(())
}

pub fn validate_launch_profile(profile: &NwnLaunchProfile) -> AppResult<()> {
    let executable = Path::new(&profile.executable_path);
    let expected = match profile.mode {
        NwnLaunchMode::Client => "nwmain.exe",
        NwnLaunchMode::Server => "nwserver.exe",
    };
    if profile.name.trim().is_empty()
        || profile.name.len() > 80
        || !executable.is_file()
        || !executable
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(expected))
        || !Path::new(&profile.working_directory).is_dir()
        || profile.arguments.len() > 64
        || profile
            .arguments
            .iter()
            .any(|value| value.len() > 4096 || value.contains('\0'))
    {
        return Err(edit_error(
            "EDIT_LAUNCH_PROFILE_INVALID",
            format!(
                "profile must target an existing {expected} with a valid working directory and bounded arguments"
            ),
        ));
    }
    Ok(())
}

fn safe_profile_name(name: &str) -> String {
    let value = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    value.trim_matches('-').chars().take(64).collect()
}

pub fn edit_module_dependencies(
    bytes: &[u8],
    source: &str,
    hak_files: &[String],
    custom_tlk: Option<&str>,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    if document.file_type != "IFO " {
        return Err(edit_error(
            "EDIT_MODULE_INFO_REQUIRED",
            format!("{source} is {}, expected IFO", document.file_type.trim()),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for hak in hak_files {
        validate_dependency_name(hak, "hak")?;
        if !seen.insert(hak.to_ascii_lowercase()) {
            return Err(edit_error(
                "EDIT_DEPENDENCY_DUPLICATE",
                format!("duplicate HAK dependency {hak:?}"),
            ));
        }
    }
    if let Some(tlk) = custom_tlk.filter(|value| !value.is_empty()) {
        validate_dependency_name(tlk, "tlk")?;
    }
    let hak_value = GenericValue::List(
        hak_files
            .iter()
            .enumerate()
            .map(|(index, hak)| GenericStruct {
                index: index as u32,
                struct_type: 0,
                fields: vec![GenericField {
                    label: "Mod_Hak".into(),
                    field_type: 10,
                    value: GenericValue::String(hak.clone()),
                }],
            })
            .collect(),
    );
    set_root_field(&mut document, "Mod_HakList", 15, hak_value);
    set_root_field(
        &mut document,
        "Mod_CustomTlk",
        10,
        GenericValue::String(custom_tlk.unwrap_or_default().to_owned()),
    );
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifestDefinition {
    pub name: String,
    pub tag: String,
    pub description: String,
    pub entry_area: String,
    pub areas: Vec<String>,
    pub hak_files: Vec<String>,
    pub custom_tlk: Option<String>,
}

pub fn edit_module_manifest(
    bytes: &[u8],
    source: &str,
    definition: &ModuleManifestDefinition,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    if document.file_type != "IFO " {
        return Err(edit_error(
            "EDIT_MODULE_INFO_REQUIRED",
            format!("{source} is {}, expected IFO", document.file_type.trim()),
        ));
    }
    if definition.name.trim().is_empty()
        || definition.name.len() > 1_024
        || definition.tag.trim().is_empty()
        || definition.tag.len() > 64
        || definition.description.len() > 64 * 1024
    {
        return Err(edit_error(
            "EDIT_MODULE_MANIFEST_INVALID",
            "module identity exceeds the supported bounds",
        ));
    }
    validate_resref(&definition.entry_area)?;
    let mut seen = std::collections::BTreeSet::new();
    for area in &definition.areas {
        validate_resref(area)?;
        if !seen.insert(area.to_ascii_lowercase()) {
            return Err(edit_error(
                "EDIT_MODULE_AREA_DUPLICATE",
                format!("duplicate module area {area}"),
            ));
        }
    }
    if !seen.contains(&definition.entry_area.to_ascii_lowercase()) {
        return Err(edit_error(
            "EDIT_MODULE_ENTRY_AREA_MISSING",
            format!("entry area {} is not declared", definition.entry_area),
        ));
    }
    set_root_field(&mut document, "Mod_Name", 12, localized(&definition.name));
    set_root_field(
        &mut document,
        "Mod_Tag",
        10,
        GenericValue::String(definition.tag.clone()),
    );
    set_root_field(
        &mut document,
        "Mod_Description",
        12,
        localized(&definition.description),
    );
    set_root_field(
        &mut document,
        "Mod_Entry_Area",
        11,
        GenericValue::ResRef(definition.entry_area.clone()),
    );
    set_root_field(
        &mut document,
        "Mod_Area_list",
        15,
        GenericValue::List(
            definition
                .areas
                .iter()
                .enumerate()
                .map(|(index, area)| GenericStruct {
                    index: index as u32 + 1,
                    struct_type: 6,
                    fields: vec![gff_field(
                        "Area_Name",
                        11,
                        GenericValue::ResRef(area.clone()),
                    )],
                })
                .collect(),
        ),
    );
    let output = write_gff(&document)?;
    edit_module_dependencies(
        &output,
        source,
        &definition.hak_files,
        definition.custom_tlk.as_deref(),
    )
}

fn set_root_field(document: &mut GenericGff, label: &str, field_type: u32, value: GenericValue) {
    if let Some(field) = document
        .root
        .fields
        .iter_mut()
        .find(|field| field.label == label)
    {
        field.field_type = field_type;
        field.value = value;
    } else {
        document.root.fields.push(GenericField {
            label: label.to_owned(),
            field_type,
            value,
        });
    }
}

pub fn inspect_git_repository(root: &Path) -> AppResult<GitWorkspaceStatus> {
    if !root.is_dir() {
        return Err(edit_error(
            "EDIT_GIT_ROOT_INVALID",
            format!("{} is not a directory", root.display()),
        ));
    }
    let repository_root = git_output(root, &["rev-parse", "--show-toplevel"])?;
    let repository_root = repository_root.trim().to_owned();
    let branch = git_output(root, &["branch", "--show-current"])?
        .trim()
        .to_owned();
    let head = git_output(root, &["rev-parse", "HEAD"])
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let porcelain = git_output(
        root,
        &[
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
        ],
    )?;
    let mut files = Vec::new();
    for line in porcelain.lines() {
        if line.len() < 4 || files.len() >= 10_000 {
            continue;
        }
        files.push(GitFileStatus {
            index_status: line[0..1].to_owned(),
            worktree_status: line[1..2].to_owned(),
            path: line[3..].to_owned(),
        });
    }
    Ok(GitWorkspaceStatus {
        root: repository_root,
        branch,
        head,
        clean: files.is_empty(),
        files,
    })
}

fn git_output(root: &Path, arguments: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| Box::new(AppError::io("run git", root.display().to_string(), &error)))?;
    if !output.status.success() {
        return Err(edit_error(
            "EDIT_GIT_COMMAND_FAILED",
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| edit_error("EDIT_GIT_OUTPUT_INVALID", error.to_string()))
}

pub fn build_custom_hak(
    output_path: &Path,
    resources: &[ErfResourceInput],
) -> AppResult<ModuleBuildReport> {
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
    let output = write_erf("HAK ", resources)?;
    atomic_write(output_path, &output)?;
    let inventory = ErfReader::default().read_inventory(output_path, &AtomicBool::new(false))?;
    Ok(ModuleBuildReport {
        output_path: output_path.display().to_string(),
        sha256: sha256_bytes(&output),
        size_bytes: output.len() as u64,
        resource_count: inventory.resource_count as usize,
        modified_resources: resources.len(),
        deleted_resources: 0,
        source_intact: true,
    })
}

pub fn scan_aurora_workspace(root: &Path) -> AppResult<AuroraSyncManifest> {
    if !root.is_dir() {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_ROOT_INVALID",
            format!("{} is not a directory", root.display()),
        ));
    }
    let mut paths = Vec::new();
    collect_aurora_files(root, root, &mut paths, 0)?;
    paths.sort();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| edit_error("EDIT_AURORA_SYNC_PATH_INVALID", path.display().to_string()))?;
        let metadata = fs::metadata(&path).map_err(|error| {
            Box::new(AppError::io(
                "inspect Aurora workspace file",
                path.display().to_string(),
                &error,
            ))
        })?;
        files.push(DevelopmentFile {
            name: relative.to_string_lossy().replace('\\', "/"),
            sha256: sha256_file(&path)?,
            size_bytes: metadata.len(),
        });
    }
    Ok(AuroraSyncManifest {
        schema_version: AURORA_SYNC_SCHEMA_VERSION,
        root: canonical_toolset_root(root)?,
        files,
    })
}

pub fn read_aurora_workspace_file(root: &Path, relative_path: &str) -> AppResult<Option<Vec<u8>>> {
    let path = safe_aurora_workspace_path(root, relative_path)?;
    if !path.is_file() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path).map_err(|error| {
        Box::new(AppError::io(
            "inspect Aurora synchronization file",
            path.display().to_string(),
            &error,
        ))
    })?;
    if metadata.len() > 256 * 1024 * 1024 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_FILE_TOO_LARGE",
            format!(
                "{} exceeds the 256 MiB synchronization limit",
                path.display()
            ),
        ));
    }
    fs::read(&path).map(Some).map_err(|error| {
        Box::new(AppError::io(
            "read Aurora synchronization file",
            path.display().to_string(),
            &error,
        ))
    })
}

pub fn write_aurora_workspace_file(
    root: &Path,
    relative_path: &str,
    bytes: Option<&[u8]>,
) -> AppResult<Option<String>> {
    let path = safe_aurora_workspace_path(root, relative_path)?;
    let backup = if path.is_file() {
        let previous = fs::read(&path).map_err(|error| {
            Box::new(AppError::io(
                "backup Aurora synchronization file",
                path.display().to_string(),
                &error,
            ))
        })?;
        let digest = sha256_bytes(&previous);
        let backup_path = root
            .join(".opennever-backups")
            .join(&digest[..16])
            .join(relative_path);
        if !backup_path.is_file() {
            atomic_write(&backup_path, &previous)?;
        }
        Some(backup_path.display().to_string())
    } else {
        None
    };
    match bytes {
        Some(bytes) => atomic_write(&path, bytes)?,
        None if path.is_file() => fs::remove_file(&path).map_err(|error| {
            Box::new(AppError::io(
                "remove synchronized Aurora file",
                path.display().to_string(),
                &error,
            ))
        })?,
        None => {}
    }
    Ok(backup)
}

fn safe_aurora_workspace_path(root: &Path, relative_path: &str) -> AppResult<PathBuf> {
    resource_key_from_aurora_path(relative_path)?;
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        Box::new(AppError::io(
            "canonicalize Aurora workspace",
            root.display().to_string(),
            &error,
        ))
    })?;
    let relative = Path::new(relative_path);
    let mut current = canonical_root.clone();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(edit_error("EDIT_AURORA_SYNC_PATH_INVALID", relative_path));
        };
        current.push(component);
        if current.exists()
            && fs::symlink_metadata(&current)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(true)
        {
            return Err(edit_error(
                "EDIT_AURORA_SYNC_SYMLINK_REJECTED",
                current.display().to_string(),
            ));
        }
    }
    if !current.starts_with(&canonical_root) {
        return Err(edit_error("EDIT_AURORA_SYNC_PATH_INVALID", relative_path));
    }
    Ok(current)
}

fn canonical_toolset_root(root: &Path) -> AppResult<String> {
    if !root.is_dir() {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_ROOT_INVALID",
            format!("{} is not a directory", root.display()),
        ));
    }
    fs::canonicalize(root)
        .map(|path| path.display().to_string())
        .map_err(|error| {
            Box::new(AppError::io(
                "canonicalize Aurora workspace",
                root.display().to_string(),
                &error,
            ))
        })
}

fn collect_aurora_files(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    depth: usize,
) -> AppResult<()> {
    if depth > 128 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_DEPTH_LIMIT",
            "Aurora workspace exceeds the directory depth limit of 128",
        ));
    }
    if paths.len() > 10_000 {
        return Err(edit_error(
            "EDIT_AURORA_SYNC_FILE_LIMIT",
            "Aurora workspace contains more than 10000 supported files",
        ));
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        Box::new(AppError::io(
            "scan Aurora workspace",
            directory.display().to_string(),
            &error,
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Box::new(AppError::io(
                "read Aurora workspace entry",
                directory.display().to_string(),
                &error,
            ))
        })?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            Box::new(AppError::io(
                "inspect Aurora workspace entry",
                entry.path().display().to_string(),
                &error,
            ))
        })?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        if metadata.is_dir()
            && entry.file_name() != ".opennever-backups"
            && entry.file_name() != ".git"
        {
            collect_aurora_files(root, &path, paths, depth + 1)?;
        } else if metadata.is_file() && supported_aurora_extension(&path) {
            if !path.starts_with(root) {
                return Err(edit_error(
                    "EDIT_AURORA_SYNC_PATH_INVALID",
                    path.display().to_string(),
                ));
            }
            if paths.len() >= 10_000 {
                return Err(edit_error(
                    "EDIT_AURORA_SYNC_FILE_LIMIT",
                    "Aurora workspace contains more than 10000 supported files",
                ));
            }
            paths.push(path);
        }
    }
    Ok(())
}

fn supported_aurora_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .and_then(resource_type_for_extension)
        .is_some_and(|resource_type| {
            !matches!(resource_type, 2011 | 2061 | 2062 | 9997 | 9998 | 9999)
        })
}

fn validate_dependency_name(value: &str, extension: &str) -> AppResult<()> {
    let path = Path::new(value);
    if value.is_empty()
        || path.components().count() != 1
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(edit_error(
            "EDIT_DEPENDENCY_NAME_INVALID",
            format!("{value:?} is not a safe {extension} dependency name"),
        ));
    }
    Ok(())
}

pub fn create_area_resources(
    resref: &str,
    name: &str,
    tileset: &str,
    width: u32,
    height: u32,
    tile_id: u32,
) -> AppResult<Vec<ErfResourceInput>> {
    validate_resref(resref)?;
    if width == 0 || height == 0 || width > MAP_MAX_WIDTH || height > MAP_MAX_HEIGHT {
        return Err(edit_error(
            "EDIT_AREA_DIMENSIONS_INVALID",
            format!(
                "area dimensions {width}x{height} exceed the compatible {MAP_MAX_WIDTH}x{MAP_MAX_HEIGHT} limit"
            ),
        ));
    }
    let tile_count = width
        .checked_mul(height)
        .ok_or_else(|| edit_error("EDIT_AREA_DIMENSIONS_INVALID", "area tile count overflow"))?;
    let tiles = (0..tile_count)
        .map(|index| GenericStruct {
            index: index + 1,
            struct_type: 1,
            fields: vec![
                gff_field("Tile_ID", 5, GenericValue::Int(tile_id as i32)),
                gff_field("Tile_Orientation", 5, GenericValue::Int(0)),
                gff_field("Tile_Height", 5, GenericValue::Int(0)),
                gff_field("Tile_MainLight1", 0, GenericValue::Byte(0)),
                gff_field("Tile_MainLight2", 0, GenericValue::Byte(0)),
                gff_field("Tile_SrcLight1", 0, GenericValue::Byte(0)),
                gff_field("Tile_SrcLight2", 0, GenericValue::Byte(0)),
                gff_field("Tile_AnimLoop1", 0, GenericValue::Byte(1)),
                gff_field("Tile_AnimLoop2", 0, GenericValue::Byte(1)),
                gff_field("Tile_AnimLoop3", 0, GenericValue::Byte(1)),
            ],
        })
        .collect();
    let are = gff_document(
        "ARE ",
        resref,
        vec![
            gff_field("ID", 5, GenericValue::Int(-1)),
            gff_field("Creator_ID", 5, GenericValue::Int(-1)),
            gff_field("Version", 4, GenericValue::Dword(18)),
            gff_field("Tag", 10, GenericValue::String(resref.to_owned())),
            gff_field("Name", 12, localized(name)),
            gff_field("ResRef", 11, GenericValue::ResRef(resref.to_owned())),
            gff_field("Comments", 10, GenericValue::String(String::new())),
            gff_field("Expansion_List", 15, GenericValue::List(Vec::new())),
            gff_field("Flags", 4, GenericValue::Dword(1)),
            gff_field("ModSpotCheck", 5, GenericValue::Int(0)),
            gff_field("ModListenCheck", 5, GenericValue::Int(0)),
            gff_field("MoonAmbientColor", 4, GenericValue::Dword(0)),
            gff_field("MoonDiffuseColor", 4, GenericValue::Dword(0)),
            gff_field("MoonFogAmount", 0, GenericValue::Byte(0)),
            gff_field("MoonFogColor", 4, GenericValue::Dword(0)),
            gff_field("MoonShadows", 0, GenericValue::Byte(0)),
            gff_field("SunAmbientColor", 4, GenericValue::Dword(0)),
            gff_field("SunDiffuseColor", 4, GenericValue::Dword(0)),
            gff_field("SunFogAmount", 0, GenericValue::Byte(0)),
            gff_field("SunFogColor", 4, GenericValue::Dword(0)),
            gff_field("SunShadows", 0, GenericValue::Byte(0)),
            gff_field("IsNight", 0, GenericValue::Byte(0)),
            gff_field("LightingScheme", 0, GenericValue::Byte(0)),
            gff_field("ShadowOpacity", 0, GenericValue::Byte(60)),
            gff_field("FogClipDist", 8, GenericValue::Float(45.0)),
            gff_field("SkyBox", 0, GenericValue::Byte(0)),
            gff_field("DayNightCycle", 0, GenericValue::Byte(1)),
            gff_field("ChanceRain", 5, GenericValue::Int(0)),
            gff_field("ChanceSnow", 5, GenericValue::Int(0)),
            gff_field("ChanceLightning", 5, GenericValue::Int(0)),
            gff_field("WindPower", 5, GenericValue::Int(0)),
            gff_field("LoadScreenID", 2, GenericValue::Word(0)),
            gff_field("PlayerVsPlayer", 0, GenericValue::Byte(0)),
            gff_field("NoRest", 0, GenericValue::Byte(0)),
            gff_field("Width", 5, GenericValue::Int(width as i32)),
            gff_field("Height", 5, GenericValue::Int(height as i32)),
            empty_resref_field("OnEnter"),
            empty_resref_field("OnExit"),
            empty_resref_field("OnHeartbeat"),
            empty_resref_field("OnUserDefined"),
            gff_field("Tileset", 11, GenericValue::ResRef(tileset.to_owned())),
            gff_field("Tile_List", 15, GenericValue::List(tiles)),
        ],
    );
    let instance_lists = || {
        [
            "Creature List",
            "Door List",
            "Encounter List",
            "List",
            "Placeable List",
            "SoundList",
            "StoreList",
            "TriggerList",
            "WaypointList",
        ]
        .into_iter()
        .map(|label| gff_field(label, 15, GenericValue::List(Vec::new())))
        .collect::<Vec<_>>()
    };
    let mut git_fields = vec![gff_field(
        "AreaProperties",
        14,
        GenericValue::Struct(Box::new(GenericStruct {
            index: 1,
            struct_type: 100,
            fields: vec![
                gff_field("AmbientSndDay", 5, GenericValue::Int(0)),
                gff_field("AmbientSndNight", 5, GenericValue::Int(0)),
                gff_field("AmbientSndDayVol", 5, GenericValue::Int(32)),
                gff_field("AmbientSndNitVol", 5, GenericValue::Int(32)),
                gff_field("EnvAudio", 5, GenericValue::Int(0)),
                gff_field("MusicBattle", 5, GenericValue::Int(0)),
                gff_field("MusicDay", 5, GenericValue::Int(0)),
                gff_field("MusicNight", 5, GenericValue::Int(0)),
                gff_field("MusicDelay", 5, GenericValue::Int(0)),
            ],
        })),
    )];
    git_fields.extend(instance_lists());
    let git = gff_document("GIT ", resref, git_fields);
    let gic = gff_document("GIC ", resref, instance_lists());
    Ok(vec![
        ErfResourceInput {
            key: ResourceKey::new(resref, 2012),
            bytes: write_gff(&are)?,
        },
        ErfResourceInput {
            key: ResourceKey::new(resref, 2023),
            bytes: write_gff(&git)?,
        },
        ErfResourceInput {
            key: ResourceKey::new(resref, 2046),
            bytes: write_gff(&gic)?,
        },
    ])
}

pub fn create_dialogue_resource(
    resref: &str,
    owner_tag: &str,
    purpose: &str,
    required_nodes: &[String],
) -> AppResult<ErfResourceInput> {
    validate_resref(resref)?;
    if owner_tag.len() > 64 || purpose.len() > 4_096 || required_nodes.len() > 10_000 {
        return Err(edit_error(
            "EDIT_DIALOGUE_DEFINITION_INVALID",
            "dialogue metadata or node count exceeds the supported bounds",
        ));
    }
    let entries = required_nodes
        .iter()
        .enumerate()
        .map(|(index, text)| GenericStruct {
            index: index as u32 + 1,
            struct_type: 0,
            fields: vec![
                gff_field("Text", 12, localized(text)),
                gff_field("Speaker", 10, GenericValue::String(owner_tag.to_owned())),
                gff_field("RepliesList", 15, GenericValue::List(Vec::new())),
            ],
        })
        .collect::<Vec<_>>();
    let starts = (0..entries.len())
        .map(|index| GenericStruct {
            index: index as u32 + 1,
            struct_type: 0,
            fields: vec![gff_field("Index", 4, GenericValue::Dword(index as u32))],
        })
        .collect::<Vec<_>>();
    let dialogue = gff_document(
        "DLG ",
        resref,
        vec![
            gff_field("EntryList", 15, GenericValue::List(entries)),
            gff_field("ReplyList", 15, GenericValue::List(Vec::new())),
            gff_field("StartingList", 15, GenericValue::List(starts)),
            gff_field("DelayEntry", 4, GenericValue::Dword(0)),
            gff_field("DelayReply", 4, GenericValue::Dword(0)),
            empty_resref_field("EndConverAbort"),
            empty_resref_field("EndConversation"),
            gff_field("NumWords", 4, GenericValue::Dword(0)),
            gff_field("PreventZoomIn", 0, GenericValue::Byte(0)),
            gff_field("Comments", 10, GenericValue::String(purpose.to_owned())),
        ],
    );
    Ok(ErfResourceInput {
        key: ResourceKey::new(resref, 2029),
        bytes: write_gff(&dialogue)?,
    })
}

fn new_module_resources(definition: &NewModuleDefinition) -> AppResult<Vec<ErfResourceInput>> {
    let mut module_id_hasher = Sha256::new();
    module_id_hasher.update(definition.name.as_bytes());
    module_id_hasher.update([0]);
    module_id_hasher.update(definition.tag.as_bytes());
    module_id_hasher.update([0]);
    module_id_hasher.update(definition.entry_area.as_bytes());
    module_id_hasher.update([0]);
    module_id_hasher.update(definition.tileset.as_bytes());
    let module_id = module_id_hasher.finalize()[..16].to_vec();
    let module_scripts = [
        "Mod_OnHeartbeat",
        "Mod_OnModLoad",
        "Mod_OnModStart",
        "Mod_OnClientEntr",
        "Mod_OnClientLeav",
        "Mod_OnActvtItem",
        "Mod_OnAcquirItem",
        "Mod_OnUsrDefined",
        "Mod_OnUnAqreItem",
        "Mod_OnPlrDeath",
        "Mod_OnPlrDying",
        "Mod_OnPlrEqItm",
        "Mod_OnPlrLvlUp",
        "Mod_OnSpawnBtnDn",
        "Mod_OnPlrRest",
        "Mod_OnPlrUnEqItm",
        "Mod_OnCutsnAbort",
        "Mod_StartMovie",
    ];
    let mut fields = vec![
        gff_field("Mod_ID", 13, GenericValue::Void(module_id)),
        gff_field(
            "Mod_MinGameVer",
            10,
            GenericValue::String("1.69".to_owned()),
        ),
        gff_field("Mod_Creator_ID", 5, GenericValue::Int(2)),
        gff_field("Mod_Version", 4, GenericValue::Dword(3)),
        gff_field("Expansion_Pack", 2, GenericValue::Word(0)),
        gff_field("Mod_Name", 12, localized(&definition.name)),
        gff_field("Mod_Tag", 10, GenericValue::String(definition.tag.clone())),
        gff_field("Mod_Description", 12, localized("")),
        gff_field("Mod_IsSaveGame", 0, GenericValue::Byte(0)),
        gff_field("Mod_CustomTlk", 10, GenericValue::String(String::new())),
        gff_field(
            "Mod_Entry_Area",
            11,
            GenericValue::ResRef(definition.entry_area.clone()),
        ),
        gff_field("Mod_Entry_X", 8, GenericValue::Float(5.0)),
        gff_field("Mod_Entry_Y", 8, GenericValue::Float(5.0)),
        gff_field("Mod_Entry_Z", 8, GenericValue::Float(0.0)),
        gff_field("Mod_Entry_Dir_X", 8, GenericValue::Float(0.0)),
        gff_field("Mod_Entry_Dir_Y", 8, GenericValue::Float(1.0)),
        gff_field("Mod_Expan_List", 15, GenericValue::List(Vec::new())),
        gff_field("Mod_DawnHour", 0, GenericValue::Byte(6)),
        gff_field("Mod_DuskHour", 0, GenericValue::Byte(18)),
        gff_field("Mod_MinPerHour", 0, GenericValue::Byte(2)),
        gff_field("Mod_StartMonth", 0, GenericValue::Byte(1)),
        gff_field("Mod_StartDay", 0, GenericValue::Byte(1)),
        gff_field("Mod_StartHour", 0, GenericValue::Byte(12)),
        gff_field("Mod_StartYear", 4, GenericValue::Dword(1372)),
        gff_field("Mod_XPScale", 0, GenericValue::Byte(10)),
    ];
    fields.extend(module_scripts.into_iter().map(empty_resref_field));
    fields.extend([
        gff_field("Mod_CutSceneList", 15, GenericValue::List(Vec::new())),
        gff_field("Mod_GVar_List", 15, GenericValue::List(Vec::new())),
        gff_field(
            "Mod_Area_list",
            15,
            GenericValue::List(vec![GenericStruct {
                index: 1,
                struct_type: 6,
                fields: vec![gff_field(
                    "Area_Name",
                    11,
                    GenericValue::ResRef(definition.entry_area.clone()),
                )],
            }]),
        ),
        gff_field("Mod_HakList", 15, GenericValue::List(Vec::new())),
        gff_field("Mod_CacheNSSList", 15, GenericValue::List(Vec::new())),
    ]);
    let ifo = gff_document("IFO ", "module", fields);
    let mut resources = vec![ErfResourceInput {
        key: ResourceKey::new("module", 2014),
        bytes: write_gff(&ifo)?,
    }];
    resources.extend(create_area_resources(
        &definition.entry_area,
        &definition.name,
        &definition.tileset,
        1,
        1,
        0,
    )?);
    Ok(resources)
}

fn gff_document(file_type: &str, source: &str, fields: Vec<GenericField>) -> GenericGff {
    GenericGff {
        file_type: file_type.to_owned(),
        file_version: "V3.2".to_owned(),
        source: format!("new::{source}"),
        struct_count: 1,
        field_count: fields.len() as u32,
        root: GenericStruct {
            index: 0,
            struct_type: u32::MAX,
            fields,
        },
    }
}

fn gff_field(label: &str, field_type: u32, value: GenericValue) -> GenericField {
    GenericField {
        label: label.to_owned(),
        field_type,
        value,
    }
}

fn empty_resref_field(label: &str) -> GenericField {
    gff_field(label, 11, GenericValue::ResRef(String::new()))
}

fn localized(text: &str) -> GenericValue {
    GenericValue::LocalizedString(LocalizedString {
        string_ref: None,
        values: vec![LocalizedValue {
            language_id: 0,
            text: text.to_owned(),
        }],
    })
}

fn validate_resref(value: &str) -> AppResult<()> {
    if value.is_empty()
        || value.len() > 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(edit_error(
            "EDIT_RESREF_INVALID",
            format!("{value:?} must match [a-z0-9_] and contain 1..=16 bytes"),
        ));
    }
    Ok(())
}

pub fn edit_gff_field(
    bytes: &[u8],
    source: &str,
    path: &str,
    before: &Value,
    after: &Value,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Err(edit_error(
            "EDIT_GFF_PATH_INVALID",
            "GFF field path must contain at least one label",
        ));
    }
    let field = find_gff_field_mut(&mut document.root, &segments)?;
    let current = serde_json::to_value(&field.value).map_err(|error| {
        edit_error(
            "EDIT_GFF_VALUE_SERIALIZE_FAILED",
            format!("cannot serialize current GFF value: {error}"),
        )
    })?;
    if &current != before {
        return Err(edit_error(
            "EDIT_GFF_PRECONDITION_FAILED",
            format!("field {path} no longer has the previewed value"),
        ));
    }
    let replacement = serde_json::from_value::<GenericValue>(after.clone()).map_err(|error| {
        edit_error(
            "EDIT_GFF_VALUE_INVALID",
            format!("field {path} replacement is not a typed GFF value: {error}"),
        )
    })?;
    if let GenericValue::ResRef(value) = &replacement {
        validate_resref(value)?;
    }
    field.value = replacement;
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

const MAX_DIALOGUE_NODES: usize = 100_000;
const MAX_DIALOGUE_LINKS: usize = 250_000;

pub fn edit_dialogue_structure(
    bytes: &[u8],
    source: &str,
    action: &DialogueStructureAction,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    if document.file_type != "DLG " {
        return Err(edit_error(
            "EDIT_DIALOGUE_FORMAT_INVALID",
            format!(
                "{source} declares GFF type {:?}, expected DLG",
                document.file_type
            ),
        ));
    }
    match action {
        DialogueStructureAction::AddNode { node_kind } => {
            let nodes = dialogue_nodes_mut(&mut document.root, *node_kind)?;
            if nodes.len() >= MAX_DIALOGUE_NODES {
                return Err(edit_error(
                    "EDIT_DIALOGUE_NODE_LIMIT_EXCEEDED",
                    format!("a dialogue accepts at most {MAX_DIALOGUE_NODES} nodes of each kind"),
                ));
            }
            nodes.push(new_dialogue_node(*node_kind, nodes.len()));
        }
        DialogueStructureAction::RemoveNode { node } => {
            remove_dialogue_node(&mut document.root, *node)?;
        }
        DialogueStructureAction::AddLink { source, target } => {
            add_dialogue_link(&mut document.root, *source, *target)?;
        }
        DialogueStructureAction::SetLinkScripts {
            source,
            position,
            condition_script,
            action_script,
        } => {
            set_dialogue_link_scripts(
                &mut document.root,
                *source,
                *position,
                condition_script.as_deref(),
                action_script.as_deref(),
            )?;
        }
        DialogueStructureAction::RemoveLink { source, position } => {
            let links = dialogue_links_mut(&mut document.root, *source)?;
            if *position >= links.len() {
                return Err(edit_error(
                    "EDIT_DIALOGUE_LINK_NOT_FOUND",
                    format!(
                        "link position {position} is outside a list of {} links",
                        links.len()
                    ),
                ));
            }
            links.remove(*position);
        }
    }
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

fn dialogue_list_labels(kind: DialogueNodeKind) -> &'static [&'static str] {
    match kind {
        DialogueNodeKind::Entry => &["EntryList", "EntriesList"],
        DialogueNodeKind::Reply => &["ReplyList", "RepliesList"],
    }
}

fn dialogue_outgoing_labels(kind: DialogueNodeKind) -> &'static [&'static str] {
    match kind {
        DialogueNodeKind::Entry => &["RepliesList", "ReplyList"],
        DialogueNodeKind::Reply => &["EntriesList", "EntryList"],
    }
}

fn dialogue_nodes(root: &GenericStruct, kind: DialogueNodeKind) -> AppResult<&Vec<GenericStruct>> {
    let labels = dialogue_list_labels(kind);
    let field = find_field(root, labels).ok_or_else(|| {
        edit_error(
            "EDIT_DIALOGUE_NODE_LIST_NOT_FOUND",
            format!("dialogue has none of the node lists {labels:?}"),
        )
    })?;
    match &field.value {
        GenericValue::List(values) => Ok(values),
        _ => Err(edit_error(
            "EDIT_DIALOGUE_NODE_LIST_INVALID",
            format!("dialogue field {:?} is not a list", field.label),
        )),
    }
}

fn dialogue_nodes_mut(
    root: &mut GenericStruct,
    kind: DialogueNodeKind,
) -> AppResult<&mut Vec<GenericStruct>> {
    list_mut_any(root, dialogue_list_labels(kind))
}

fn new_dialogue_node(kind: DialogueNodeKind, index: usize) -> GenericStruct {
    GenericStruct {
        index: index as u32 + 1,
        struct_type: 0,
        fields: vec![
            gff_field(
                "Text",
                12,
                GenericValue::LocalizedString(LocalizedString {
                    string_ref: None,
                    values: vec![LocalizedValue {
                        language_id: 0,
                        text: String::new(),
                    }],
                }),
            ),
            gff_field(
                dialogue_outgoing_labels(kind)[0],
                15,
                GenericValue::List(Vec::new()),
            ),
        ],
    }
}

fn dialogue_links_mut(
    root: &mut GenericStruct,
    source: Option<DialogueNodeRef>,
) -> AppResult<&mut Vec<GenericStruct>> {
    let Some(source) = source else {
        return list_mut_any(root, &["StartingList"]);
    };
    let nodes = dialogue_nodes_mut(root, source.kind)?;
    let node = nodes.get_mut(source.index).ok_or_else(|| {
        edit_error(
            "EDIT_DIALOGUE_NODE_NOT_FOUND",
            format!(
                "dialogue has no {:?} node at index {}",
                source.kind, source.index
            ),
        )
    })?;
    list_mut_any(node, dialogue_outgoing_labels(source.kind))
}

fn add_dialogue_link(
    root: &mut GenericStruct,
    source: Option<DialogueNodeRef>,
    target: DialogueNodeRef,
) -> AppResult<()> {
    let expected_target = match source {
        None => DialogueNodeKind::Entry,
        Some(DialogueNodeRef {
            kind: DialogueNodeKind::Entry,
            ..
        }) => DialogueNodeKind::Reply,
        Some(DialogueNodeRef {
            kind: DialogueNodeKind::Reply,
            ..
        }) => DialogueNodeKind::Entry,
    };
    if target.kind != expected_target {
        return Err(edit_error(
            "EDIT_DIALOGUE_LINK_KIND_INVALID",
            format!("a link from {source:?} must target a {expected_target:?} node"),
        ));
    }
    if target.index >= dialogue_nodes(root, target.kind)?.len() {
        return Err(edit_error(
            "EDIT_DIALOGUE_LINK_TARGET_NOT_FOUND",
            format!(
                "dialogue has no {:?} node at index {}",
                target.kind, target.index
            ),
        ));
    }
    if let Some(source) = source
        && source.index >= dialogue_nodes(root, source.kind)?.len()
    {
        return Err(edit_error(
            "EDIT_DIALOGUE_NODE_NOT_FOUND",
            format!(
                "dialogue has no {:?} node at index {}",
                source.kind, source.index
            ),
        ));
    }
    let links = dialogue_links_mut(root, source)?;
    if links.len() >= MAX_DIALOGUE_LINKS {
        return Err(edit_error(
            "EDIT_DIALOGUE_LINK_LIMIT_EXCEEDED",
            format!("a dialogue link list accepts at most {MAX_DIALOGUE_LINKS} links"),
        ));
    }
    links.push(GenericStruct {
        index: links.len() as u32 + 1,
        struct_type: 0,
        fields: vec![
            gff_field("Index", 4, GenericValue::Dword(target.index as u32)),
            gff_field("IsChild", 0, GenericValue::Byte(0)),
        ],
    });
    Ok(())
}

fn set_dialogue_link_scripts(
    root: &mut GenericStruct,
    source: Option<DialogueNodeRef>,
    position: usize,
    condition_script: Option<&str>,
    action_script: Option<&str>,
) -> AppResult<()> {
    let links = dialogue_links_mut(root, source)?;
    let link_count = links.len();
    let link = links.get_mut(position).ok_or_else(|| {
        edit_error(
            "EDIT_DIALOGUE_LINK_NOT_FOUND",
            format!(
                "link position {position} is outside a list of {} links",
                link_count
            ),
        )
    })?;
    set_optional_dialogue_resref(link, &["Active", "Conditional"], "Active", condition_script)?;
    set_optional_dialogue_resref(link, &["Script", "ActionScript"], "Script", action_script)?;
    Ok(())
}

fn set_optional_dialogue_resref(
    structure: &mut GenericStruct,
    labels: &[&str],
    preferred_label: &str,
    value: Option<&str>,
) -> AppResult<()> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if let Some(value) = value {
        validate_resref(value)?;
        if let Some(field) = structure
            .fields
            .iter_mut()
            .find(|field| labels.iter().any(|label| field.label == *label))
        {
            field.value = GenericValue::ResRef(value.to_owned());
        } else {
            structure.fields.push(gff_field(
                preferred_label,
                11,
                GenericValue::ResRef(value.to_owned()),
            ));
        }
    } else {
        structure
            .fields
            .retain(|field| !labels.iter().any(|label| field.label == *label));
    }
    Ok(())
}

fn remove_dialogue_node(root: &mut GenericStruct, node: DialogueNodeRef) -> AppResult<()> {
    let nodes = dialogue_nodes_mut(root, node.kind)?;
    if node.index >= nodes.len() {
        return Err(edit_error(
            "EDIT_DIALOGUE_NODE_NOT_FOUND",
            format!(
                "dialogue has no {:?} node at index {}",
                node.kind, node.index
            ),
        ));
    }
    nodes.remove(node.index);
    match node.kind {
        DialogueNodeKind::Entry => {
            retarget_dialogue_links(root, &["StartingList"], node.index)?;
            let replies = dialogue_nodes_mut(root, DialogueNodeKind::Reply)?;
            for reply in replies {
                retarget_dialogue_links(
                    reply,
                    dialogue_outgoing_labels(DialogueNodeKind::Reply),
                    node.index,
                )?;
            }
        }
        DialogueNodeKind::Reply => {
            let entries = dialogue_nodes_mut(root, DialogueNodeKind::Entry)?;
            for entry in entries {
                retarget_dialogue_links(
                    entry,
                    dialogue_outgoing_labels(DialogueNodeKind::Entry),
                    node.index,
                )?;
            }
        }
    }
    Ok(())
}

fn retarget_dialogue_links(
    structure: &mut GenericStruct,
    labels: &[&str],
    removed_index: usize,
) -> AppResult<()> {
    let Some(field) = structure
        .fields
        .iter_mut()
        .find(|field| labels.iter().any(|label| field.label == *label))
    else {
        return Ok(());
    };
    let GenericValue::List(links) = &mut field.value else {
        return Err(edit_error(
            "EDIT_DIALOGUE_LINK_LIST_INVALID",
            format!("dialogue field {:?} is not a list", field.label),
        ));
    };
    let mut updated = Vec::with_capacity(links.len());
    for mut link in std::mem::take(links) {
        let target = find_field(&link, &["Index"])
            .and_then(|field| integer(&field.value))
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                edit_error(
                    "EDIT_DIALOGUE_LINK_INDEX_INVALID",
                    "dialogue link has no valid unsigned Index field",
                )
            })?;
        if target == removed_index {
            continue;
        }
        if target > removed_index {
            set_integer(&mut link, &["Index"], (target - 1) as i64)?;
        }
        updated.push(link);
    }
    *links = updated;
    Ok(())
}

pub fn edit_journal_structure(
    bytes: &[u8],
    source: &str,
    action: &JournalStructureAction,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    if document.file_type != "JRL " {
        return Err(edit_error(
            "EDIT_JOURNAL_FORMAT_INVALID",
            format!(
                "{source} declares GFF type {:?}, expected JRL",
                document.file_type
            ),
        ));
    }
    match action {
        JournalStructureAction::AddCategory { tag } => {
            validate_journal_tag(tag)?;
            let categories = list_mut_any(&mut document.root, &["Categories", "CategoryList"])?;
            if categories.len() >= MAX_DIALOGUE_NODES {
                return Err(edit_error(
                    "EDIT_JOURNAL_CATEGORY_LIMIT_EXCEEDED",
                    "journal category limit exceeded",
                ));
            }
            if categories.iter().any(|category| {
                find_field(category, &["Tag"])
                    .and_then(|field| match &field.value {
                        GenericValue::String(value) => Some(value.eq_ignore_ascii_case(tag)),
                        _ => None,
                    })
                    .unwrap_or(false)
            }) {
                return Err(edit_error(
                    "EDIT_JOURNAL_TAG_DUPLICATE",
                    format!("journal already contains category tag {tag:?}"),
                ));
            }
            categories.push(new_journal_category(tag, categories.len()));
        }
        JournalStructureAction::RemoveCategory { category_index } => {
            let categories = list_mut_any(&mut document.root, &["Categories", "CategoryList"])?;
            if *category_index >= categories.len() {
                return Err(edit_error(
                    "EDIT_JOURNAL_CATEGORY_NOT_FOUND",
                    format!("journal has no category at index {category_index}"),
                ));
            }
            categories.remove(*category_index);
        }
        JournalStructureAction::AddEntry { category_index } => {
            let category = journal_category_mut(&mut document.root, *category_index)?;
            let next_id = journal_next_entry_id(category)?;
            let entries = list_mut_any(category, &["EntryList", "Entries"])?;
            if entries.len() >= MAX_DIALOGUE_NODES {
                return Err(edit_error(
                    "EDIT_JOURNAL_ENTRY_LIMIT_EXCEEDED",
                    "journal entry limit exceeded",
                ));
            }
            entries.push(new_journal_entry(next_id, entries.len()));
        }
        JournalStructureAction::RemoveEntry {
            category_index,
            entry_index,
        } => {
            let category = journal_category_mut(&mut document.root, *category_index)?;
            let entries = list_mut_any(category, &["EntryList", "Entries"])?;
            if *entry_index >= entries.len() {
                return Err(edit_error(
                    "EDIT_JOURNAL_ENTRY_NOT_FOUND",
                    format!(
                        "journal category {category_index} has no entry at index {entry_index}"
                    ),
                ));
            }
            entries.remove(*entry_index);
        }
    }
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

const MAX_FACTIONS: usize = 65_535;
const MAX_FACTION_NAME_BYTES: usize = 255;

pub fn edit_faction_structure(
    bytes: &[u8],
    source: &str,
    action: &FactionStructureAction,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    if document.file_type != "FAC " {
        return Err(edit_error(
            "EDIT_FACTION_FORMAT_INVALID",
            format!(
                "{source} declares GFF type {:?}, expected FAC",
                document.file_type
            ),
        ));
    }
    match action {
        FactionStructureAction::AddFaction { name, parent_id } => {
            let name = name.trim();
            validate_faction_name(name)?;
            let faction_count = faction_list(&document.root)?.len();
            if faction_count >= MAX_FACTIONS {
                return Err(edit_error(
                    "EDIT_FACTION_LIMIT_EXCEEDED",
                    format!("a FAC resource accepts at most {MAX_FACTIONS} factions"),
                ));
            }
            if let Some(parent_id) = parent_id
                && usize::try_from(*parent_id).map_or(true, |index| index >= faction_count)
            {
                return Err(edit_error(
                    "EDIT_FACTION_PARENT_NOT_FOUND",
                    format!("FAC has no parent faction with id {parent_id}"),
                ));
            }
            if faction_list(&document.root)?.iter().any(|faction| {
                faction_name(faction).is_some_and(|existing| existing.eq_ignore_ascii_case(name))
            }) {
                return Err(edit_error(
                    "EDIT_FACTION_NAME_DUPLICATE",
                    format!("FAC already contains faction {name:?}"),
                ));
            }
            let new_id = u32::try_from(faction_count).map_err(|_| {
                edit_error(
                    "EDIT_FACTION_LIMIT_EXCEEDED",
                    "faction identifier no longer fits the FAC format",
                )
            })?;
            faction_list_mut(&mut document.root)?.push(new_faction(
                name,
                parent_id.unwrap_or(u32::MAX),
                new_id,
            ));
            let reputations = reputation_list_mut(&mut document.root)?;
            for source_id in 0..new_id {
                if !has_reputation(reputations, source_id, new_id)? {
                    reputations.push(new_reputation(source_id, new_id, 50, reputations.len()));
                }
            }
            for target_id in 1..=new_id {
                if !has_reputation(reputations, new_id, target_id)? {
                    let value = if target_id == new_id { 100 } else { 50 };
                    reputations.push(new_reputation(new_id, target_id, value, reputations.len()));
                }
            }
            normalize_faction_struct_types(&mut document.root)?;
        }
        FactionStructureAction::RemoveFaction { faction_index } => {
            remove_faction(&mut document.root, *faction_index)?;
        }
        FactionStructureAction::AddReputation {
            source_id,
            target_id,
            value,
        } => {
            validate_reputation(&document.root, *source_id, *target_id, *value)?;
            let reputations = reputation_list_mut(&mut document.root)?;
            if has_reputation(reputations, *source_id, *target_id)? {
                return Err(edit_error(
                    "EDIT_FACTION_REPUTATION_DUPLICATE",
                    format!("reputation {source_id} -> {target_id} already exists"),
                ));
            }
            reputations.push(new_reputation(
                *source_id,
                *target_id,
                *value,
                reputations.len(),
            ));
            normalize_reputation_struct_types(reputations);
        }
        FactionStructureAction::RemoveReputation { reputation_index } => {
            let reputations = reputation_list_mut(&mut document.root)?;
            if *reputation_index >= reputations.len() {
                return Err(edit_error(
                    "EDIT_FACTION_REPUTATION_NOT_FOUND",
                    format!(
                        "FAC has no reputation at index {reputation_index}; it contains {}",
                        reputations.len()
                    ),
                ));
            }
            reputations.remove(*reputation_index);
            normalize_reputation_struct_types(reputations);
        }
    }
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

fn validate_faction_name(name: &str) -> AppResult<()> {
    if name.is_empty() || name.len() > MAX_FACTION_NAME_BYTES || name.contains(['\0', '\r', '\n']) {
        return Err(edit_error(
            "EDIT_FACTION_NAME_INVALID",
            format!(
                "faction name must contain 1..={MAX_FACTION_NAME_BYTES} bytes without NUL or line breaks"
            ),
        ));
    }
    Ok(())
}

fn faction_list(root: &GenericStruct) -> AppResult<&Vec<GenericStruct>> {
    let field = find_field(root, &["FactionList", "Factions"]).ok_or_else(|| {
        edit_error(
            "EDIT_FACTION_LIST_NOT_FOUND",
            "FAC has no FactionList or Factions field",
        )
    })?;
    match &field.value {
        GenericValue::List(values) => Ok(values),
        _ => Err(edit_error(
            "EDIT_FACTION_LIST_INVALID",
            format!("FAC field {:?} is not a list", field.label),
        )),
    }
}

fn faction_list_mut(root: &mut GenericStruct) -> AppResult<&mut Vec<GenericStruct>> {
    list_mut_any(root, &["FactionList", "Factions"])
}

fn reputation_list_mut(root: &mut GenericStruct) -> AppResult<&mut Vec<GenericStruct>> {
    list_mut_any(root, &["RepList", "ReputationList"])
}

fn faction_name(faction: &GenericStruct) -> Option<&str> {
    find_field(faction, &["FactionName", "Name"]).and_then(|field| match &field.value {
        GenericValue::String(value) => Some(value.as_str()),
        _ => None,
    })
}

fn new_faction(name: &str, parent_id: u32, id: u32) -> GenericStruct {
    GenericStruct {
        index: id.saturating_add(1),
        struct_type: id,
        fields: vec![
            gff_field("FactionParentID", 4, GenericValue::Dword(parent_id)),
            gff_field("FactionName", 10, GenericValue::String(name.to_owned())),
            gff_field("FactionGlobal", 2, GenericValue::Word(0)),
        ],
    }
}

fn new_reputation(source_id: u32, target_id: u32, value: u32, index: usize) -> GenericStruct {
    GenericStruct {
        index: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
        struct_type: u32::try_from(index).unwrap_or(u32::MAX),
        fields: vec![
            gff_field("FactionID1", 4, GenericValue::Dword(source_id)),
            gff_field("FactionID2", 4, GenericValue::Dword(target_id)),
            gff_field("FactionRep", 4, GenericValue::Dword(value)),
        ],
    }
}

fn reputation_ids(reputation: &GenericStruct) -> AppResult<(u32, u32)> {
    let read = |labels: &[&str], label: &str| {
        find_field(reputation, labels)
            .and_then(|field| integer(&field.value))
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                edit_error(
                    "EDIT_FACTION_REPUTATION_INVALID",
                    format!("FAC reputation has no valid unsigned {label} field"),
                )
            })
    };
    Ok((
        read(&["FactionID1", "SourceID"], "source")?,
        read(&["FactionID2", "TargetID"], "target")?,
    ))
}

fn has_reputation(
    reputations: &[GenericStruct],
    source_id: u32,
    target_id: u32,
) -> AppResult<bool> {
    for reputation in reputations {
        if reputation_ids(reputation)? == (source_id, target_id) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_reputation(
    root: &GenericStruct,
    source_id: u32,
    target_id: u32,
    value: u32,
) -> AppResult<()> {
    let faction_count = faction_list(root)?.len();
    if usize::try_from(source_id).map_or(true, |index| index >= faction_count)
        || usize::try_from(target_id).map_or(true, |index| index >= faction_count)
    {
        return Err(edit_error(
            "EDIT_FACTION_REPUTATION_ENDPOINT_NOT_FOUND",
            format!(
                "reputation {source_id} -> {target_id} references a FAC with {faction_count} factions"
            ),
        ));
    }
    if target_id == 0 {
        return Err(edit_error(
            "EDIT_FACTION_REPUTATION_PC_TARGET_INVALID",
            "the PC faction (id 0) cannot be a reputation target in an Aurora FAC matrix",
        ));
    }
    if value > 100 {
        return Err(edit_error(
            "EDIT_FACTION_REPUTATION_VALUE_INVALID",
            format!("reputation value {value} is outside 0..=100"),
        ));
    }
    Ok(())
}

fn remove_faction(root: &mut GenericStruct, faction_index: usize) -> AppResult<()> {
    if faction_index == 0 {
        return Err(edit_error(
            "EDIT_FACTION_PC_REMOVE_FORBIDDEN",
            "the PC faction at index 0 is required by the Aurora FAC format",
        ));
    }
    let faction_count = faction_list(root)?.len();
    if faction_index >= faction_count {
        return Err(edit_error(
            "EDIT_FACTION_NOT_FOUND",
            format!("FAC has no faction at index {faction_index}"),
        ));
    }
    let removed_id = u32::try_from(faction_index).map_err(|_| {
        edit_error(
            "EDIT_FACTION_NOT_FOUND",
            "faction index does not fit the FAC identifier format",
        )
    })?;
    faction_list_mut(root)?.remove(faction_index);
    for faction in faction_list_mut(root)? {
        let Some(parent) = find_field(faction, &["FactionParentID", "ParentID"])
            .and_then(|field| integer(&field.value))
            .and_then(|value| u32::try_from(value).ok())
        else {
            continue;
        };
        if parent == removed_id {
            set_integer(
                faction,
                &["FactionParentID", "ParentID"],
                i64::from(u32::MAX),
            )?;
        } else if parent != u32::MAX && parent > removed_id {
            set_integer(
                faction,
                &["FactionParentID", "ParentID"],
                i64::from(parent - 1),
            )?;
        }
    }
    let reputations = reputation_list_mut(root)?;
    let mut updated = Vec::with_capacity(reputations.len());
    for mut reputation in std::mem::take(reputations) {
        let (source_id, target_id) = reputation_ids(&reputation)?;
        if source_id == removed_id || target_id == removed_id {
            continue;
        }
        if source_id > removed_id {
            set_integer(
                &mut reputation,
                &["FactionID1", "SourceID"],
                i64::from(source_id - 1),
            )?;
        }
        if target_id > removed_id {
            set_integer(
                &mut reputation,
                &["FactionID2", "TargetID"],
                i64::from(target_id - 1),
            )?;
        }
        updated.push(reputation);
    }
    *reputations = updated;
    normalize_faction_struct_types(root)?;
    Ok(())
}

fn normalize_faction_struct_types(root: &mut GenericStruct) -> AppResult<()> {
    for (index, faction) in faction_list_mut(root)?.iter_mut().enumerate() {
        let id = u32::try_from(index).unwrap_or(u32::MAX);
        faction.struct_type = id;
        if find_field(faction, &["FactionID", "ID"]).is_some() {
            set_integer(faction, &["FactionID", "ID"], i64::from(id))?;
        }
    }
    normalize_reputation_struct_types(reputation_list_mut(root)?);
    Ok(())
}

fn normalize_reputation_struct_types(reputations: &mut [GenericStruct]) {
    for (index, reputation) in reputations.iter_mut().enumerate() {
        reputation.struct_type = u32::try_from(index).unwrap_or(u32::MAX);
    }
}

pub fn edit_blueprint_structure(
    bytes: &[u8],
    source: &str,
    action: &BlueprintStructureAction,
) -> AppResult<(Vec<u8>, GenericGff)> {
    let mut document = parse_gff(bytes, source)?;
    match action {
        BlueprintStructureAction::AddFeat { feat_id } => push_blueprint_entry(
            &mut document,
            BlueprintListKind::Feat,
            GenericStruct {
                index: 0,
                struct_type: 1,
                fields: vec![gff_field("Feat", 2, GenericValue::Word(*feat_id))],
            },
            Some(("Feat", i64::from(*feat_id))),
        )?,
        BlueprintStructureAction::AddSpecialAbility {
            spell_id,
            caster_level,
            flags,
        } => push_blueprint_entry(
            &mut document,
            BlueprintListKind::SpecialAbility,
            GenericStruct {
                index: 0,
                struct_type: 4,
                fields: vec![
                    gff_field("Spell", 2, GenericValue::Word(*spell_id)),
                    gff_field("SpellFlags", 0, GenericValue::Byte(*flags)),
                    gff_field("SpellCasterLevel", 0, GenericValue::Byte(*caster_level)),
                ],
            },
            None,
        )?,
        BlueprintStructureAction::AddClass {
            class_id,
            class_level,
        } => {
            if *class_level == 0 || *class_level > 60 {
                return Err(edit_error(
                    "EDIT_BLUEPRINT_CLASS_LEVEL_INVALID",
                    format!("class level {class_level} is outside 1..=60"),
                ));
            }
            let class_id = i32::try_from(*class_id).map_err(|_| {
                edit_error(
                    "EDIT_BLUEPRINT_CLASS_ID_INVALID",
                    format!("class id {class_id} does not fit an Aurora INT"),
                )
            })?;
            let class_level = i16::try_from(*class_level).map_err(|_| {
                edit_error(
                    "EDIT_BLUEPRINT_CLASS_LEVEL_INVALID",
                    format!("class level {class_level} does not fit an Aurora SHORT"),
                )
            })?;
            push_blueprint_entry(
                &mut document,
                BlueprintListKind::Class,
                GenericStruct {
                    index: 0,
                    struct_type: 2,
                    fields: vec![
                        gff_field("Class", 5, GenericValue::Int(class_id)),
                        gff_field("ClassLevel", 3, GenericValue::Short(class_level)),
                    ],
                },
                Some(("Class", i64::from(class_id))),
            )?;
        }
        BlueprintStructureAction::AddEquippedItem { resref, slot } => {
            validate_resref(resref)?;
            push_blueprint_entry(
                &mut document,
                BlueprintListKind::EquippedItem,
                GenericStruct {
                    index: 0,
                    struct_type: *slot,
                    fields: vec![gff_field(
                        "EquippedRes",
                        11,
                        GenericValue::ResRef(resref.clone()),
                    )],
                },
                None,
            )?;
        }
        BlueprintStructureAction::AddItemProperty {
            property_name,
            subtype,
            cost_table,
            cost_value,
            param1,
            param1_value,
            chance_appear,
        } => {
            if *chance_appear > 100 {
                return Err(edit_error(
                    "EDIT_BLUEPRINT_PROPERTY_CHANCE_INVALID",
                    format!("item property chance {chance_appear} is outside 0..=100"),
                ));
            }
            push_blueprint_entry(
                &mut document,
                BlueprintListKind::ItemProperty,
                GenericStruct {
                    index: 0,
                    struct_type: 0,
                    fields: vec![
                        gff_field("PropertyName", 2, GenericValue::Word(*property_name)),
                        gff_field("Subtype", 2, GenericValue::Word(*subtype)),
                        gff_field("CostTable", 0, GenericValue::Byte(*cost_table)),
                        gff_field("CostValue", 2, GenericValue::Word(*cost_value)),
                        gff_field("Param1", 0, GenericValue::Byte(*param1)),
                        gff_field("Param1Value", 0, GenericValue::Byte(*param1_value)),
                        gff_field("ChanceAppear", 0, GenericValue::Byte(*chance_appear)),
                    ],
                },
                None,
            )?;
        }
        BlueprintStructureAction::AddSound { resref } => {
            validate_resref(resref)?;
            push_blueprint_entry(
                &mut document,
                BlueprintListKind::Sound,
                GenericStruct {
                    index: 0,
                    struct_type: 0,
                    fields: vec![gff_field("Sound", 11, GenericValue::ResRef(resref.clone()))],
                },
                None,
            )?;
        }
        BlueprintStructureAction::AddEncounterCreature {
            resref,
            appearance,
            challenge_rating,
            single_spawn,
        } => {
            validate_resref(resref)?;
            let appearance = i32::try_from(*appearance).map_err(|_| {
                edit_error(
                    "EDIT_BLUEPRINT_APPEARANCE_INVALID",
                    format!("appearance id {appearance} does not fit an Aurora INT"),
                )
            })?;
            if !challenge_rating.is_finite() || *challenge_rating < 0.0 {
                return Err(edit_error(
                    "EDIT_BLUEPRINT_CHALLENGE_RATING_INVALID",
                    format!("challenge rating {challenge_rating} must be finite and non-negative"),
                ));
            }
            push_blueprint_entry(
                &mut document,
                BlueprintListKind::EncounterCreature,
                GenericStruct {
                    index: 0,
                    struct_type: 0,
                    fields: vec![
                        gff_field("Appearance", 5, GenericValue::Int(appearance)),
                        gff_field("CR", 8, GenericValue::Float(*challenge_rating)),
                        gff_field("ResRef", 11, GenericValue::ResRef(resref.clone())),
                        gff_field(
                            "SingleSpawn",
                            0,
                            GenericValue::Byte(u8::from(*single_spawn)),
                        ),
                    ],
                },
                None,
            )?;
        }
        BlueprintStructureAction::RemoveEntry {
            list_kind,
            entry_index,
        } => {
            validate_blueprint_list_format(&document, *list_kind)?;
            let list = blueprint_list_mut(&mut document.root, *list_kind)?;
            if *entry_index >= list.len() {
                return Err(edit_error(
                    "EDIT_BLUEPRINT_ENTRY_NOT_FOUND",
                    format!(
                        "blueprint {:?} list has no entry at index {entry_index}",
                        list_kind
                    ),
                ));
            }
            list.remove(*entry_index);
        }
    }
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

fn push_blueprint_entry(
    document: &mut GenericGff,
    kind: BlueprintListKind,
    entry: GenericStruct,
    unique_integer: Option<(&str, i64)>,
) -> AppResult<()> {
    validate_blueprint_list_format(document, kind)?;
    let list = blueprint_list_mut(&mut document.root, kind)?;
    if list.len() >= MAX_DIALOGUE_NODES {
        return Err(edit_error(
            "EDIT_BLUEPRINT_LIST_LIMIT_EXCEEDED",
            format!("blueprint {:?} list limit exceeded", kind),
        ));
    }
    if let Some((label, expected)) = unique_integer
        && list.iter().any(|entry| {
            find_field(entry, &[label]).and_then(|field| integer(&field.value)) == Some(expected)
        })
    {
        return Err(edit_error(
            "EDIT_BLUEPRINT_ENTRY_DUPLICATE",
            format!(
                "blueprint {:?} list already contains {label}={expected}",
                kind
            ),
        ));
    }
    list.push(entry);
    Ok(())
}

fn validate_blueprint_list_format(document: &GenericGff, kind: BlueprintListKind) -> AppResult<()> {
    let expected = match kind {
        BlueprintListKind::Feat
        | BlueprintListKind::SpecialAbility
        | BlueprintListKind::Class
        | BlueprintListKind::EquippedItem => "UTC ",
        BlueprintListKind::ItemProperty => "UTI ",
        BlueprintListKind::Sound => "UTS ",
        BlueprintListKind::EncounterCreature => "UTE ",
    };
    if document.file_type != expected {
        return Err(edit_error(
            "EDIT_BLUEPRINT_LIST_FORMAT_INVALID",
            format!(
                "blueprint {:?} list requires GFF type {:?}, found {:?}",
                kind, expected, document.file_type
            ),
        ));
    }
    Ok(())
}

fn blueprint_list_mut(
    root: &mut GenericStruct,
    kind: BlueprintListKind,
) -> AppResult<&mut Vec<GenericStruct>> {
    let labels: &[&str] = match kind {
        BlueprintListKind::Feat => &["FeatList"],
        BlueprintListKind::SpecialAbility => &["SpecAbilityList"],
        BlueprintListKind::Class => &["ClassList"],
        BlueprintListKind::EquippedItem => &["Equip_ItemList", "EquipItemList"],
        BlueprintListKind::ItemProperty => &["PropertiesList", "PropertyList"],
        BlueprintListKind::Sound => &["Sounds"],
        BlueprintListKind::EncounterCreature => &["CreatureList"],
    };
    list_mut_any(root, labels)
}

fn validate_journal_tag(tag: &str) -> AppResult<()> {
    if tag.is_empty()
        || tag.len() > 64
        || !tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(edit_error(
            "EDIT_JOURNAL_TAG_INVALID",
            format!("journal tag {tag:?} must contain 1..=64 ASCII letters, digits, '_' or '-'"),
        ));
    }
    Ok(())
}

fn new_journal_category(tag: &str, index: usize) -> GenericStruct {
    GenericStruct {
        index: index as u32 + 1,
        struct_type: 0,
        fields: vec![
            gff_field("Tag", 10, GenericValue::String(tag.to_owned())),
            gff_field(
                "Name",
                12,
                GenericValue::LocalizedString(LocalizedString {
                    string_ref: None,
                    values: vec![LocalizedValue {
                        language_id: 0,
                        text: tag.to_owned(),
                    }],
                }),
            ),
            gff_field("Priority", 4, GenericValue::Dword(0)),
            gff_field("XP", 4, GenericValue::Dword(0)),
            gff_field("EntryList", 15, GenericValue::List(Vec::new())),
        ],
    }
}

fn journal_category_mut(
    root: &mut GenericStruct,
    category_index: usize,
) -> AppResult<&mut GenericStruct> {
    let categories = list_mut_any(root, &["Categories", "CategoryList"])?;
    categories.get_mut(category_index).ok_or_else(|| {
        edit_error(
            "EDIT_JOURNAL_CATEGORY_NOT_FOUND",
            format!("journal has no category at index {category_index}"),
        )
    })
}

fn journal_next_entry_id(category: &GenericStruct) -> AppResult<u32> {
    let field = find_field(category, &["EntryList", "Entries"]).ok_or_else(|| {
        edit_error(
            "EDIT_JOURNAL_ENTRY_LIST_NOT_FOUND",
            "journal category has no entry list",
        )
    })?;
    let GenericValue::List(entries) = &field.value else {
        return Err(edit_error(
            "EDIT_JOURNAL_ENTRY_LIST_INVALID",
            "journal entry field is not a list",
        ));
    };
    let maximum = entries
        .iter()
        .filter_map(|entry| {
            find_field(entry, &["ID", "Id"])
                .and_then(|field| integer(&field.value))
                .and_then(|value| u32::try_from(value).ok())
        })
        .max();
    match maximum {
        Some(value) => value
            .checked_add(1)
            .ok_or_else(|| edit_error("EDIT_JOURNAL_ENTRY_ID_EXHAUSTED", "entry ID overflow")),
        None => Ok(0),
    }
}

fn new_journal_entry(id: u32, index: usize) -> GenericStruct {
    GenericStruct {
        index: index as u32 + 1,
        struct_type: 0,
        fields: vec![
            gff_field("ID", 4, GenericValue::Dword(id)),
            gff_field(
                "Text",
                12,
                GenericValue::LocalizedString(LocalizedString {
                    string_ref: None,
                    values: vec![LocalizedValue {
                        language_id: 0,
                        text: String::new(),
                    }],
                }),
            ),
            gff_field("End", 0, GenericValue::Byte(0)),
            gff_field("Delay", 4, GenericValue::Dword(0)),
        ],
    }
}

pub fn edit_area_instance(
    bytes: &[u8],
    source: &str,
    list_label: &str,
    index: usize,
    before: Transform,
    after: Transform,
) -> AppResult<Vec<u8>> {
    let mut document = parse_gff(bytes, source)?;
    let values = list_mut(&mut document.root, list_label)?;
    let instance = values.get_mut(index).ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_NOT_FOUND",
            format!("list {list_label:?} has no instance at index {index}"),
        )
    })?;
    verify_numeric(instance, &["XPosition", "X"], before.x)?;
    verify_numeric(instance, &["YPosition", "Y"], before.y)?;
    verify_numeric(instance, &["ZPosition", "Z"], before.z)?;
    verify_numeric(instance, &["Bearing", "XOrientation"], before.bearing)?;
    set_numeric(instance, &["XPosition", "X"], after.x)?;
    set_numeric(instance, &["YPosition", "Y"], after.y)?;
    set_numeric(instance, &["ZPosition", "Z"], after.z)?;
    set_numeric(instance, &["Bearing", "XOrientation"], after.bearing)?;
    write_gff(&document)
}

pub fn edit_area_instance_by_id(
    bytes: &[u8],
    source: &str,
    area: &str,
    instance_id: &str,
    before: Transform,
    after: Transform,
) -> AppResult<Vec<u8>> {
    let (list_label, index) = parse_area_instance_id(area, instance_id)?;
    edit_area_instance(bytes, source, list_label, index, before, after)
}

pub fn edit_area_tile_at(
    bytes: &[u8],
    source: &str,
    x: u32,
    y: u32,
    before: TileState,
    after: TileState,
) -> AppResult<Vec<u8>> {
    let document = parse_gff(bytes, source)?;
    let width = find_field(&document.root, &["Width"])
        .and_then(|field| integer(&field.value))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| edit_error("EDIT_AREA_DIMENSIONS_INVALID", "ARE width is missing"))?;
    let height = find_field(&document.root, &["Height"])
        .and_then(|field| integer(&field.value))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| edit_error("EDIT_AREA_DIMENSIONS_INVALID", "ARE height is missing"))?;
    if x >= width || y >= height {
        return Err(edit_error(
            "EDIT_AREA_TILE_NOT_FOUND",
            format!("tile {x},{y} is outside the {width}x{height} area"),
        ));
    }
    let index = y
        .checked_mul(width)
        .and_then(|value| value.checked_add(x))
        .ok_or_else(|| edit_error("EDIT_AREA_TILE_INDEX_OVERFLOW", "tile index overflow"))?
        as usize;
    edit_area_tile(bytes, source, index, before, after)
}

pub fn edit_area_environment(
    bytes: &[u8],
    source: &str,
    patch: &AreaEnvironmentPatch,
) -> AppResult<Vec<u8>> {
    validate_area_environment_patch(patch)?;
    let mut document = parse_gff(bytes, source)?;
    let root = &mut document.root;
    if let Some(value) = &patch.tag {
        set_or_insert_string(root, "Tag", value.clone())?;
    }
    if let Some(value) = &patch.comments {
        set_or_insert_string(root, "Comments", value.clone())?;
    }
    patch_bool(root, "DayNightCycle", patch.day_night_cycle)?;
    patch_bool(root, "IsNight", patch.is_night)?;
    patch_bool(root, "NoRest", patch.no_rest)?;
    patch_integer(
        root,
        "PlayerVsPlayer",
        patch.player_vs_player.map(i64::from),
        0,
    )?;
    patch_integer(root, "ChanceRain", patch.chance_rain.map(i64::from), 5)?;
    patch_integer(root, "ChanceSnow", patch.chance_snow.map(i64::from), 5)?;
    patch_integer(
        root,
        "ChanceLightning",
        patch.chance_lightning.map(i64::from),
        5,
    )?;
    patch_integer(root, "WindPower", patch.wind_power.map(i64::from), 5)?;
    if let Some(value) = patch.fog_clip_distance {
        set_or_insert_float(root, "FogClipDist", value)?;
    }
    patch_integer(root, "SkyBox", patch.sky_box.map(i64::from), 0)?;
    patch_integer(root, "LoadScreenID", patch.load_screen_id.map(i64::from), 2)?;
    patch_integer(
        root,
        "LightingScheme",
        patch.lighting_scheme.map(i64::from),
        0,
    )?;
    patch_integer(
        root,
        "ShadowOpacity",
        patch.shadow_opacity.map(i64::from),
        0,
    )?;
    patch_integer(
        root,
        "SunAmbientColor",
        patch.sun_ambient_color.map(i64::from),
        4,
    )?;
    patch_integer(
        root,
        "SunDiffuseColor",
        patch.sun_diffuse_color.map(i64::from),
        4,
    )?;
    patch_integer(root, "SunFogColor", patch.sun_fog_color.map(i64::from), 4)?;
    patch_integer(root, "SunFogAmount", patch.sun_fog_amount.map(i64::from), 0)?;
    patch_bool(root, "SunShadows", patch.sun_shadows)?;
    patch_integer(
        root,
        "MoonAmbientColor",
        patch.moon_ambient_color.map(i64::from),
        4,
    )?;
    patch_integer(
        root,
        "MoonDiffuseColor",
        patch.moon_diffuse_color.map(i64::from),
        4,
    )?;
    patch_integer(root, "MoonFogColor", patch.moon_fog_color.map(i64::from), 4)?;
    patch_integer(
        root,
        "MoonFogAmount",
        patch.moon_fog_amount.map(i64::from),
        0,
    )?;
    patch_bool(root, "MoonShadows", patch.moon_shadows)?;
    patch_resref(root, "OnEnter", patch.on_enter.as_deref())?;
    patch_resref(root, "OnExit", patch.on_exit.as_deref())?;
    patch_resref(root, "OnHeartbeat", patch.on_heartbeat.as_deref())?;
    patch_resref(root, "OnUserDefined", patch.on_user_defined.as_deref())?;
    write_gff(&document)
}

pub fn inspect_area_environment(bytes: &[u8], source: &str) -> AppResult<AreaEnvironmentPatch> {
    let document = parse_gff(bytes, source)?;
    let root = &document.root;
    Ok(AreaEnvironmentPatch {
        tag: read_text_field(root, "Tag"),
        comments: read_text_field(root, "Comments"),
        day_night_cycle: read_bool_field(root, "DayNightCycle"),
        is_night: read_bool_field(root, "IsNight"),
        no_rest: read_bool_field(root, "NoRest"),
        player_vs_player: read_integer_field(root, "PlayerVsPlayer").and_then(as_u8),
        chance_rain: read_integer_field(root, "ChanceRain").and_then(as_u8),
        chance_snow: read_integer_field(root, "ChanceSnow").and_then(as_u8),
        chance_lightning: read_integer_field(root, "ChanceLightning").and_then(as_u8),
        wind_power: read_integer_field(root, "WindPower").and_then(as_u8),
        fog_clip_distance: read_numeric_field(root, "FogClipDist").map(|value| value as f32),
        sky_box: read_integer_field(root, "SkyBox").and_then(as_u8),
        load_screen_id: read_integer_field(root, "LoadScreenID")
            .and_then(|value| u16::try_from(value).ok()),
        lighting_scheme: read_integer_field(root, "LightingScheme").and_then(as_u8),
        shadow_opacity: read_integer_field(root, "ShadowOpacity").and_then(as_u8),
        sun_ambient_color: read_integer_field(root, "SunAmbientColor").and_then(as_u32),
        sun_diffuse_color: read_integer_field(root, "SunDiffuseColor").and_then(as_u32),
        sun_fog_color: read_integer_field(root, "SunFogColor").and_then(as_u32),
        sun_fog_amount: read_integer_field(root, "SunFogAmount").and_then(as_u8),
        sun_shadows: read_bool_field(root, "SunShadows"),
        moon_ambient_color: read_integer_field(root, "MoonAmbientColor").and_then(as_u32),
        moon_diffuse_color: read_integer_field(root, "MoonDiffuseColor").and_then(as_u32),
        moon_fog_color: read_integer_field(root, "MoonFogColor").and_then(as_u32),
        moon_fog_amount: read_integer_field(root, "MoonFogAmount").and_then(as_u8),
        moon_shadows: read_bool_field(root, "MoonShadows"),
        on_enter: read_text_field(root, "OnEnter"),
        on_exit: read_text_field(root, "OnExit"),
        on_heartbeat: read_text_field(root, "OnHeartbeat"),
        on_user_defined: read_text_field(root, "OnUserDefined"),
    })
}

pub fn edit_area_audio(bytes: &[u8], source: &str, patch: &AreaAudioPatch) -> AppResult<Vec<u8>> {
    validate_area_audio_patch(patch)?;
    let mut document = parse_gff(bytes, source)?;
    let properties = document
        .root
        .fields
        .iter_mut()
        .find(|field| field.label == "AreaProperties")
        .and_then(|field| match &mut field.value {
            GenericValue::Struct(value) => Some(value.as_mut()),
            _ => None,
        })
        .ok_or_else(|| {
            edit_error(
                "EDIT_AREA_AUDIO_PROPERTIES_MISSING",
                "GIT AreaProperties is missing or invalid",
            )
        })?;
    patch_integer(
        properties,
        "AmbientSndDay",
        patch.ambient_sound_day.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "AmbientSndNight",
        patch.ambient_sound_night.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "AmbientSndDayVol",
        patch.ambient_sound_day_volume.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "AmbientSndNitVol",
        patch.ambient_sound_night_volume.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "EnvAudio",
        patch.environment_audio.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "MusicBattle",
        patch.music_battle.map(i64::from),
        5,
    )?;
    patch_integer(properties, "MusicDay", patch.music_day.map(i64::from), 5)?;
    patch_integer(
        properties,
        "MusicNight",
        patch.music_night.map(i64::from),
        5,
    )?;
    patch_integer(
        properties,
        "MusicDelay",
        patch.music_delay.map(i64::from),
        5,
    )?;
    write_gff(&document)
}

pub fn inspect_area_audio(bytes: &[u8], source: &str) -> AppResult<AreaAudioPatch> {
    let document = parse_gff(bytes, source)?;
    let properties = document
        .root
        .fields
        .iter()
        .find(|field| field.label == "AreaProperties")
        .and_then(|field| match &field.value {
            GenericValue::Struct(value) => Some(value.as_ref()),
            _ => None,
        })
        .ok_or_else(|| {
            edit_error(
                "EDIT_AREA_AUDIO_PROPERTIES_MISSING",
                "GIT AreaProperties is missing or invalid",
            )
        })?;
    Ok(AreaAudioPatch {
        ambient_sound_day: read_integer_field(properties, "AmbientSndDay").and_then(as_i32),
        ambient_sound_night: read_integer_field(properties, "AmbientSndNight").and_then(as_i32),
        ambient_sound_day_volume: read_integer_field(properties, "AmbientSndDayVol")
            .and_then(as_u8),
        ambient_sound_night_volume: read_integer_field(properties, "AmbientSndNitVol")
            .and_then(as_u8),
        environment_audio: read_integer_field(properties, "EnvAudio").and_then(as_i32),
        music_battle: read_integer_field(properties, "MusicBattle").and_then(as_i32),
        music_day: read_integer_field(properties, "MusicDay").and_then(as_i32),
        music_night: read_integer_field(properties, "MusicNight").and_then(as_i32),
        music_delay: read_integer_field(properties, "MusicDelay").and_then(as_i32),
    })
}

pub fn edit_area_structure(
    bytes: &[u8],
    source: &str,
    area: &str,
    action: &AreaStructureAction,
    item_template: Option<&GenericGff>,
) -> AppResult<(Vec<u8>, GenericGff)> {
    const MAX_AREA_POINTS: usize = 256;
    let mut document = parse_gff(bytes, source)?;
    match action {
        AreaStructureAction::SetGeometry {
            instance_id,
            points,
        } => {
            let (list_label, index) = parse_area_instance_id(area, instance_id)?;
            let (point_prefix, struct_type) = match list_label {
                "TriggerList" => ("Point", 3),
                "Encounter List" => ("", 1),
                _ => {
                    return Err(edit_error(
                        "EDIT_AREA_GEOMETRY_CATEGORY_INVALID",
                        format!("{list_label:?} does not support polygon geometry"),
                    ));
                }
            };
            validate_area_polygon(points, MAX_AREA_POINTS)?;
            let instance = area_instance_mut(&mut document.root, list_label, index)?;
            let values = points
                .iter()
                .enumerate()
                .map(|(index, point)| GenericStruct {
                    index: index as u32 + 1,
                    struct_type,
                    fields: vec![
                        gff_field(
                            &format!("{point_prefix}X"),
                            8,
                            GenericValue::Float(point.x as f32),
                        ),
                        gff_field(
                            &format!("{point_prefix}Y"),
                            8,
                            GenericValue::Float(point.y as f32),
                        ),
                        gff_field(
                            &format!("{point_prefix}Z"),
                            8,
                            GenericValue::Float(point.z as f32),
                        ),
                    ],
                })
                .collect();
            replace_or_insert_list(instance, "Geometry", values)?;
        }
        AreaStructureAction::SetSpawnPoints {
            instance_id,
            points,
        } => {
            let (list_label, index) = parse_area_instance_id(area, instance_id)?;
            if list_label != "Encounter List" {
                return Err(edit_error(
                    "EDIT_AREA_SPAWN_CATEGORY_INVALID",
                    format!("{list_label:?} does not support encounter spawn points"),
                ));
            }
            if points.len() > MAX_AREA_POINTS
                || points.iter().any(|point| {
                    !point.x.is_finite()
                        || !point.y.is_finite()
                        || !point.z.is_finite()
                        || !point.orientation.is_finite()
                })
            {
                return Err(edit_error(
                    "EDIT_AREA_SPAWN_POINTS_INVALID",
                    format!(
                        "spawn point list must contain at most {MAX_AREA_POINTS} finite points"
                    ),
                ));
            }
            let instance = area_instance_mut(&mut document.root, list_label, index)?;
            let values = points
                .iter()
                .enumerate()
                .map(|(index, point)| GenericStruct {
                    index: index as u32 + 1,
                    struct_type: 2,
                    fields: vec![
                        gff_field("X", 8, GenericValue::Float(point.x as f32)),
                        gff_field("Y", 8, GenericValue::Float(point.y as f32)),
                        gff_field("Z", 8, GenericValue::Float(point.z as f32)),
                        gff_field(
                            "Orientation",
                            8,
                            GenericValue::Float(point.orientation as f32),
                        ),
                    ],
                })
                .collect();
            replace_or_insert_list(instance, "SpawnPointList", values)?;
        }
        AreaStructureAction::SetTransition {
            instance_id,
            destination,
            flags,
            load_screen_id,
        } => {
            let (list_label, index) = parse_area_instance_id(area, instance_id)?;
            if !matches!(list_label, "Door List" | "TriggerList") {
                return Err(edit_error(
                    "EDIT_AREA_TRANSITION_CATEGORY_INVALID",
                    format!("{list_label:?} does not support transitions"),
                ));
            }
            if destination.len() > 64
                || destination
                    .chars()
                    .any(|character| character == '\0' || character.is_control())
            {
                return Err(edit_error(
                    "EDIT_AREA_TRANSITION_DESTINATION_INVALID",
                    "transition destination must contain at most 64 printable characters",
                ));
            }
            let instance = area_instance_mut(&mut document.root, list_label, index)?;
            set_or_insert_string(instance, "LinkedTo", destination.clone())?;
            set_or_insert_integer(instance, "LinkedToFlags", i64::from(*flags), 0)?;
            set_or_insert_integer(instance, "LoadScreenID", i64::from(*load_screen_id), 2)?;
        }
        AreaStructureAction::AddInventoryItem {
            instance_id,
            resref,
            stack_size,
            x,
            y,
            infinite,
            category_index,
        } => {
            validate_resref(resref)?;
            if *stack_size == 0 {
                return Err(edit_error(
                    "EDIT_AREA_INVENTORY_STACK_INVALID",
                    "inventory stack size must be at least 1",
                ));
            }
            let template = item_template.ok_or_else(|| {
                edit_error(
                    "EDIT_AREA_INVENTORY_TEMPLATE_REQUIRED",
                    format!("item blueprint {resref:?} must be resolved before insertion"),
                )
            })?;
            if template.file_type != "UTI " {
                return Err(edit_error(
                    "EDIT_AREA_INVENTORY_TEMPLATE_INVALID",
                    format!(
                        "inventory item requires UTI, found {:?}",
                        template.file_type
                    ),
                ));
            }
            let (list_label, index) = parse_area_instance_id(area, instance_id)?;
            let instance = area_instance_mut(&mut document.root, list_label, index)?;
            let items = area_inventory_list_mut(instance, list_label, *category_index, true)?;
            if items.len() >= MAX_DIALOGUE_NODES {
                return Err(edit_error(
                    "EDIT_AREA_INVENTORY_LIMIT_EXCEEDED",
                    "inventory item limit exceeded",
                ));
            }
            let mut item = template.root.clone();
            item.index = items.len() as u32 + 1;
            item.struct_type = 0;
            set_or_insert_resref(&mut item, "TemplateResRef", resref.clone())?;
            set_or_insert_integer(&mut item, "StackSize", i64::from(*stack_size), 2)?;
            set_or_insert_integer(&mut item, "Repos_PosX", i64::from(*x), 2)?;
            set_or_insert_integer(&mut item, "Repos_Posy", i64::from(*y), 2)?;
            if list_label == "StoreList" {
                set_or_insert_integer(&mut item, "Infinite", i64::from(*infinite), 0)?;
            }
            items.push(item);
            if list_label == "Placeable List" {
                set_or_insert_integer(instance, "HasInventory", 1, 0)?;
            }
        }
        AreaStructureAction::RemoveInventoryItem {
            instance_id,
            item_index,
            category_index,
        } => {
            let (list_label, index) = parse_area_instance_id(area, instance_id)?;
            let instance = area_instance_mut(&mut document.root, list_label, index)?;
            let items = area_inventory_list_mut(instance, list_label, *category_index, false)?;
            if *item_index >= items.len() {
                return Err(edit_error(
                    "EDIT_AREA_INVENTORY_ITEM_NOT_FOUND",
                    format!("inventory has no item at index {item_index}"),
                ));
            }
            items.remove(*item_index);
        }
    }
    let output = write_gff(&document)?;
    let reopened = parse_gff(&output, source)?;
    Ok((output, reopened))
}

fn parse_area_instance_id<'a>(area: &str, instance_id: &'a str) -> AppResult<(&'a str, usize)> {
    let prefix = format!("{area}:");
    let identity = instance_id.strip_prefix(&prefix).ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} does not belong to area {area:?}"),
        )
    })?;
    let (list_label, index) = identity.rsplit_once(':').ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} has no list index"),
        )
    })?;
    let index = index.parse::<usize>().map_err(|_| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} has an invalid list index"),
        )
    })?;
    Ok((list_label, index))
}

fn area_instance_mut<'a>(
    root: &'a mut GenericStruct,
    list_label: &str,
    index: usize,
) -> AppResult<&'a mut GenericStruct> {
    list_mut(root, list_label)?.get_mut(index).ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_NOT_FOUND",
            format!("list {list_label:?} has no instance at index {index}"),
        )
    })
}

fn validate_area_polygon(points: &[AreaPoint], maximum: usize) -> AppResult<()> {
    if points.len() < 3
        || points.len() > maximum
        || points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
    {
        return Err(edit_error(
            "EDIT_AREA_GEOMETRY_INVALID",
            format!("polygon must contain 3..={maximum} finite points"),
        ));
    }
    if points.windows(2).any(|pair| {
        (pair[0].x - pair[1].x).abs() < 0.000_1
            && (pair[0].y - pair[1].y).abs() < 0.000_1
            && (pair[0].z - pair[1].z).abs() < 0.000_1
    }) {
        return Err(edit_error(
            "EDIT_AREA_GEOMETRY_DUPLICATE_POINT",
            "polygon contains consecutive duplicate points",
        ));
    }
    let area = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left.x * right.y - right.x * left.y)
        .sum::<f64>()
        .abs()
        * 0.5;
    if area < 0.000_1 {
        return Err(edit_error(
            "EDIT_AREA_GEOMETRY_DEGENERATE",
            "polygon XY area must be greater than zero",
        ));
    }
    Ok(())
}

fn area_inventory_list_mut<'a>(
    instance: &'a mut GenericStruct,
    list_label: &str,
    category_index: Option<usize>,
    create: bool,
) -> AppResult<&'a mut Vec<GenericStruct>> {
    match list_label {
        "Placeable List" => {
            if create && find_field(instance, &["ItemList"]).is_none() {
                instance
                    .fields
                    .push(gff_field("ItemList", 15, GenericValue::List(Vec::new())));
            }
            list_mut(instance, "ItemList")
        }
        "StoreList" => {
            let category_index = category_index.ok_or_else(|| {
                edit_error(
                    "EDIT_AREA_INVENTORY_CATEGORY_REQUIRED",
                    "store inventory requires a category index",
                )
            })?;
            let categories = list_mut(instance, "StoreList")?;
            let category = categories.get_mut(category_index).ok_or_else(|| {
                edit_error(
                    "EDIT_AREA_INVENTORY_CATEGORY_NOT_FOUND",
                    format!("store has no inventory category at index {category_index}"),
                )
            })?;
            if create && find_field(category, &["ItemList"]).is_none() {
                category
                    .fields
                    .push(gff_field("ItemList", 15, GenericValue::List(Vec::new())));
            }
            list_mut(category, "ItemList")
        }
        _ => Err(edit_error(
            "EDIT_AREA_INVENTORY_CATEGORY_INVALID",
            format!("{list_label:?} does not support an editable inventory"),
        )),
    }
}

fn replace_or_insert_list(
    structure: &mut GenericStruct,
    label: &str,
    values: Vec<GenericStruct>,
) -> AppResult<()> {
    if let Some(field) = structure
        .fields
        .iter_mut()
        .find(|field| field.label == label)
    {
        if !matches!(field.value, GenericValue::List(_)) {
            return Err(edit_error(
                "EDIT_GFF_PATH_INVALID",
                format!("field {label:?} is not a list"),
            ));
        }
        field.value = GenericValue::List(values);
    } else {
        structure
            .fields
            .push(gff_field(label, 15, GenericValue::List(values)));
    }
    Ok(())
}

fn set_or_insert_string(
    structure: &mut GenericStruct,
    label: &str,
    value: String,
) -> AppResult<()> {
    if let Some(field) = structure
        .fields
        .iter_mut()
        .find(|field| field.label == label)
    {
        if !matches!(field.value, GenericValue::String(_)) {
            return Err(edit_error(
                "EDIT_GFF_VALUE_INVALID",
                format!("field {label:?} is not a string"),
            ));
        }
        field.value = GenericValue::String(value);
    } else {
        structure
            .fields
            .push(gff_field(label, 10, GenericValue::String(value)));
    }
    Ok(())
}

fn set_or_insert_resref(
    structure: &mut GenericStruct,
    label: &str,
    value: String,
) -> AppResult<()> {
    if let Some(field) = structure
        .fields
        .iter_mut()
        .find(|field| field.label == label)
    {
        if !matches!(field.value, GenericValue::ResRef(_)) {
            return Err(edit_error(
                "EDIT_GFF_VALUE_INVALID",
                format!("field {label:?} is not a ResRef"),
            ));
        }
        field.value = GenericValue::ResRef(value);
    } else {
        structure
            .fields
            .push(gff_field(label, 11, GenericValue::ResRef(value)));
    }
    Ok(())
}

fn set_or_insert_integer(
    structure: &mut GenericStruct,
    label: &str,
    value: i64,
    field_type: u32,
) -> AppResult<()> {
    if find_field(structure, &[label]).is_some() {
        return set_integer(structure, &[label], value);
    }
    let value = match field_type {
        0 => GenericValue::Byte(u8::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit BYTE"))
        })?),
        2 => GenericValue::Word(u16::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit WORD"))
        })?),
        4 => GenericValue::Dword(u32::try_from(value).map_err(|_| {
            edit_error(
                "EDIT_GFF_VALUE_RANGE",
                format!("{value} does not fit DWORD"),
            )
        })?),
        5 => GenericValue::Int(i32::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit INT"))
        })?),
        _ => {
            return Err(edit_error(
                "EDIT_GFF_FIELD_TYPE_INVALID",
                format!("cannot insert integer GFF field type {field_type}"),
            ));
        }
    };
    structure.fields.push(gff_field(label, field_type, value));
    Ok(())
}

fn set_or_insert_float(structure: &mut GenericStruct, label: &str, value: f32) -> AppResult<()> {
    if find_field(structure, &[label]).is_some() {
        return set_numeric(structure, &[label], f64::from(value));
    }
    structure
        .fields
        .push(gff_field(label, 8, GenericValue::Float(value)));
    Ok(())
}

fn patch_integer(
    structure: &mut GenericStruct,
    label: &str,
    value: Option<i64>,
    field_type: u32,
) -> AppResult<()> {
    if let Some(value) = value {
        set_or_insert_integer(structure, label, value, field_type)?;
    }
    Ok(())
}

fn patch_bool(structure: &mut GenericStruct, label: &str, value: Option<bool>) -> AppResult<()> {
    patch_integer(structure, label, value.map(i64::from), 0)
}

fn patch_resref(structure: &mut GenericStruct, label: &str, value: Option<&str>) -> AppResult<()> {
    if let Some(value) = value {
        if !value.is_empty() {
            validate_resref(value)?;
        }
        set_or_insert_resref(structure, label, value.to_owned())?;
    }
    Ok(())
}

fn read_text_field(structure: &GenericStruct, label: &str) -> Option<String> {
    find_field(structure, &[label]).and_then(|field| match &field.value {
        GenericValue::String(value) | GenericValue::ResRef(value) => Some(value.clone()),
        _ => None,
    })
}

fn read_integer_field(structure: &GenericStruct, label: &str) -> Option<i64> {
    find_field(structure, &[label]).and_then(|field| integer(&field.value))
}

fn read_numeric_field(structure: &GenericStruct, label: &str) -> Option<f64> {
    find_field(structure, &[label]).and_then(|field| numeric(&field.value))
}

fn read_bool_field(structure: &GenericStruct, label: &str) -> Option<bool> {
    read_integer_field(structure, label).map(|value| value != 0)
}

fn as_u8(value: i64) -> Option<u8> {
    u8::try_from(value).ok()
}

fn as_u32(value: i64) -> Option<u32> {
    u32::try_from(value).ok()
}

fn as_i32(value: i64) -> Option<i32> {
    i32::try_from(value).ok()
}

fn validate_area_environment_patch(patch: &AreaEnvironmentPatch) -> AppResult<()> {
    if patch.tag.as_ref().is_some_and(|value| {
        value.is_empty()
            || value.len() > 64
            || value.chars().any(|character| character.is_control())
    }) || patch
        .comments
        .as_ref()
        .is_some_and(|value| value.len() > 16 * 1024 || value.contains('\0'))
        || patch.player_vs_player.is_some_and(|value| value > 2)
        || [patch.chance_rain, patch.chance_snow, patch.chance_lightning]
            .into_iter()
            .flatten()
            .any(|value| value > 100)
        || patch.wind_power.is_some_and(|value| value > 2)
        || patch
            .fog_clip_distance
            .is_some_and(|value| !value.is_finite() || !(1.0..=1_000.0).contains(&value))
        || patch.shadow_opacity.is_some_and(|value| value > 100)
    {
        return Err(edit_error(
            "EDIT_AREA_ENVIRONMENT_INVALID",
            "area environment values exceed their conservative NWN bounds",
        ));
    }
    for script in [
        patch.on_enter.as_deref(),
        patch.on_exit.as_deref(),
        patch.on_heartbeat.as_deref(),
        patch.on_user_defined.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !script.is_empty() {
            validate_resref(script)?;
        }
    }
    Ok(())
}

fn validate_area_audio_patch(patch: &AreaAudioPatch) -> AppResult<()> {
    let ids = [
        patch.ambient_sound_day,
        patch.ambient_sound_night,
        patch.environment_audio,
        patch.music_battle,
        patch.music_day,
        patch.music_night,
        patch.music_delay,
    ];
    if ids.into_iter().flatten().any(|value| value < 0)
        || patch
            .ambient_sound_day_volume
            .is_some_and(|value| value > 127)
        || patch
            .ambient_sound_night_volume
            .is_some_and(|value| value > 127)
    {
        return Err(edit_error(
            "EDIT_AREA_AUDIO_INVALID",
            "area audio identifiers must be non-negative and volumes must be within 0..=127",
        ));
    }
    Ok(())
}

pub fn add_area_instance(
    bytes: &[u8],
    source: &str,
    area: &str,
    placement: &InstancePlacement,
) -> AppResult<(Vec<u8>, String)> {
    let mut document = parse_gff(bytes, source)?;
    let instance_id = append_area_instance(&mut document, area, placement)?;
    let output = write_gff(&document)?;
    Ok((output, instance_id))
}

fn append_area_instance(
    document: &mut GenericGff,
    area: &str,
    placement: &InstancePlacement,
) -> AppResult<String> {
    validate_resref(&placement.template_resref)?;
    if !placement.x.is_finite()
        || !placement.y.is_finite()
        || !placement.z.is_finite()
        || !placement.bearing.is_finite()
    {
        return Err(edit_error(
            "EDIT_AREA_COORDINATE_INVALID",
            "instance coordinates must be finite",
        ));
    }
    let list_label = instance_list_label(&placement.category)?;
    let values = list_mut(&mut document.root, list_label)?;
    let index = values.len();
    let mut fields = vec![
        gff_field(
            "TemplateResRef",
            11,
            GenericValue::ResRef(placement.template_resref.clone()),
        ),
        gff_field("Tag", 10, GenericValue::String(placement.tag.clone())),
        gff_field("XPosition", 8, GenericValue::Float(placement.x as f32)),
        gff_field("YPosition", 8, GenericValue::Float(placement.y as f32)),
        gff_field("ZPosition", 8, GenericValue::Float(placement.z as f32)),
        gff_field("Bearing", 8, GenericValue::Float(placement.bearing as f32)),
    ];
    match list_label {
        "Door List" | "TriggerList" => {
            fields.push(gff_field(
                "LinkedTo",
                10,
                GenericValue::String(placement.linked_to.clone().unwrap_or_default()),
            ));
            fields.push(gff_field("LinkedToFlags", 0, GenericValue::Byte(0)));
            fields.push(gff_field("LoadScreenID", 2, GenericValue::Word(0)));
            if list_label == "TriggerList" {
                fields.push(gff_field("Geometry", 15, GenericValue::List(Vec::new())));
            }
        }
        "Encounter List" => {
            fields.push(gff_field("Geometry", 15, GenericValue::List(Vec::new())));
            fields.push(gff_field(
                "SpawnPointList",
                15,
                GenericValue::List(Vec::new()),
            ));
        }
        "Placeable List" => {
            fields.push(gff_field("HasInventory", 0, GenericValue::Byte(0)));
            fields.push(gff_field("ItemList", 15, GenericValue::List(Vec::new())));
        }
        "StoreList" => {
            fields.push(gff_field("WillNotBuy", 15, GenericValue::List(Vec::new())));
            fields.push(gff_field("WillOnlyBuy", 15, GenericValue::List(Vec::new())));
            fields.push(gff_field(
                "StoreList",
                15,
                GenericValue::List(
                    (0..5)
                        .map(|index| GenericStruct {
                            index: index + 1,
                            struct_type: index,
                            fields: vec![gff_field("ItemList", 15, GenericValue::List(Vec::new()))],
                        })
                        .collect(),
                ),
            ));
        }
        _ => {}
    }
    values.push(GenericStruct {
        index: index as u32 + 1,
        struct_type: 1,
        fields,
    });
    Ok(format!("{area}:{list_label}:{index}"))
}

pub fn remove_area_instance(
    bytes: &[u8],
    source: &str,
    area: &str,
    instance_id: &str,
) -> AppResult<Vec<u8>> {
    let (prefix, index) = instance_id.rsplit_once(':').ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} has no list index"),
        )
    })?;
    let index = index.parse::<usize>().map_err(|_| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} has an invalid list index"),
        )
    })?;
    let area_prefix = format!("{area}:");
    let list_label = prefix.strip_prefix(&area_prefix).ok_or_else(|| {
        edit_error(
            "EDIT_AREA_INSTANCE_ID_INVALID",
            format!("instance id {instance_id:?} does not belong to area {area:?}"),
        )
    })?;
    let mut document = parse_gff(bytes, source)?;
    let values = list_mut(&mut document.root, list_label)?;
    if index >= values.len() {
        return Err(edit_error(
            "EDIT_AREA_INSTANCE_NOT_FOUND",
            format!("list {list_label:?} has no instance at index {index}"),
        ));
    }
    values.remove(index);
    write_gff(&document)
}

fn instance_list_label(category: &str) -> AppResult<&'static str> {
    match category.to_ascii_lowercase().as_str() {
        "creature" => Ok("Creature List"),
        "door" => Ok("Door List"),
        "encounter" => Ok("Encounter List"),
        "item" => Ok("List"),
        "placeable" => Ok("Placeable List"),
        "sound" => Ok("SoundList"),
        "store" => Ok("StoreList"),
        "trigger" => Ok("TriggerList"),
        "waypoint" => Ok("WaypointList"),
        _ => Err(edit_error(
            "EDIT_INSTANCE_CATEGORY_UNSUPPORTED",
            format!("unsupported instance category {category:?}"),
        )),
    }
}

pub fn edit_area_tile(
    bytes: &[u8],
    source: &str,
    tile_index: usize,
    before: TileState,
    after: TileState,
) -> AppResult<Vec<u8>> {
    let mut document = parse_gff(bytes, source)?;
    let values = list_mut_any(&mut document.root, &["Tile_List", "TileList"])?;
    let tile = values.get_mut(tile_index).ok_or_else(|| {
        edit_error(
            "EDIT_AREA_TILE_NOT_FOUND",
            format!("tile list has no entry at index {tile_index}"),
        )
    })?;
    verify_integer(tile, &["Tile_ID", "TileID"], before.tile_id as i64)?;
    verify_integer(
        tile,
        &["Tile_Orientation", "Orientation"],
        i64::from(before.orientation),
    )?;
    if before.height != after.height && find_field(tile, &["Tile_Height", "Height"]).is_some() {
        verify_integer(tile, &["Tile_Height", "Height"], i64::from(before.height))?;
    }
    set_integer(tile, &["Tile_ID", "TileID"], i64::from(after.tile_id))?;
    set_integer(
        tile,
        &["Tile_Orientation", "Orientation"],
        i64::from(after.orientation),
    )?;
    if before.height != after.height && find_field(tile, &["Tile_Height", "Height"]).is_some() {
        set_integer(tile, &["Tile_Height", "Height"], i64::from(after.height))?;
    }
    write_gff(&document)
}

pub fn create_generated_map_resources(
    plan: &MapGenerationPlan,
) -> AppResult<Vec<ErfResourceInput>> {
    let expected = generate_map_plan_with_compatibility(&plan.spec, plan.compatibility.clone())?;
    if expected.plan_sha256 != plan.plan_sha256
        || expected.tiles != plan.tiles
        || expected.placements != plan.placements
    {
        return Err(edit_error(
            "EDIT_MAP_PLAN_CHANGED",
            "map plan does not match its deterministic specification",
        ));
    }
    let mut resources = create_area_resources(
        &plan.spec.resref,
        &plan.spec.name,
        &plan.spec.tileset,
        plan.spec.width,
        plan.spec.height,
        plan.spec.base_tile_id,
    )?;
    let are = resources
        .iter_mut()
        .find(|resource| resource.key.resource_type == 2012)
        .ok_or_else(|| edit_error("EDIT_MAP_RESOURCE_MISSING", "generated ARE is missing"))?;
    let mut are_document = parse_gff(&are.bytes, &format!("map-plan::{}", are.key.file_name()))?;
    let tile_values = list_mut_any(&mut are_document.root, &["Tile_List", "TileList"])?;
    if tile_values.len() != plan.tiles.len() {
        return Err(edit_error(
            "EDIT_MAP_TILE_COUNT_INVALID",
            format!(
                "generated ARE has {} tiles but plan has {}",
                tile_values.len(),
                plan.tiles.len()
            ),
        ));
    }
    for tile in &plan.tiles {
        let index = tile
            .y
            .checked_mul(plan.spec.width)
            .and_then(|value| value.checked_add(tile.x))
            .ok_or_else(|| edit_error("EDIT_MAP_TILE_INDEX_OVERFLOW", "tile index overflow"))?
            as usize;
        let value = tile_values.get_mut(index).ok_or_else(|| {
            edit_error(
                "EDIT_MAP_TILE_NOT_FOUND",
                format!("generated ARE has no tile at {},{}", tile.x, tile.y),
            )
        })?;
        set_integer(value, &["Tile_ID", "TileID"], i64::from(tile.tile_id))?;
        set_integer(
            value,
            &["Tile_Orientation", "Orientation"],
            i64::from(tile.orientation),
        )?;
        set_integer(value, &["Tile_Height", "Height"], i64::from(tile.height))?;
    }
    are.bytes = write_gff(&are_document)?;

    let git = resources
        .iter_mut()
        .find(|resource| resource.key.resource_type == 2023)
        .ok_or_else(|| edit_error("EDIT_MAP_RESOURCE_MISSING", "generated GIT is missing"))?;
    let mut git_document = parse_gff(&git.bytes, &format!("map-plan::{}", git.key.file_name()))?;
    for placement in &plan.placements {
        append_area_instance(&mut git_document, &plan.spec.resref, placement)?;
    }
    git.bytes = write_gff(&git_document)?;
    Ok(resources)
}

fn list_mut<'a>(
    structure: &'a mut GenericStruct,
    label: &str,
) -> AppResult<&'a mut Vec<GenericStruct>> {
    list_mut_any(structure, &[label])
}

fn list_mut_any<'a>(
    structure: &'a mut GenericStruct,
    labels: &[&str],
) -> AppResult<&'a mut Vec<GenericStruct>> {
    let field = structure
        .fields
        .iter_mut()
        .find(|field| labels.iter().any(|label| field.label == *label))
        .ok_or_else(|| {
            edit_error(
                "EDIT_GFF_FIELD_NOT_FOUND",
                format!("struct does not contain any of {labels:?}"),
            )
        })?;
    match &mut field.value {
        GenericValue::List(values) => Ok(values),
        _ => Err(edit_error(
            "EDIT_GFF_PATH_INVALID",
            format!("field {:?} is not a list", field.label),
        )),
    }
}

fn find_field<'a>(structure: &'a GenericStruct, labels: &[&str]) -> Option<&'a GenericField> {
    structure
        .fields
        .iter()
        .find(|field| labels.iter().any(|label| field.label == *label))
}

fn find_field_mut_any<'a>(
    structure: &'a mut GenericStruct,
    labels: &[&str],
) -> AppResult<&'a mut GenericField> {
    structure
        .fields
        .iter_mut()
        .find(|field| labels.iter().any(|label| field.label == *label))
        .ok_or_else(|| {
            edit_error(
                "EDIT_GFF_FIELD_NOT_FOUND",
                format!("struct does not contain any of {labels:?}"),
            )
        })
}

fn numeric(value: &GenericValue) -> Option<f64> {
    match value {
        GenericValue::Byte(value) => Some(f64::from(*value)),
        GenericValue::Char(value) => Some(f64::from(*value)),
        GenericValue::Word(value) => Some(f64::from(*value)),
        GenericValue::Short(value) => Some(f64::from(*value)),
        GenericValue::Dword(value) => Some(f64::from(*value)),
        GenericValue::Int(value) => Some(f64::from(*value)),
        GenericValue::Float(value) => Some(f64::from(*value)),
        GenericValue::Double(value) => Some(*value),
        _ => None,
    }
}

fn verify_numeric(structure: &GenericStruct, labels: &[&str], expected: f64) -> AppResult<()> {
    let field = find_field(structure, labels).ok_or_else(|| {
        edit_error(
            "EDIT_GFF_FIELD_NOT_FOUND",
            format!("struct does not contain any of {labels:?}"),
        )
    })?;
    let current = numeric(&field.value).ok_or_else(|| {
        edit_error(
            "EDIT_GFF_VALUE_INVALID",
            format!("field {:?} is not numeric", field.label),
        )
    })?;
    if (current - expected).abs() > 0.000_1 {
        return Err(edit_error(
            "EDIT_AREA_PRECONDITION_FAILED",
            format!("field {:?} is {current}, expected {expected}", field.label),
        ));
    }
    Ok(())
}

fn set_numeric(structure: &mut GenericStruct, labels: &[&str], value: f64) -> AppResult<()> {
    let field = find_field_mut_any(structure, labels)?;
    field.value = match &field.value {
        GenericValue::Float(_) => GenericValue::Float(value as f32),
        GenericValue::Double(_) => GenericValue::Double(value),
        _ => {
            return Err(edit_error(
                "EDIT_GFF_VALUE_INVALID",
                format!("field {:?} is not a floating-point value", field.label),
            ));
        }
    };
    Ok(())
}

fn integer(value: &GenericValue) -> Option<i64> {
    match value {
        GenericValue::Byte(value) => Some(i64::from(*value)),
        GenericValue::Char(value) => Some(i64::from(*value)),
        GenericValue::Word(value) => Some(i64::from(*value)),
        GenericValue::Short(value) => Some(i64::from(*value)),
        GenericValue::Dword(value) => Some(i64::from(*value)),
        GenericValue::Int(value) => Some(i64::from(*value)),
        _ => None,
    }
}

fn verify_integer(structure: &GenericStruct, labels: &[&str], expected: i64) -> AppResult<()> {
    let field = find_field(structure, labels).ok_or_else(|| {
        edit_error(
            "EDIT_GFF_FIELD_NOT_FOUND",
            format!("struct does not contain any of {labels:?}"),
        )
    })?;
    let current = integer(&field.value).ok_or_else(|| {
        edit_error(
            "EDIT_GFF_VALUE_INVALID",
            format!("field {:?} is not an integer", field.label),
        )
    })?;
    if current != expected {
        return Err(edit_error(
            "EDIT_AREA_PRECONDITION_FAILED",
            format!("field {:?} is {current}, expected {expected}", field.label),
        ));
    }
    Ok(())
}

fn set_integer(structure: &mut GenericStruct, labels: &[&str], value: i64) -> AppResult<()> {
    let field = find_field_mut_any(structure, labels)?;
    field.value = match &field.value {
        GenericValue::Byte(_) => GenericValue::Byte(u8::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit BYTE"))
        })?),
        GenericValue::Char(_) => GenericValue::Char(i8::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit CHAR"))
        })?),
        GenericValue::Word(_) => GenericValue::Word(u16::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit WORD"))
        })?),
        GenericValue::Short(_) => GenericValue::Short(i16::try_from(value).map_err(|_| {
            edit_error(
                "EDIT_GFF_VALUE_RANGE",
                format!("{value} does not fit SHORT"),
            )
        })?),
        GenericValue::Dword(_) => GenericValue::Dword(u32::try_from(value).map_err(|_| {
            edit_error(
                "EDIT_GFF_VALUE_RANGE",
                format!("{value} does not fit DWORD"),
            )
        })?),
        GenericValue::Int(_) => GenericValue::Int(i32::try_from(value).map_err(|_| {
            edit_error("EDIT_GFF_VALUE_RANGE", format!("{value} does not fit INT"))
        })?),
        _ => {
            return Err(edit_error(
                "EDIT_GFF_VALUE_INVALID",
                format!("field {:?} is not an integer", field.label),
            ));
        }
    };
    Ok(())
}

fn find_gff_field_mut<'a>(
    structure: &'a mut GenericStruct,
    segments: &[&str],
) -> AppResult<&'a mut GenericField> {
    let label = segments[0];
    let field = structure
        .fields
        .iter_mut()
        .find(|field| field.label == label)
        .ok_or_else(|| {
            edit_error(
                "EDIT_GFF_FIELD_NOT_FOUND",
                format!("struct does not contain field {label:?}"),
            )
        })?;
    if segments.len() == 1 {
        return Ok(field);
    }
    match &mut field.value {
        GenericValue::Struct(child) => find_gff_field_mut(child, &segments[1..]),
        GenericValue::List(children) => {
            if segments.len() < 3 {
                return Err(edit_error(
                    "EDIT_GFF_PATH_INVALID",
                    format!("list field {label:?} requires an index and child field"),
                ));
            }
            let index = segments[1].parse::<usize>().map_err(|_| {
                edit_error(
                    "EDIT_GFF_PATH_INVALID",
                    format!("list index {:?} is not an unsigned integer", segments[1]),
                )
            })?;
            let child = children.get_mut(index).ok_or_else(|| {
                edit_error(
                    "EDIT_GFF_LIST_INDEX_INVALID",
                    format!("list {label:?} has no child at index {index}"),
                )
            })?;
            find_gff_field_mut(child, &segments[2..])
        }
        _ => Err(edit_error(
            "EDIT_GFF_PATH_INVALID",
            format!("field {label:?} is not a struct or list"),
        )),
    }
}

impl EditCommand {
    fn affected_resources(&self) -> Vec<ResourceKey> {
        let mut resources = match self {
            Self::SetField { resource, .. }
            | Self::TransformResource { resource, .. }
            | Self::ReplaceText { resource, .. }
            | Self::CompileScript { resource, .. }
            | Self::CreateResource { resource, .. }
            | Self::DeleteResource { resource, .. } => vec![resource.clone()],
            Self::MoveInstance { area, .. } => vec![ResourceKey::new(area, 2023)],
            Self::AddInstance { area, .. } | Self::RemoveInstance { area, .. } => {
                vec![ResourceKey::new(area, 2023)]
            }
            Self::SetTile { area, .. } => vec![ResourceKey::new(area, 2012)],
            Self::CreateResourceSet { resources } | Self::DeleteResourceSet { resources } => {
                resources
                    .iter()
                    .map(|entry| entry.resource.clone())
                    .collect()
            }
        };
        resources.sort();
        resources
    }

    fn target(&self) -> String {
        match self {
            Self::SetField { resource, path, .. } => format!("field:{resource}:{path}"),
            Self::TransformResource {
                resource,
                before_sha256,
                ..
            } => format!("structure:{resource}:{before_sha256}"),
            Self::ReplaceText { resource, .. } => format!("text:{resource}"),
            Self::CompileScript { resource, .. } => format!("compiled:{resource}"),
            Self::MoveInstance {
                area, instance_id, ..
            } => format!("instance:{area}:{instance_id}:transform"),
            Self::AddInstance {
                area, instance_id, ..
            } => format!("instance:{area}:{instance_id}:exists"),
            Self::RemoveInstance {
                area, instance_id, ..
            } => format!("instance:{area}:{instance_id}:exists"),
            Self::SetTile { area, x, y, .. } => format!("tile:{area}:{x}:{y}"),
            Self::CreateResource { resource, .. } | Self::DeleteResource { resource, .. } => {
                format!("resource:{resource}:exists")
            }
            Self::CreateResourceSet { resources } => format!(
                "resource_set:{}:exists",
                resources
                    .iter()
                    .map(|entry| entry.resource.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::DeleteResourceSet { resources } => format!(
                "resource_set:{}:exists",
                resources
                    .iter()
                    .map(|entry| entry.resource.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn values(&self) -> (Value, Value) {
        match self {
            Self::SetField { before, after, .. } => (before.clone(), after.clone()),
            Self::TransformResource {
                before_sha256,
                after_sha256,
                ..
            } => (
                Value::String(before_sha256.clone()),
                Value::String(after_sha256.clone()),
            ),
            Self::ReplaceText { before, after, .. } => {
                (Value::String(before.clone()), Value::String(after.clone()))
            }
            Self::CompileScript {
                before_sha256,
                after_sha256,
                ..
            } => (
                before_sha256
                    .as_ref()
                    .map_or(Value::Null, |value| Value::String(value.clone())),
                Value::String(after_sha256.clone()),
            ),
            Self::MoveInstance { before, after, .. } => (
                serde_json::to_value(before).expect("transform serializes"),
                serde_json::to_value(after).expect("transform serializes"),
            ),
            Self::SetTile { before, after, .. } => (
                serde_json::to_value(before).expect("tile serializes"),
                serde_json::to_value(after).expect("tile serializes"),
            ),
            Self::AddInstance { placement, .. } => (
                Value::Bool(false),
                serde_json::to_value(placement).expect("placement serializes"),
            ),
            Self::RemoveInstance { .. } => (Value::Bool(true), Value::Bool(false)),
            Self::CreateResource { content_sha256, .. } => {
                (Value::Bool(false), Value::String(content_sha256.clone()))
            }
            Self::DeleteResource { content_sha256, .. } => {
                (Value::String(content_sha256.clone()), Value::Bool(false))
            }
            Self::CreateResourceSet { resources } => (
                Value::Bool(false),
                serde_json::to_value(resources).expect("resource digests serialize"),
            ),
            Self::DeleteResourceSet { resources } => (
                serde_json::to_value(resources).expect("resource digests serialize"),
                Value::Bool(false),
            ),
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::SetField { path, .. } if path.trim().is_empty() => {
                Err("Le chemin de champ ne peut pas être vide.".to_owned())
            }
            Self::TransformResource {
                operation,
                before_sha256,
                after_sha256,
                ..
            } if operation.trim().is_empty()
                || !valid_sha256(before_sha256)
                || !valid_sha256(after_sha256) =>
            {
                Err(
                    "La transformation doit déclarer une opération et deux SHA-256 valides."
                        .to_owned(),
                )
            }
            Self::ReplaceText { after, .. } if after.len() > 8 * 1024 * 1024 => {
                Err("Le texte dépasse la limite de 8 Mio.".to_owned())
            }
            Self::SetTile { before, after, .. }
                if before.orientation > 3 || after.orientation > 3 =>
            {
                Err("L’orientation d’une tuile doit être comprise entre 0 et 3.".to_owned())
            }
            Self::AddInstance { placement, .. }
                if !placement.x.is_finite()
                    || !placement.y.is_finite()
                    || !placement.z.is_finite()
                    || !placement.bearing.is_finite() =>
            {
                Err("Les coordonnées de placement doivent être finies.".to_owned())
            }
            Self::CreateResource { content_sha256, .. }
            | Self::DeleteResource { content_sha256, .. }
                if !is_sha256(content_sha256) =>
            {
                Err("L’empreinte de ressource doit être un SHA-256 hexadécimal.".to_owned())
            }
            Self::CreateResourceSet { resources } | Self::DeleteResourceSet { resources }
                if resources.is_empty()
                    || resources
                        .iter()
                        .any(|entry| !is_sha256(&entry.content_sha256)) =>
            {
                Err("Resource sets must contain valid SHA-256 digests.".to_owned())
            }
            Self::CompileScript {
                inputs,
                compiler_sha256,
                before_sha256,
                after_sha256,
                ..
            } if inputs.is_empty()
                || inputs.iter().any(|entry| !is_sha256(&entry.content_sha256))
                || !is_sha256(compiler_sha256)
                || before_sha256
                    .as_ref()
                    .is_some_and(|value| !is_sha256(value))
                || !is_sha256(after_sha256) =>
            {
                Err(
                    "Les empreintes de compilation doivent être des SHA-256 hexadécimaux."
                        .to_owned(),
                )
            }
            _ => Ok(()),
        }
    }
}

fn ensure_safe_workspace_root(root: &Path, source_path: &Path) -> AppResult<()> {
    let source = source_path.canonicalize().map_err(|error| {
        Box::new(AppError::io(
            "canonicalize source module",
            source_path.display().to_string(),
            &error,
        ))
    })?;
    let root_absolute = if root.exists() {
        root.canonicalize().map_err(|error| {
            Box::new(AppError::io(
                "canonicalize edit workspace",
                root.display().to_string(),
                &error,
            ))
        })?
    } else {
        let parent = root.parent().ok_or_else(|| {
            edit_error(
                "EDIT_WORKSPACE_PATH_INVALID",
                "workspace root has no parent directory",
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create edit workspace parent",
                parent.display().to_string(),
                &error,
            ))
        })?;
        parent
            .canonicalize()
            .map_err(|error| {
                Box::new(AppError::io(
                    "canonicalize edit workspace parent",
                    parent.display().to_string(),
                    &error,
                ))
            })?
            .join(root.file_name().ok_or_else(|| {
                edit_error(
                    "EDIT_WORKSPACE_PATH_INVALID",
                    "workspace root has no final component",
                )
            })?)
    };
    if source.starts_with(&root_absolute) || root_absolute == source {
        return Err(edit_error(
            "EDIT_WORKSPACE_SOURCE_OVERLAP",
            "workspace root cannot contain or equal the source module",
        ));
    }
    Ok(())
}

fn verify_source(path: &Path, expected_sha256: &str, expected_size: u64) -> AppResult<()> {
    let metadata = fs::metadata(path).map_err(|error| {
        Box::new(AppError::io(
            "verify source module",
            path.display().to_string(),
            &error,
        ))
    })?;
    if metadata.len() != expected_size {
        return Err(edit_error(
            "EDIT_SOURCE_CHANGED",
            format!(
                "source size changed from {expected_size} to {} bytes",
                metadata.len()
            ),
        ));
    }
    let actual = sha256_file(path)?;
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(edit_error(
            "EDIT_SOURCE_CHANGED",
            format!("source hash changed from {expected_sha256} to {actual}"),
        ));
    }
    Ok(())
}

fn ensure_output_is_not_source(output: &Path, source: &Path) -> AppResult<()> {
    let source = source.canonicalize().map_err(|error| {
        Box::new(AppError::io(
            "canonicalize source module",
            source.display().to_string(),
            &error,
        ))
    })?;
    let output_absolute = if output.exists() {
        output.canonicalize().map_err(|error| {
            Box::new(AppError::io(
                "canonicalize build output",
                output.display().to_string(),
                &error,
            ))
        })?
    } else {
        let parent = output.parent().ok_or_else(|| {
            edit_error(
                "EDIT_BUILD_PATH_INVALID",
                "build output has no parent directory",
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create build output directory",
                parent.display().to_string(),
                &error,
            ))
        })?;
        parent
            .canonicalize()
            .map_err(|error| {
                Box::new(AppError::io(
                    "canonicalize build output directory",
                    parent.display().to_string(),
                    &error,
                ))
            })?
            .join(output.file_name().ok_or_else(|| {
                edit_error("EDIT_BUILD_PATH_INVALID", "build output has no file name")
            })?)
    };
    if output_absolute == source {
        return Err(edit_error(
            "EDIT_BUILD_SOURCE_OVERWRITE_BLOCKED",
            "build output cannot overwrite the source module",
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let file = fs::File::open(path).map_err(|error| {
        Box::new(AppError::io(
            "hash source module",
            path.display().to_string(),
            &error,
        ))
    })?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| {
            Box::new(AppError::io(
                "hash source module",
                path.display().to_string(),
                &error,
            ))
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn atomic_copy(source: &Path, destination: &Path) -> AppResult<()> {
    let parent = destination.parent().ok_or_else(|| {
        edit_error(
            "EDIT_WORKSPACE_PATH_INVALID",
            format!("{} has no parent", destination.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        Box::new(AppError::io(
            "create atomic copy directory",
            parent.display().to_string(),
            &error,
        ))
    })?;
    let input = File::open(source).map_err(|error| {
        Box::new(AppError::io(
            "open atomic copy source",
            source.display().to_string(),
            &error,
        ))
    })?;
    let mut input = BufReader::new(input);
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        Box::new(AppError::io(
            "create atomic copy temporary file",
            destination.display().to_string(),
            &error,
        ))
    })?;
    std::io::copy(&mut input, &mut temporary).map_err(|error| {
        Box::new(AppError::io(
            "stream atomic copy",
            destination.display().to_string(),
            &error,
        ))
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        Box::new(AppError::io(
            "flush atomic copy",
            destination.display().to_string(),
            &error,
        ))
    })?;
    if destination.is_file() {
        let backup = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
            Box::new(AppError::io(
                "create atomic copy backup",
                destination.display().to_string(),
                &error,
            ))
        })?;
        let backup_path = backup.path().to_path_buf();
        drop(backup);
        fs::rename(destination, &backup_path).map_err(|error| {
            Box::new(AppError::io(
                "backup atomic copy destination",
                destination.display().to_string(),
                &error,
            ))
        })?;
        if let Err(error) = temporary.persist(destination) {
            let _ = fs::rename(&backup_path, destination);
            return Err(Box::new(AppError::io(
                "persist atomic copy",
                destination.display().to_string(),
                &error.error,
            )));
        }
        fs::remove_file(&backup_path).map_err(|error| {
            Box::new(AppError::io(
                "remove atomic copy backup",
                backup_path.display().to_string(),
                &error,
            ))
        })?;
    } else {
        temporary.persist(destination).map_err(|error| {
            Box::new(AppError::io(
                "persist atomic copy",
                destination.display().to_string(),
                &error.error,
            ))
        })?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        edit_error(
            "EDIT_WORKSPACE_PATH_INVALID",
            format!("{} has no parent", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        Box::new(AppError::io(
            "create workspace directory",
            parent.display().to_string(),
            &error,
        ))
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        Box::new(AppError::io(
            "create temporary workspace file",
            path.display().to_string(),
            &error,
        ))
    })?;
    temporary.write_all(bytes).map_err(|error| {
        Box::new(AppError::io(
            "write temporary workspace file",
            path.display().to_string(),
            &error,
        ))
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        Box::new(AppError::io(
            "flush temporary workspace file",
            path.display().to_string(),
            &error,
        ))
    })?;
    if path.is_file() {
        let backup = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
            Box::new(AppError::io(
                "create workspace backup path",
                path.display().to_string(),
                &error,
            ))
        })?;
        let backup_path = backup.path().to_path_buf();
        drop(backup);
        fs::rename(path, &backup_path).map_err(|error| {
            Box::new(AppError::io(
                "backup existing workspace file",
                path.display().to_string(),
                &error,
            ))
        })?;
        if let Err(error) = temporary.persist(path) {
            let _ = fs::rename(&backup_path, path);
            return Err(Box::new(AppError::io(
                "persist workspace file",
                path.display().to_string(),
                &error.error,
            )));
        }
        fs::remove_file(&backup_path).map_err(|error| {
            Box::new(AppError::io(
                "remove workspace backup",
                backup_path.display().to_string(),
                &error,
            ))
        })?;
    } else {
        temporary.persist(path).map_err(|error| {
            Box::new(AppError::io(
                "persist workspace file",
                path.display().to_string(),
                &error.error,
            ))
        })?;
    }
    Ok(())
}

fn edit_error(code: &str, detail: impl Into<String>) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "L’opération d’édition contrôlée a été refusée.",
            detail,
            ErrorSeverity::Error,
        )
        .with_import_stage("edit_workspace"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn workspace() -> (tempfile::TempDir, PathBuf, EditWorkspace) {
        let temp = tempfile::tempdir().expect("temporary directory");
        let source = temp.path().join("source.mod");
        fs::write(&source, b"immutable module").expect("source fixture");
        let hash = sha256_bytes(b"immutable module");
        let workspace = EditWorkspace::create(
            temp.path().join("workspace"),
            &source,
            &hash,
            b"immutable module".len() as u64,
        )
        .expect("workspace");
        (temp, source, workspace)
    }

    #[test]
    fn applies_previews_undoes_and_redoes_without_touching_source() {
        let (_temp, source, mut workspace) = workspace();
        let command = EditCommand::SetField {
            resource: ResourceKey::new("module", 2014),
            path: "Mod_Tag".to_owned(),
            before: json!("OLD"),
            after: json!("NEW"),
        };
        assert!(workspace.preview(command.clone()).valid);
        workspace
            .stage_resource(ResourceKey::new("module", 2014), Some(b"old"), b"new")
            .expect("stage command bytes");
        let applied = workspace.apply(command).expect("apply");
        assert!(applied.can_undo);
        assert!(!applied.can_redo);
        let undone = workspace.undo().expect("undo");
        assert!(undone.can_redo);
        let redone = workspace.redo().expect("redo");
        assert_eq!(redone.cursor, 1);
        assert_eq!(fs::read(source).expect("source"), b"immutable module");
        assert_eq!(redone.journal_events, 5);
    }

    #[test]
    fn structural_transform_requires_the_exact_staged_hashes() {
        let (_temp, _source, mut workspace) = workspace();
        let resource = ResourceKey::new("dialogue", 2029);
        workspace
            .stage_resource(resource.clone(), Some(b"before"), b"after")
            .expect("stage transform");
        let error = workspace
            .apply(EditCommand::TransformResource {
                resource: resource.clone(),
                operation: "add_node".to_owned(),
                before_sha256: sha256_bytes(b"wrong"),
                after_sha256: sha256_bytes(b"after"),
            })
            .expect_err("mismatched transform hash must fail");
        assert_eq!(error.code, "EDIT_RESOURCE_HASH_TRANSACTION_MISMATCH");
        assert!(
            workspace
                .staged_resource_bytes(&resource)
                .expect("staged resource state")
                .is_none()
        );

        workspace
            .stage_resource(resource.clone(), Some(b"before"), b"after")
            .expect("restage transform");
        let applied = workspace
            .apply(EditCommand::TransformResource {
                resource,
                operation: "add_node".to_owned(),
                before_sha256: sha256_bytes(b"before"),
                after_sha256: sha256_bytes(b"after"),
            })
            .expect("exact transform hashes");
        assert_eq!(applied.cursor, 1);
    }

    #[test]
    fn rejects_a_stale_command_precondition() {
        let (_temp, _source, mut workspace) = workspace();
        let first = EditCommand::ReplaceText {
            resource: ResourceKey::new("script", 2009),
            before: "void main() {}".to_owned(),
            after: "void main() { int n = 1; }".to_owned(),
        };
        workspace
            .stage_resource(
                ResourceKey::new("script", 2009),
                Some(b"void main() {}"),
                b"void main() { int n = 1; }",
            )
            .expect("stage first edit");
        workspace.apply(first).expect("first edit");
        let stale = EditCommand::ReplaceText {
            resource: ResourceKey::new("script", 2009),
            before: "void main() {}".to_owned(),
            after: "void main() { int n = 2; }".to_owned(),
        };
        workspace
            .stage_resource(
                ResourceKey::new("script", 2009),
                Some(b"void main() {}"),
                b"void main() { int n = 2; }",
            )
            .expect("stage stale edit");
        let error = workspace.apply(stale).expect_err("stale command");
        assert_eq!(error.code, "EDIT_PRECONDITION_FAILED");
    }

    #[test]
    fn rejected_resource_transaction_preserves_the_redo_branch() {
        let (_temp, _source, mut workspace) = workspace();
        let resource = ResourceKey::new("script", 2009);
        workspace
            .stage_resource(resource.clone(), Some(b"old"), b"new")
            .expect("stage initial edit");
        workspace
            .apply(EditCommand::ReplaceText {
                resource: resource.clone(),
                before: "old".to_owned(),
                after: "new".to_owned(),
            })
            .expect("apply initial edit");
        workspace.undo().expect("undo initial edit");

        workspace
            .stage_resource(resource, Some(b"old"), b"other")
            .expect("stage mismatched edit");
        let error = workspace
            .apply(EditCommand::ReplaceText {
                resource: ResourceKey::new("different", 2009),
                before: "old".to_owned(),
                after: "other".to_owned(),
            })
            .expect_err("reject mismatched resource transaction");
        assert_eq!(error.code, "EDIT_RESOURCE_TRANSACTION_REQUIRED");

        let snapshot = workspace.snapshot().expect("snapshot after rejection");
        assert_eq!(snapshot.cursor, 0);
        assert!(snapshot.can_redo);
        workspace.redo().expect("redo branch remains usable");
    }

    #[test]
    fn recovers_an_interrupted_staged_resource_when_reopening() {
        let (temp, _source, mut workspace) = workspace();
        let resource = ResourceKey::new("script", 2009);
        let staged = workspace
            .stage_resource(resource, Some(b"old"), b"new")
            .expect("stage resource");
        let old_hash = sha256_bytes(b"old");
        assert_eq!(staged.source_sha256.as_deref(), Some(old_hash.as_str()));
        assert_eq!(
            fs::read(temp.path().join("workspace/resources/script.nss")).expect("overlay"),
            b"new"
        );
        let reopened = EditWorkspace::open(temp.path().join("workspace")).expect("reopen");
        assert!(
            reopened
                .snapshot()
                .expect("snapshot")
                .modified_resources
                .is_empty()
        );
        assert!(!temp.path().join("workspace/resources/script.nss").exists());
    }

    #[test]
    fn undo_and_redo_restore_the_staged_resource_bytes() {
        let (temp, _source, mut workspace) = workspace();
        let resource = ResourceKey::new("script", 2009);
        workspace
            .stage_resource(resource.clone(), Some(b"old"), b"new")
            .expect("stage resource");
        workspace
            .apply(EditCommand::ReplaceText {
                resource: resource.clone(),
                before: "old".to_owned(),
                after: "new".to_owned(),
            })
            .expect("apply staged edit");
        workspace.undo().expect("undo staged edit");
        assert!(
            workspace
                .staged_resource_bytes(&resource)
                .expect("overlay")
                .is_none()
        );
        workspace.redo().expect("redo staged edit");
        assert_eq!(
            workspace
                .staged_resource_bytes(&resource)
                .expect("overlay")
                .as_deref(),
            Some(b"new".as_slice())
        );
        assert_eq!(
            fs::read(temp.path().join("workspace/resources/script.nss")).expect("resource"),
            b"new"
        );
    }

    #[test]
    fn resource_deletion_is_reversible_and_excluded_from_builds() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let source = temp.path().join("source.mod");
        let source_bytes = write_erf(
            "MOD ",
            &[
                ErfResourceInput {
                    key: ResourceKey::new("module", 2014),
                    bytes: b"ifo".to_vec(),
                },
                ErfResourceInput {
                    key: ResourceKey::new("obsolete", 2009),
                    bytes: b"old".to_vec(),
                },
            ],
        )
        .expect("source MOD");
        fs::write(&source, &source_bytes).expect("source fixture");
        let mut workspace = EditWorkspace::create(
            temp.path().join("workspace"),
            &source,
            &sha256_bytes(&source_bytes),
            source_bytes.len() as u64,
        )
        .expect("workspace");
        let key = ResourceKey::new("obsolete", 2009);
        let deleted = workspace
            .delete_resource(key.clone(), Some(b"old"))
            .expect("delete");
        assert_eq!(deleted.deleted_resources, vec![key.clone()]);
        workspace.undo().expect("undo delete");
        assert!(
            workspace
                .snapshot()
                .expect("snapshot")
                .deleted_resources
                .is_empty()
        );
        workspace.redo().expect("redo delete");
        let output = temp.path().join("rebuilt.mod");
        let report = workspace
            .build_module(&output)
            .expect("build without deleted resource");
        assert_eq!(report.resource_count, 1);
        workspace.undo().expect("restore source before build");
    }

    #[test]
    fn creates_a_reopenable_empty_module_with_an_entry_area() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let output = temp.path().join("new-module.mod");
        let report = create_empty_module(
            &output,
            &NewModuleDefinition {
                name: "Nouveau module".to_owned(),
                tag: "NEW_MODULE".to_owned(),
                entry_area: "startarea".to_owned(),
                tileset: "tno01".to_owned(),
            },
        )
        .expect("new module");
        assert_eq!(report.resource_count, 4);
        let inventory = ErfReader::default()
            .read_inventory(&output, &AtomicBool::new(false))
            .expect("reopen module");
        assert!(
            inventory
                .resources
                .iter()
                .any(|value| value.key == ResourceKey::new("module", 2014))
        );
        assert!(
            inventory
                .resources
                .iter()
                .any(|value| value.key == ResourceKey::new("startarea", 2012))
        );
    }

    #[test]
    fn validates_walkmeshes_and_keeps_ai_changes_as_previews() {
        let valid = validate_walkmesh(&WalkmeshDraft {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            surface_ids: vec![1],
            ..WalkmeshDraft::default()
        });
        assert!(valid.valid);
        let invalid = validate_walkmesh(&WalkmeshDraft {
            vertices: vec![[f32::NAN, 0.0, 0.0]],
            faces: vec![[0, 0, 4]],
            surface_ids: vec![1],
            ..WalkmeshDraft::default()
        });
        assert!(!invalid.valid);

        let (_temp, _source, workspace) = workspace();
        let preview = workspace.preview_ai_change_set(&AiChangeSet {
            summary: "Renommer le module".to_owned(),
            commands: vec![
                EditCommand::SetField {
                    resource: ResourceKey::new("module", 2014),
                    path: "Mod_Tag".to_owned(),
                    before: json!("OLD"),
                    after: json!("INTERMEDIATE"),
                },
                EditCommand::SetField {
                    resource: ResourceKey::new("module", 2014),
                    path: "Mod_Tag".to_owned(),
                    before: json!("INTERMEDIATE"),
                    after: json!("NEW"),
                },
            ],
        });
        assert!(preview.all_valid);
        assert_eq!(preview.previews[1].current, json!("INTERMEDIATE"));
        assert_eq!(workspace.snapshot().expect("snapshot").cursor, 0);
    }

    #[test]
    fn validates_applies_and_undoes_a_controlled_ai_change_set() {
        let (_temp, _source, mut workspace) = workspace();
        let resources = new_module_resources(&NewModuleDefinition {
            name: "AI fixture".to_owned(),
            tag: "OLD".to_owned(),
            entry_area: "startarea".to_owned(),
            tileset: "tno01".to_owned(),
        })
        .expect("module resources");
        let module = resources
            .iter()
            .find(|entry| entry.key == ResourceKey::new("module", 2014))
            .expect("module IFO")
            .bytes
            .clone();
        let mut sources = BTreeMap::new();
        sources.insert(ResourceKey::new("module", 2014).to_string(), module);
        let change_set = AiChangeSet {
            summary: "Renommer le tag du module".to_owned(),
            commands: vec![EditCommand::SetField {
                resource: ResourceKey::new("module", 2014),
                path: "Mod_Tag".to_owned(),
                before: json!({"kind": "string", "value": "OLD"}),
                after: json!({"kind": "string", "value": "NEW"}),
            }],
        };
        let digest = ai_change_set_sha256(&change_set).expect("proposal digest");
        let preview = workspace
            .preview_controlled_ai_change_set(&change_set, &sources)
            .expect("controlled preview");
        assert!(preview.all_valid);
        assert_eq!(workspace.snapshot().expect("preview snapshot").cursor, 0);

        let report = workspace
            .apply_controlled_ai_change_set(&change_set, &digest, &sources)
            .expect("apply proposal");
        assert_eq!(report.applied_commands, 1);
        assert_eq!(report.proposal_sha256, digest);
        assert_eq!(report.workspace.cursor, 1);
        workspace.undo().expect("undo AI command");
        assert!(
            workspace
                .snapshot()
                .expect("undo snapshot")
                .modified_resources
                .is_empty()
        );
    }

    #[test]
    fn rejects_uncontrolled_ai_commands_and_changed_confirmations() {
        let (_temp, _source, mut workspace) = workspace();
        let unsupported = AiChangeSet {
            summary: "Déplacer une instance".to_owned(),
            commands: vec![EditCommand::MoveInstance {
                area: "startarea".to_owned(),
                instance_id: "creature:0".to_owned(),
                before: Transform {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    bearing: 0.0,
                },
                after: Transform {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                    bearing: 0.0,
                },
            }],
        };
        assert_eq!(
            validate_controlled_ai_change_set(&unsupported)
                .expect_err("unsupported command")
                .code,
            "EDIT_AI_COMMAND_UNSUPPORTED"
        );

        let change_set = AiChangeSet {
            summary: "Modifier le script".to_owned(),
            commands: vec![EditCommand::ReplaceText {
                resource: ResourceKey::new("start", 2009),
                before: "void main() {}".to_owned(),
                after: "void main() { int value = 1; }".to_owned(),
            }],
        };
        let mut sources = BTreeMap::new();
        sources.insert(
            ResourceKey::new("start", 2009).to_string(),
            b"void main() {}".to_vec(),
        );
        assert_eq!(
            workspace
                .apply_controlled_ai_change_set(&change_set, &"0".repeat(64), &sources)
                .expect_err("changed confirmation")
                .code,
            "EDIT_AI_PROPOSAL_CHANGED"
        );
    }

    #[test]
    fn completes_a_module_lifecycle_without_aurora_and_preserves_the_source() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let source = temp.path().join("source.mod");
        create_empty_module(
            &source,
            &NewModuleDefinition {
                name: "OpenNever standalone".to_owned(),
                tag: "STANDALONE_OLD".to_owned(),
                entry_area: "startarea".to_owned(),
                tileset: "tno01".to_owned(),
            },
        )
        .expect("create source module");
        let source_before = fs::read(&source).expect("source bytes");
        let mut workspace = EditWorkspace::create(
            temp.path().join("workspace"),
            &source,
            &sha256_bytes(&source_before),
            source_before.len() as u64,
        )
        .expect("standalone workspace");
        let reader = ErfReader::default();
        let inventory = reader
            .read_inventory(&source, &AtomicBool::new(false))
            .expect("source inventory");
        let module_entry = inventory
            .resources
            .iter()
            .find(|entry| entry.key == ResourceKey::new("module", 2014))
            .expect("module IFO");
        let module_bytes = reader
            .read_resource(&source, module_entry, &AtomicBool::new(false))
            .expect("module IFO bytes");
        let mut sources = BTreeMap::new();
        sources.insert(ResourceKey::new("module", 2014).to_string(), module_bytes);
        let proposal = AiChangeSet {
            summary: "Finaliser le tag du module autonome".to_owned(),
            commands: vec![EditCommand::SetField {
                resource: ResourceKey::new("module", 2014),
                path: "Mod_Tag".to_owned(),
                before: json!({"kind": "string", "value": "STANDALONE_OLD"}),
                after: json!({"kind": "string", "value": "STANDALONE_READY"}),
            }],
        };
        let digest = ai_change_set_sha256(&proposal).expect("proposal digest");
        workspace
            .apply_controlled_ai_change_set(&proposal, &digest, &sources)
            .expect("apply standalone edit");
        let output = temp.path().join("standalone-ready.mod");
        let report = workspace
            .build_module(&output)
            .expect("build standalone module");
        assert!(report.source_intact);
        assert_eq!(
            fs::read(&source).expect("source after build"),
            source_before
        );
        let built_inventory = reader
            .read_inventory(&output, &AtomicBool::new(false))
            .expect("built inventory");
        let built_module = built_inventory
            .resources
            .iter()
            .find(|entry| entry.key == ResourceKey::new("module", 2014))
            .expect("built module IFO");
        let built_info = reader
            .read_resource(&output, built_module, &AtomicBool::new(false))
            .and_then(|bytes| read_module_info(&bytes, "standalone-ready::module.ifo"))
            .expect("reopen built module info");
        assert_eq!(built_info.tag, "STANDALONE_READY");
    }

    #[test]
    fn serializes_and_reopens_all_walkmesh_resource_kinds() {
        for kind in [WalkmeshKind::Wok, WalkmeshKind::Pwk, WalkmeshKind::Dwk] {
            let mut draft = WalkmeshDraft {
                vertices: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
                faces: vec![[0, 1, 2]],
                surface_ids: vec![3],
                ..WalkmeshDraft::default()
            };
            split_walkmesh_face(&mut draft, 0).expect("split face");
            assert_eq!(draft.vertices.len(), 4);
            assert_eq!(draft.faces.len(), 3);
            assert_eq!(draft.surface_ids, [3, 3, 3]);

            let bytes = serialize_walkmesh_ascii("onf_test", kind, &draft).expect("serialize");
            let text = std::str::from_utf8(&bytes).expect("ASCII walkmesh");
            let document = inspect_walkmesh("onf_test", kind, &bytes).expect("reopen");
            assert_eq!(document.kind, kind);
            assert_eq!(document.source_format, "ascii");
            assert_eq!(document.draft.faces, draft.faces);
            assert_eq!(document.draft.surface_ids, draft.surface_ids);
            match kind {
                WalkmeshKind::Wok => {
                    assert!(text.contains("#NWmax WALKMESH  ASCII"));
                    assert!(text.contains("beginwalkmeshgeom onf_test"));
                    assert!(text.contains("\n    aabb "));
                    assert!(document.draft.variants.is_empty());
                    assert!(document.draft.hooks.is_empty());
                }
                WalkmeshKind::Pwk => {
                    assert!(text.contains("#NWmax PWKMESH  ASCII"));
                    assert_eq!(document.draft.hooks.len(), 2);
                }
                WalkmeshKind::Dwk => {
                    assert!(text.contains("#NWmax DWKMESH  ASCII"));
                    assert_eq!(document.draft.variants.len(), 2);
                    assert_eq!(document.draft.hooks.len(), 6);
                }
            }
        }
    }

    #[test]
    fn applies_all_lot20_topology_operations_with_validation() {
        let square = WalkmeshDraft {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 2.0, 0.0],
                [0.0, 2.0, 0.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            surface_ids: vec![1, 1],
            ..WalkmeshDraft::default()
        };

        let mut split = square.clone();
        assert!(
            apply_walkmesh_operation(&mut split, &WalkmeshOperation::SplitFace { face_index: 0 })
                .expect("split")
                .valid
        );
        assert_eq!((split.vertices.len(), split.faces.len()), (5, 4));

        let mut removed = square.clone();
        assert!(
            apply_walkmesh_operation(
                &mut removed,
                &WalkmeshOperation::RemoveFace { face_index: 1 }
            )
            .expect("remove")
            .valid
        );
        assert_eq!((removed.vertices.len(), removed.faces.len()), (3, 1));

        let mut extruded = WalkmeshDraft {
            vertices: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            faces: vec![[0, 1, 2]],
            surface_ids: vec![3],
            ..WalkmeshDraft::default()
        };
        assert!(
            apply_walkmesh_operation(
                &mut extruded,
                &WalkmeshOperation::ExtrudeFace {
                    face_index: 0,
                    distance: 1.0,
                }
            )
            .expect("extrude")
            .valid
        );
        assert_eq!((extruded.vertices.len(), extruded.faces.len()), (6, 8));

        let mut welded = WalkmeshDraft {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.000_001, 0.0, 0.0],
            ],
            faces: vec![[0, 1, 2], [0, 3, 2]],
            surface_ids: vec![4, 4],
            ..WalkmeshDraft::default()
        };
        assert!(
            apply_walkmesh_operation(
                &mut welded,
                &WalkmeshOperation::WeldVertices { tolerance: 0.001 }
            )
            .expect("weld")
            .valid
        );
        assert_eq!((welded.vertices.len(), welded.faces.len()), (3, 1));

        let mut adjusted = square;
        assert!(
            apply_walkmesh_operation(
                &mut adjusted,
                &WalkmeshOperation::MoveVertex {
                    vertex_index: 2,
                    position: [2.0, 2.0, 0.5],
                }
            )
            .expect("move")
            .valid
        );
        assert!(
            apply_walkmesh_operation(
                &mut adjusted,
                &WalkmeshOperation::SetSurface {
                    face_index: 1,
                    surface_id: 18,
                }
            )
            .expect("surface")
            .valid
        );
        assert_eq!(adjusted.surface_ids, [1, 18]);
    }

    #[test]
    fn preserves_legacy_hook_only_pwk_resources() {
        let source = b"#MAXDOOR ASCII\n# model: plc_hook_pwk\nnode dummy plc_hook_pwk_use01\n  parent plc_hook_pwk\n  position 0 1 0\n  orientation 0 0 0 0\nendnode\n";
        let document =
            inspect_walkmesh("plc_hook", WalkmeshKind::Pwk, source).expect("import hook-only PWK");
        assert!(document.draft.faces.is_empty());
        assert_eq!(document.draft.hooks.len(), 1);
        assert!(validate_walkmesh_for_kind(&document.draft, WalkmeshKind::Pwk).valid);
        let output = serialize_walkmesh_ascii("plc_hook", WalkmeshKind::Pwk, &document.draft)
            .expect("serialize hook-only PWK");
        let reopened =
            inspect_walkmesh("plc_hook", WalkmeshKind::Pwk, &output).expect("reopen hook-only PWK");
        assert_eq!(reopened.draft.hooks, document.draft.hooks);
        assert!(reopened.draft.faces.is_empty());
    }

    #[test]
    fn rejects_zero_area_walkmesh_faces() {
        let invalid = validate_walkmesh(&WalkmeshDraft {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            faces: vec![[0, 1, 2]],
            surface_ids: vec![1],
            ..WalkmeshDraft::default()
        });
        assert!(!invalid.valid);
    }

    #[test]
    fn scans_an_aurora_workspace_without_following_symlinks_or_reading_unknown_files() {
        let temp = tempfile::tempdir().expect("temporary directory");
        fs::write(temp.path().join("script.nss"), b"void main() {}").expect("NSS");
        fs::write(temp.path().join("ignored.json"), b"not a resource").expect("JSON");
        let manifest = scan_aurora_workspace(temp.path()).expect("scan");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].name, "script.nss");
    }

    #[test]
    fn synchronizes_toolset_files_with_recoverable_backups() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let path = temp.path().join("script.nss");
        fs::write(&path, b"old").expect("old source");
        let backup = write_aurora_workspace_file(temp.path(), "script.nss", Some(b"new"))
            .expect("write")
            .expect("backup");
        assert_eq!(fs::read(&path).expect("new source"), b"new");
        assert_eq!(fs::read(&backup).expect("backup source"), b"old");
        let removed_backup = write_aurora_workspace_file(temp.path(), "script.nss", None)
            .expect("remove")
            .expect("removal backup");
        assert!(!path.exists());
        assert_eq!(
            fs::read(removed_backup).expect("removed source backup"),
            b"new"
        );
        assert!(
            scan_aurora_workspace(temp.path())
                .expect("scan")
                .files
                .is_empty()
        );
    }

    #[test]
    fn migrates_a_legacy_workspace_with_an_exact_backup() {
        let (_temp, _source, workspace) = workspace();
        let root = workspace.root.clone();
        drop(workspace);
        let state_path = root.join("workspace.json");
        let mut state = serde_json::from_slice::<serde_json::Value>(
            &fs::read(&state_path).expect("workspace state"),
        )
        .expect("workspace json");
        state["schemaVersion"] = serde_json::json!(1);
        state
            .as_object_mut()
            .expect("workspace object")
            .remove("migrationHistory");
        let legacy = serde_json::to_vec_pretty(&state).expect("legacy json");
        fs::write(&state_path, &legacy).expect("legacy workspace");

        let migrated = EditWorkspace::open(&root).expect("migrate workspace");
        let snapshot = migrated.snapshot().expect("snapshot");
        assert_eq!(snapshot.schema_version, EDIT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(snapshot.migration_history.len(), 1);
        assert_eq!(
            fs::read(root.join("workspace.json.v1.bak")).expect("migration backup"),
            legacy
        );
    }

    #[test]
    fn validates_reproducible_build_profiles_and_hak_outputs() {
        let profile = ModuleBuildProfile {
            name: "Test local".to_owned(),
            output_name: "test.mod".to_owned(),
            block_on_warnings: true,
            deploy_development: false,
            hak_files: vec!["custom.hak".to_owned()],
            custom_tlk: Some("dialog.tlk".to_owned()),
        };
        validate_build_profile(&profile).expect("profile");
        let (_workspace_temp, _source, workspace) = workspace();
        let saved = workspace
            .save_build_profile(profile.clone())
            .expect("save profile");
        assert_eq!(saved, vec![profile.clone()]);
        assert_eq!(
            workspace.list_build_profiles().expect("list profiles"),
            vec![profile]
        );
        let temp = tempfile::tempdir().expect("temporary directory");
        let report = build_custom_hak(
            &temp.path().join("custom.hak"),
            &[ErfResourceInput {
                key: ResourceKey::new("custom", 2017),
                bytes: b"2DA V2.0\n".to_vec(),
            }],
        )
        .expect("HAK");
        assert_eq!(report.resource_count, 1);
    }

    #[test]
    fn edits_module_hak_and_tlk_dependencies_without_losing_unknown_fields() {
        let resources = new_module_resources(&NewModuleDefinition {
            name: "Dependency Test".into(),
            tag: "DEPENDENCY_TEST".into(),
            entry_area: "startarea".into(),
            tileset: "tno01".into(),
        })
        .expect("module resources");
        let ifo = resources
            .iter()
            .find(|resource| resource.key == ResourceKey::new("module", 2014))
            .expect("module.ifo");
        let (bytes, reopened) = edit_module_dependencies(
            &ifo.bytes,
            "module.ifo",
            &["first.hak".into(), "second".into()],
            Some("custom.tlk"),
        )
        .expect("edit dependencies");
        assert_eq!(write_gff(&reopened).expect("rewrite"), bytes);
        assert!(matches!(
            find_field(&reopened.root, &["Mod_HakList"]).map(|field| &field.value),
            Some(GenericValue::List(values)) if values.len() == 2
        ));
        assert_eq!(
            find_field(&reopened.root, &["Mod_CustomTlk"]).map(|field| &field.value),
            Some(&GenericValue::String("custom.tlk".into()))
        );
        assert!(find_field(&reopened.root, &["Mod_Entry_Area"]).is_some());
    }

    #[test]
    fn verifies_two_identical_module_builds_from_a_profile() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let source = temp.path().join("source.mod");
        create_empty_module(
            &source,
            &NewModuleDefinition {
                name: "Reproducible".into(),
                tag: "REPRODUCIBLE".into(),
                entry_area: "startarea".into(),
                tileset: "tno01".into(),
            },
        )
        .expect("create source module");
        let source_hash = sha256_file(&source).expect("source hash");
        let source_size = fs::metadata(&source).expect("source metadata").len();
        let workspace = EditWorkspace::create(
            temp.path().join("workspace"),
            &source,
            &source_hash,
            source_size,
        )
        .expect("workspace");
        let verification = workspace
            .verify_reproducible_build(&ModuleBuildProfile {
                name: "Double build".into(),
                output_name: "double.mod".into(),
                block_on_warnings: true,
                deploy_development: false,
                hak_files: Vec::new(),
                custom_tlk: None,
            })
            .expect("verification");
        assert!(verification.identical);
        assert_eq!(verification.first_sha256, verification.second_sha256);
        assert_eq!(verification.resource_count, 4);
    }

    #[test]
    fn reports_git_branch_and_untracked_files_without_mutating_the_repository() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let initialized = Command::new("git")
            .arg("init")
            .arg("--initial-branch=main")
            .arg(temp.path())
            .status()
            .expect("git init");
        assert!(initialized.success());
        fs::write(temp.path().join("module.nss"), b"void main() {}\n").expect("untracked file");
        let status = inspect_git_repository(temp.path()).expect("git status");
        assert_eq!(status.branch, "main");
        assert!(!status.clean);
        assert_eq!(status.files[0].path, "module.nss");
        assert_eq!(status.files[0].index_status, "?");
        assert_eq!(status.files[0].worktree_status, "?");
    }

    #[test]
    fn persists_only_bounded_nwn_launch_profiles() {
        let (_temp, _source, workspace) = workspace();
        let executable = workspace.root.join("nwserver.exe");
        fs::write(&executable, b"synthetic executable fixture").expect("fixture executable");
        let profile = NwnLaunchProfile {
            name: "Serveur local".into(),
            mode: NwnLaunchMode::Server,
            executable_path: executable.display().to_string(),
            working_directory: workspace.root.display().to_string(),
            arguments: vec!["-module".into(), "opennever-test".into()],
        };
        validate_launch_profile(&profile).expect("launch profile");
        assert_eq!(
            workspace
                .save_launch_profile(profile.clone())
                .expect("save launch profile"),
            vec![profile]
        );
    }

    #[test]
    fn adds_a_typed_instance_to_the_expected_git_list() {
        let resources =
            create_area_resources("town", "Ville", "tno01", 1, 1, 0).expect("area resources");
        let git = resources
            .iter()
            .find(|resource| resource.key.resource_type == 2023)
            .expect("GIT");
        let (output, instance_id) = add_area_instance(
            &git.bytes,
            "town.git",
            "town",
            &InstancePlacement {
                category: "door".to_owned(),
                template_resref: "door01".to_owned(),
                tag: "exit".to_owned(),
                x: 5.0,
                y: 6.0,
                z: 0.0,
                bearing: 1.5,
                linked_to: Some("destination".to_owned()),
            },
        )
        .expect("add instance");
        assert_eq!(instance_id, "town:Door List:0");
        let mut reopened = parse_gff(&output, "town.git").expect("reopen GIT");
        let doors = list_mut(&mut reopened.root, "Door List")
            .expect("door list")
            .len();
        assert_eq!(doors, 1);
    }

    #[test]
    fn detects_a_changed_source_before_reporting_intact() {
        let (_temp, source, workspace) = workspace();
        fs::write(source, b"changed module").expect("mutate fixture");
        assert!(!workspace.snapshot().expect("snapshot").source_intact);
    }

    #[test]
    fn edits_a_typed_gff_field_and_reopens_it() {
        let document = GenericGff {
            file_type: "UTC ".to_owned(),
            file_version: "V3.2".to_owned(),
            source: "creature.utc".to_owned(),
            struct_count: 1,
            field_count: 1,
            root: GenericStruct {
                index: 0,
                struct_type: u32::MAX,
                fields: vec![GenericField {
                    label: "Tag".to_owned(),
                    field_type: 10,
                    value: GenericValue::String("OLD".to_owned()),
                }],
            },
        };
        let bytes = write_gff(&document).expect("fixture GFF");
        let before = serde_json::to_value(GenericValue::String("OLD".to_owned())).expect("before");
        let after = serde_json::to_value(GenericValue::String("NEW".to_owned())).expect("after");
        let (_output, reopened) =
            edit_gff_field(&bytes, "creature.utc", "/Tag", &before, &after).expect("edit");
        assert_eq!(
            reopened.root.fields[0].value,
            GenericValue::String("NEW".to_owned())
        );
    }

    #[test]
    fn rejects_an_invalid_blueprint_resref_before_writing() {
        let document = gff_document(
            "UTC ",
            "creature",
            vec![gff_field(
                "TemplateResRef",
                11,
                GenericValue::ResRef("creature01".to_owned()),
            )],
        );
        let bytes = write_gff(&document).expect("blueprint fixture");
        let before =
            serde_json::to_value(GenericValue::ResRef("creature01".to_owned())).expect("before");
        let after =
            serde_json::to_value(GenericValue::ResRef("INVALID RESREF".to_owned())).expect("after");
        let error = edit_gff_field(&bytes, "creature.utc", "/TemplateResRef", &before, &after)
            .expect_err("invalid ResRef must be rejected");
        assert_eq!(error.code, "EDIT_RESREF_INVALID");
    }

    #[test]
    fn edits_a_nested_dialogue_node_field_without_flattening_the_gff() {
        let document = gff_document(
            "DLG ",
            "dialogue",
            vec![gff_field(
                "EntryList",
                15,
                GenericValue::List(vec![GenericStruct {
                    index: 1,
                    struct_type: 0,
                    fields: vec![
                        gff_field("Speaker", 10, GenericValue::String("OLD".to_owned())),
                        gff_field(
                            "RepliesList",
                            15,
                            GenericValue::List(vec![GenericStruct {
                                index: 2,
                                struct_type: 0,
                                fields: vec![gff_field("Index", 4, GenericValue::Dword(0))],
                            }]),
                        ),
                    ],
                }]),
            )],
        );
        let bytes = write_gff(&document).expect("dialogue fixture");
        let before = serde_json::to_value(GenericValue::String("OLD".to_owned())).expect("before");
        let after = serde_json::to_value(GenericValue::String("NEW".to_owned())).expect("after");
        let (output, reopened) = edit_gff_field(
            &bytes,
            "dialogue.dlg",
            "/EntryList/0/Speaker",
            &before,
            &after,
        )
        .expect("nested dialogue edit");
        let GenericValue::List(entries) = &reopened.root.fields[0].value else {
            panic!("EntryList must remain a list");
        };
        assert_eq!(
            entries[0].fields[0].value,
            GenericValue::String("NEW".to_owned())
        );
        assert!(matches!(entries[0].fields[1].value, GenericValue::List(_)));
        let before_index = serde_json::to_value(GenericValue::Dword(0)).expect("before index");
        let after_index = serde_json::to_value(GenericValue::Dword(3)).expect("after index");
        let (_output, reopened) = edit_gff_field(
            &output,
            "dialogue.dlg",
            "/EntryList/0/RepliesList/0/Index",
            &before_index,
            &after_index,
        )
        .expect("nested dialogue link edit");
        let GenericValue::List(entries) = &reopened.root.fields[0].value else {
            panic!("EntryList must remain a list");
        };
        let GenericValue::List(replies) = &entries[0].fields[1].value else {
            panic!("RepliesList must remain a list");
        };
        assert_eq!(replies[0].fields[0].value, GenericValue::Dword(3));
    }

    #[test]
    fn edits_one_dialogue_localization_without_losing_strref_or_other_variants() {
        let original = LocalizedString {
            string_ref: Some(42),
            values: vec![
                LocalizedValue {
                    language_id: 0,
                    text: "Bonjour".to_owned(),
                },
                LocalizedValue {
                    language_id: 2,
                    text: "Hello".to_owned(),
                },
            ],
        };
        let mut edited = original.clone();
        edited.values[0].text = "Bonsoir".to_owned();
        let document = gff_document(
            "DLG ",
            "dialogue",
            vec![gff_field(
                "EntryList",
                15,
                GenericValue::List(vec![GenericStruct {
                    index: 1,
                    struct_type: 0,
                    fields: vec![gff_field(
                        "Text",
                        12,
                        GenericValue::LocalizedString(original.clone()),
                    )],
                }]),
            )],
        );
        let bytes = write_gff(&document).expect("dialogue fixture");
        let before = serde_json::to_value(GenericValue::LocalizedString(original)).expect("before");
        let after =
            serde_json::to_value(GenericValue::LocalizedString(edited.clone())).expect("after");
        let (_output, reopened) =
            edit_gff_field(&bytes, "dialogue.dlg", "/EntryList/0/Text", &before, &after)
                .expect("localized dialogue edit");
        let GenericValue::List(entries) = &reopened.root.fields[0].value else {
            panic!("EntryList must remain a list");
        };
        assert_eq!(
            entries[0].fields[0].value,
            GenericValue::LocalizedString(edited)
        );
    }

    #[test]
    fn dialogue_structure_operations_retarget_links_without_leaving_broken_indexes() {
        let link = |index| GenericStruct {
            index: 0,
            struct_type: 0,
            fields: vec![gff_field("Index", 4, GenericValue::Dword(index))],
        };
        let node = |outgoing: &str, targets: &[u32]| GenericStruct {
            index: 0,
            struct_type: 0,
            fields: vec![
                gff_field(
                    "Text",
                    12,
                    GenericValue::LocalizedString(LocalizedString {
                        string_ref: None,
                        values: Vec::new(),
                    }),
                ),
                gff_field(
                    outgoing,
                    15,
                    GenericValue::List(targets.iter().copied().map(link).collect()),
                ),
            ],
        };
        let document = gff_document(
            "DLG ",
            "dialogue",
            vec![
                gff_field(
                    "EntryList",
                    15,
                    GenericValue::List(vec![node("RepliesList", &[0, 1])]),
                ),
                gff_field(
                    "ReplyList",
                    15,
                    GenericValue::List(vec![node("EntriesList", &[0]), node("EntriesList", &[0])]),
                ),
                gff_field("StartingList", 15, GenericValue::List(vec![link(0)])),
            ],
        );
        let bytes = write_gff(&document).expect("dialogue fixture");
        let (bytes, _) = edit_dialogue_structure(
            &bytes,
            "dialogue.dlg",
            &DialogueStructureAction::AddNode {
                node_kind: DialogueNodeKind::Entry,
            },
        )
        .expect("add entry");
        let (bytes, _) = edit_dialogue_structure(
            &bytes,
            "dialogue.dlg",
            &DialogueStructureAction::AddLink {
                source: None,
                target: DialogueNodeRef {
                    kind: DialogueNodeKind::Entry,
                    index: 1,
                },
            },
        )
        .expect("add start link");
        let (_bytes, reopened) = edit_dialogue_structure(
            &bytes,
            "dialogue.dlg",
            &DialogueStructureAction::RemoveNode {
                node: DialogueNodeRef {
                    kind: DialogueNodeKind::Reply,
                    index: 0,
                },
            },
        )
        .expect("remove reply and retarget links");
        let entries = dialogue_nodes(&reopened.root, DialogueNodeKind::Entry).expect("entries");
        let GenericValue::List(links) = &entries[0]
            .fields
            .iter()
            .find(|field| field.label == "RepliesList")
            .expect("replies")
            .value
        else {
            panic!("RepliesList must remain a list");
        };
        assert_eq!(links.len(), 1);
        assert_eq!(
            find_field(&links[0], &["Index"]).map(|field| &field.value),
            Some(&GenericValue::Dword(0))
        );
        assert_eq!(
            dialogue_nodes(&reopened.root, DialogueNodeKind::Reply)
                .expect("replies")
                .len(),
            1
        );
        let starts = find_field(&reopened.root, &["StartingList"]).expect("starting list");
        assert!(matches!(&starts.value, GenericValue::List(values) if values.len() == 2));
    }

    #[test]
    fn dialogue_links_can_add_change_and_remove_trigger_scripts() {
        let link = GenericStruct {
            index: 0,
            struct_type: 0,
            fields: vec![
                gff_field("Index", 4, GenericValue::Dword(0)),
                gff_field("IsChild", 0, GenericValue::Byte(0)),
            ],
        };
        let document = gff_document(
            "DLG ",
            "dialogue",
            vec![
                gff_field("EntryList", 15, GenericValue::List(Vec::new())),
                gff_field("ReplyList", 15, GenericValue::List(Vec::new())),
                gff_field("StartingList", 15, GenericValue::List(vec![link])),
            ],
        );
        let bytes = write_gff(&document).expect("dialogue fixture");
        let (bytes, reopened) = edit_dialogue_structure(
            &bytes,
            "dialogue.dlg",
            &DialogueStructureAction::SetLinkScripts {
                source: None,
                position: 0,
                condition_script: Some("check_start".to_owned()),
                action_script: Some("begin_scene".to_owned()),
            },
        )
        .expect("associate trigger scripts");
        let GenericValue::List(starts) = &find_field(&reopened.root, &["StartingList"])
            .expect("starting list")
            .value
        else {
            panic!("StartingList must remain a list");
        };
        assert_eq!(
            find_field(&starts[0], &["Active"]).map(|field| &field.value),
            Some(&GenericValue::ResRef("check_start".to_owned()))
        );
        assert_eq!(
            find_field(&starts[0], &["Script"]).map(|field| &field.value),
            Some(&GenericValue::ResRef("begin_scene".to_owned()))
        );

        let (_bytes, reopened) = edit_dialogue_structure(
            &bytes,
            "dialogue.dlg",
            &DialogueStructureAction::SetLinkScripts {
                source: None,
                position: 0,
                condition_script: None,
                action_script: Some("next_scene".to_owned()),
            },
        )
        .expect("remove condition and change action");
        let GenericValue::List(starts) = &find_field(&reopened.root, &["StartingList"])
            .expect("starting list")
            .value
        else {
            panic!("StartingList must remain a list");
        };
        assert!(find_field(&starts[0], &["Active", "Conditional"]).is_none());
        assert_eq!(
            find_field(&starts[0], &["Script"]).map(|field| &field.value),
            Some(&GenericValue::ResRef("next_scene".to_owned()))
        );
    }

    #[test]
    fn dialogue_structure_actions_use_the_typescript_ipc_shape() {
        let value = serde_json::to_value(DialogueStructureAction::AddNode {
            node_kind: DialogueNodeKind::Entry,
        })
        .expect("serialize dialogue action");
        assert_eq!(value, json!({"kind": "add_node", "nodeKind": "entry"}));
        let parsed = serde_json::from_value::<DialogueStructureAction>(json!({
            "kind": "add_link",
            "source": {"kind": "entry", "index": 2},
            "target": {"kind": "reply", "index": 4}
        }))
        .expect("deserialize dialogue action");
        assert!(matches!(
            parsed,
            DialogueStructureAction::AddLink {
                source: Some(DialogueNodeRef {
                    kind: DialogueNodeKind::Entry,
                    index: 2
                }),
                target: DialogueNodeRef {
                    kind: DialogueNodeKind::Reply,
                    index: 4
                }
            }
        ));
        let value = serde_json::to_value(DialogueStructureAction::SetLinkScripts {
            source: None,
            position: 1,
            condition_script: Some("check_start".to_owned()),
            action_script: None,
        })
        .expect("serialize dialogue trigger action");
        assert_eq!(
            value,
            json!({
                "kind": "set_link_scripts",
                "source": null,
                "position": 1,
                "conditionScript": "check_start",
                "actionScript": null
            })
        );
    }

    #[test]
    fn journal_structure_adds_unique_categories_and_stable_entry_ids() {
        let document = gff_document(
            "JRL ",
            "module",
            vec![gff_field("Categories", 15, GenericValue::List(Vec::new()))],
        );
        let bytes = write_gff(&document).expect("journal fixture");
        let (bytes, _) = edit_journal_structure(
            &bytes,
            "module.jrl",
            &JournalStructureAction::AddCategory {
                tag: "main_quest".to_owned(),
            },
        )
        .expect("add category");
        let (bytes, _) = edit_journal_structure(
            &bytes,
            "module.jrl",
            &JournalStructureAction::AddEntry { category_index: 0 },
        )
        .expect("add first entry");
        let (_bytes, mut reopened) = edit_journal_structure(
            &bytes,
            "module.jrl",
            &JournalStructureAction::AddEntry { category_index: 0 },
        )
        .expect("add second entry");
        let categories = list_mut_any(&mut reopened.root, &["Categories"]).expect("category list");
        let GenericValue::List(entries) = &categories[0]
            .fields
            .iter()
            .find(|field| field.label == "EntryList")
            .expect("entry list")
            .value
        else {
            panic!("EntryList must remain a list");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(
            find_field(&entries[0], &["ID"]).map(|field| &field.value),
            Some(&GenericValue::Dword(0))
        );
        assert_eq!(
            find_field(&entries[1], &["ID"]).map(|field| &field.value),
            Some(&GenericValue::Dword(1))
        );
        let error = edit_journal_structure(
            &bytes,
            "module.jrl",
            &JournalStructureAction::AddCategory {
                tag: "MAIN_QUEST".to_owned(),
            },
        )
        .expect_err("duplicate category tag");
        assert_eq!(error.code, "EDIT_JOURNAL_TAG_DUPLICATE");
    }

    #[test]
    fn faction_structure_completes_the_matrix_and_retargets_references_on_removal() {
        let factions = vec![
            new_faction("PC", u32::MAX, 0),
            new_faction("Hostile", u32::MAX, 1),
            new_faction("Guild", 1, 2),
        ];
        let pairs = [
            (0, 1, 0),
            (0, 2, 50),
            (1, 1, 100),
            (1, 2, 0),
            (2, 1, 0),
            (2, 2, 100),
        ];
        let reputations = pairs
            .into_iter()
            .enumerate()
            .map(|(index, (source, target, value))| new_reputation(source, target, value, index))
            .collect();
        let document = gff_document(
            "FAC ",
            "repute",
            vec![
                gff_field("FactionList", 15, GenericValue::List(factions)),
                gff_field("RepList", 15, GenericValue::List(reputations)),
            ],
        );
        let bytes = write_gff(&document).expect("faction fixture");
        let (bytes, added) = edit_faction_structure(
            &bytes,
            "repute.fac",
            &FactionStructureAction::AddFaction {
                name: "Merchants".to_owned(),
                parent_id: Some(2),
            },
        )
        .expect("add faction");
        assert_eq!(faction_list(&added.root).expect("factions").len(), 4);
        let added_reputations = find_field(&added.root, &["RepList"]).expect("reputations");
        assert!(
            matches!(&added_reputations.value, GenericValue::List(values) if values.len() == 12)
        );

        let (_bytes, removed) = edit_faction_structure(
            &bytes,
            "repute.fac",
            &FactionStructureAction::RemoveFaction { faction_index: 1 },
        )
        .expect("remove faction");
        let remaining = faction_list(&removed.root).expect("remaining factions");
        assert_eq!(remaining.len(), 3);
        assert_eq!(faction_name(&remaining[1]), Some("Guild"));
        assert_eq!(
            find_field(&remaining[1], &["FactionParentID"]).map(|field| &field.value),
            Some(&GenericValue::Dword(u32::MAX))
        );
        assert_eq!(
            find_field(&remaining[2], &["FactionParentID"]).map(|field| &field.value),
            Some(&GenericValue::Dword(1))
        );
        let reputations = find_field(&removed.root, &["RepList"]).expect("remaining reputations");
        let GenericValue::List(reputations) = &reputations.value else {
            panic!("RepList must remain a list");
        };
        assert_eq!(reputations.len(), 6);
        for reputation in reputations {
            let (source, target) = reputation_ids(reputation).expect("valid reputation");
            assert!(source < 3 && (1..3).contains(&target));
        }
    }

    #[test]
    fn faction_structure_validates_pc_and_typescript_ipc_invariants() {
        let action = FactionStructureAction::AddReputation {
            source_id: 2,
            target_id: 3,
            value: 75,
        };
        assert_eq!(
            serde_json::to_value(action).expect("serialize faction action"),
            json!({"kind": "add_reputation", "sourceId": 2, "targetId": 3, "value": 75})
        );
        let document = gff_document(
            "FAC ",
            "repute",
            vec![
                gff_field(
                    "FactionList",
                    15,
                    GenericValue::List(vec![
                        new_faction("PC", u32::MAX, 0),
                        new_faction("Hostile", u32::MAX, 1),
                    ]),
                ),
                gff_field("RepList", 15, GenericValue::List(Vec::new())),
            ],
        );
        let bytes = write_gff(&document).expect("faction fixture");
        let remove_error = edit_faction_structure(
            &bytes,
            "repute.fac",
            &FactionStructureAction::RemoveFaction { faction_index: 0 },
        )
        .expect_err("PC faction is required");
        assert_eq!(remove_error.code, "EDIT_FACTION_PC_REMOVE_FORBIDDEN");
        let target_error = edit_faction_structure(
            &bytes,
            "repute.fac",
            &FactionStructureAction::AddReputation {
                source_id: 1,
                target_id: 0,
                value: 50,
            },
        )
        .expect_err("PC cannot be a target");
        assert_eq!(
            target_error.code,
            "EDIT_FACTION_REPUTATION_PC_TARGET_INVALID"
        );
    }

    #[test]
    fn blueprint_structure_adds_and_removes_typed_utc_entries() {
        let document = gff_document(
            "UTC ",
            "creature",
            vec![
                gff_field("FeatList", 15, GenericValue::List(Vec::new())),
                gff_field("SpecAbilityList", 15, GenericValue::List(Vec::new())),
                gff_field("ClassList", 15, GenericValue::List(Vec::new())),
                gff_field("Equip_ItemList", 15, GenericValue::List(Vec::new())),
            ],
        );
        let mut bytes = write_gff(&document).expect("creature fixture");
        for action in [
            BlueprintStructureAction::AddFeat { feat_id: 42 },
            BlueprintStructureAction::AddSpecialAbility {
                spell_id: 12,
                caster_level: 3,
                flags: 1,
            },
            BlueprintStructureAction::AddClass {
                class_id: 4,
                class_level: 5,
            },
            BlueprintStructureAction::AddEquippedItem {
                resref: "test_armor".to_owned(),
                slot: 2,
            },
        ] {
            bytes = edit_blueprint_structure(&bytes, "creature.utc", &action)
                .expect("add typed UTC entry")
                .0;
        }
        let (_, reopened) = edit_blueprint_structure(
            &bytes,
            "creature.utc",
            &BlueprintStructureAction::RemoveEntry {
                list_kind: BlueprintListKind::Feat,
                entry_index: 0,
            },
        )
        .expect("remove feat");
        for (label, expected) in [
            ("FeatList", 0),
            ("SpecAbilityList", 1),
            ("ClassList", 1),
            ("Equip_ItemList", 1),
        ] {
            assert!(matches!(
                find_field(&reopened.root, &[label]).map(|field| &field.value),
                Some(GenericValue::List(values)) if values.len() == expected
            ));
        }
    }

    #[test]
    fn blueprint_structure_covers_item_sound_and_encounter_lists() {
        let cases = [
            (
                "UTI ",
                "PropertiesList",
                BlueprintStructureAction::AddItemProperty {
                    property_name: 1,
                    subtype: 2,
                    cost_table: 3,
                    cost_value: 4,
                    param1: 5,
                    param1_value: 6,
                    chance_appear: 100,
                },
            ),
            (
                "UTS ",
                "Sounds",
                BlueprintStructureAction::AddSound {
                    resref: "as_test".to_owned(),
                },
            ),
            (
                "UTE ",
                "CreatureList",
                BlueprintStructureAction::AddEncounterCreature {
                    resref: "test_creature".to_owned(),
                    appearance: 55,
                    challenge_rating: 2.5,
                    single_spawn: true,
                },
            ),
        ];
        for (file_type, label, action) in cases {
            let document = gff_document(
                file_type,
                "blueprint",
                vec![gff_field(label, 15, GenericValue::List(Vec::new()))],
            );
            let bytes = write_gff(&document).expect("blueprint fixture");
            let (_, reopened) = edit_blueprint_structure(&bytes, "blueprint", &action)
                .expect("add blueprint entry");
            assert!(matches!(
                find_field(&reopened.root, &[label]).map(|field| &field.value),
                Some(GenericValue::List(values)) if values.len() == 1
            ));
        }
        assert_eq!(
            serde_json::to_value(BlueprintStructureAction::RemoveEntry {
                list_kind: BlueprintListKind::ItemProperty,
                entry_index: 3,
            })
            .expect("serialize blueprint action"),
            json!({"kind": "remove_entry", "listKind": "item_property", "entryIndex": 3})
        );
    }

    #[test]
    fn area_structure_edits_typed_geometry_spawn_points_and_transitions() {
        let trigger = GenericStruct {
            index: 1,
            struct_type: 1,
            fields: vec![
                gff_field("Geometry", 15, GenericValue::List(Vec::new())),
                gff_field("LinkedTo", 10, GenericValue::String(String::new())),
                gff_field("LinkedToFlags", 0, GenericValue::Byte(0)),
                gff_field("LoadScreenID", 2, GenericValue::Word(0)),
            ],
        };
        let encounter = GenericStruct {
            index: 2,
            struct_type: 1,
            fields: vec![
                gff_field("Geometry", 15, GenericValue::List(Vec::new())),
                gff_field("SpawnPointList", 15, GenericValue::List(Vec::new())),
            ],
        };
        let document = gff_document(
            "GIT ",
            "town",
            vec![
                gff_field("TriggerList", 15, GenericValue::List(vec![trigger])),
                gff_field("Encounter List", 15, GenericValue::List(vec![encounter])),
            ],
        );
        let mut bytes = write_gff(&document).expect("area fixture");
        bytes = edit_area_structure(
            &bytes,
            "town.git",
            "town",
            &AreaStructureAction::SetGeometry {
                instance_id: "town:TriggerList:0".to_owned(),
                points: vec![
                    AreaPoint {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    AreaPoint {
                        x: 2.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    AreaPoint {
                        x: 0.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ],
            },
            None,
        )
        .expect("set trigger polygon")
        .0;
        bytes = edit_area_structure(
            &bytes,
            "town.git",
            "town",
            &AreaStructureAction::SetTransition {
                instance_id: "town:TriggerList:0".to_owned(),
                destination: "destination_waypoint".to_owned(),
                flags: 2,
                load_screen_id: 7,
            },
            None,
        )
        .expect("set transition")
        .0;
        let (_, reopened) = edit_area_structure(
            &bytes,
            "town.git",
            "town",
            &AreaStructureAction::SetSpawnPoints {
                instance_id: "town:Encounter List:0".to_owned(),
                points: vec![AreaSpawnPoint {
                    x: 4.0,
                    y: 5.0,
                    z: 0.0,
                    orientation: 1.5,
                }],
            },
            None,
        )
        .expect("set encounter spawn points");
        let triggers = find_field(&reopened.root, &["TriggerList"]).expect("triggers");
        let GenericValue::List(triggers) = &triggers.value else {
            panic!("TriggerList must remain a list")
        };
        assert!(matches!(
            find_field(&triggers[0], &["Geometry"]).map(|field| &field.value),
            Some(GenericValue::List(points)) if points.len() == 3 && points[0].struct_type == 3
        ));
        assert_eq!(
            find_field(&triggers[0], &["LinkedToFlags"]).map(|field| &field.value),
            Some(&GenericValue::Byte(2))
        );
        let encounters = find_field(&reopened.root, &["Encounter List"]).expect("encounters");
        assert!(matches!(
            &encounters.value,
            GenericValue::List(values)
                if matches!(find_field(&values[0], &["SpawnPointList"]).map(|field| &field.value), Some(GenericValue::List(points)) if points.len() == 1 && points[0].struct_type == 2)
        ));
    }

    #[test]
    fn area_inventory_embeds_the_resolved_uti_and_preserves_unknown_fields() {
        let placeable = GenericStruct {
            index: 1,
            struct_type: 9,
            fields: vec![gff_field("HasInventory", 0, GenericValue::Byte(0))],
        };
        let git = gff_document(
            "GIT ",
            "town",
            vec![gff_field(
                "Placeable List",
                15,
                GenericValue::List(vec![placeable]),
            )],
        );
        let item = gff_document(
            "UTI ",
            "potion",
            vec![
                gff_field(
                    "TemplateResRef",
                    11,
                    GenericValue::ResRef("potion".to_owned()),
                ),
                gff_field("Tag", 10, GenericValue::String("POTION".to_owned())),
                gff_field("FutureField", 4, GenericValue::Dword(99)),
            ],
        );
        let bytes = write_gff(&git).expect("area fixture");
        let (_, reopened) = edit_area_structure(
            &bytes,
            "town.git",
            "town",
            &AreaStructureAction::AddInventoryItem {
                instance_id: "town:Placeable List:0".to_owned(),
                resref: "potion".to_owned(),
                stack_size: 3,
                x: 2,
                y: 1,
                infinite: false,
                category_index: None,
            },
            Some(&item),
        )
        .expect("add inventory item");
        let placeables = find_field(&reopened.root, &["Placeable List"]).expect("placeables");
        let GenericValue::List(placeables) = &placeables.value else {
            panic!("Placeable List must remain a list")
        };
        let items = find_field(&placeables[0], &["ItemList"]).expect("item list");
        let GenericValue::List(items) = &items.value else {
            panic!("ItemList must be a list")
        };
        assert_eq!(items.len(), 1);
        assert_eq!(
            find_field(&items[0], &["FutureField"]).map(|field| &field.value),
            Some(&GenericValue::Dword(99))
        );
        assert_eq!(
            find_field(&items[0], &["StackSize"]).map(|field| &field.value),
            Some(&GenericValue::Word(3))
        );
        assert_eq!(
            find_field(&placeables[0], &["HasInventory"]).map(|field| &field.value),
            Some(&GenericValue::Byte(1))
        );
    }

    #[test]
    fn area_store_inventory_targets_one_category_and_marks_infinite_stock() {
        let categories = (0..5)
            .map(|index| GenericStruct {
                index: index + 1,
                struct_type: index,
                fields: vec![gff_field("ItemList", 15, GenericValue::List(Vec::new()))],
            })
            .collect();
        let store = GenericStruct {
            index: 1,
            struct_type: 11,
            fields: vec![gff_field("StoreList", 15, GenericValue::List(categories))],
        };
        let git = gff_document(
            "GIT ",
            "town",
            vec![gff_field("StoreList", 15, GenericValue::List(vec![store]))],
        );
        let item = gff_document(
            "UTI ",
            "potion",
            vec![gff_field(
                "TemplateResRef",
                11,
                GenericValue::ResRef("potion".to_owned()),
            )],
        );
        let (_, reopened) = edit_area_structure(
            &write_gff(&git).expect("store fixture"),
            "town.git",
            "town",
            &AreaStructureAction::AddInventoryItem {
                instance_id: "town:StoreList:0".to_owned(),
                resref: "potion".to_owned(),
                stack_size: 1,
                x: 0,
                y: 0,
                infinite: true,
                category_index: Some(3),
            },
            Some(&item),
        )
        .expect("add store inventory item");
        let stores = find_field(&reopened.root, &["StoreList"]).expect("stores");
        let GenericValue::List(stores) = &stores.value else {
            panic!("StoreList must be a list")
        };
        let categories = find_field(&stores[0], &["StoreList"]).expect("categories");
        let GenericValue::List(categories) = &categories.value else {
            panic!("nested StoreList must be a list")
        };
        assert!(matches!(
            find_field(&categories[3], &["ItemList"]).map(|field| &field.value),
            Some(GenericValue::List(items))
                if items.len() == 1
                    && find_field(&items[0], &["Infinite"]).map(|field| &field.value)
                        == Some(&GenericValue::Byte(1))
        ));
        assert_eq!(
            serde_json::to_value(AreaStructureAction::RemoveInventoryItem {
                instance_id: "town:StoreList:0".to_owned(),
                item_index: 2,
                category_index: Some(3),
            })
            .expect("serialize area action"),
            json!({"kind":"remove_inventory_item","instanceId":"town:StoreList:0","itemIndex":2,"categoryIndex":3})
        );
    }

    #[test]
    fn builds_a_new_mod_and_cleans_only_unchanged_development_files() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let source = temp.path().join("source.mod");
        let source_bytes = write_erf(
            "MOD ",
            &[ErfResourceInput {
                key: ResourceKey::new("module", 2014),
                bytes: b"IFO!".to_vec(),
            }],
        )
        .expect("source MOD");
        fs::write(&source, &source_bytes).expect("source fixture");
        let mut workspace = EditWorkspace::create(
            temp.path().join("workspace"),
            &source,
            &sha256_bytes(&source_bytes),
            source_bytes.len() as u64,
        )
        .expect("workspace");
        workspace
            .stage_resource(ResourceKey::new("start", 2009), None, b"void main() {}")
            .expect("stage NSS");
        workspace
            .apply(EditCommand::ReplaceText {
                resource: ResourceKey::new("start", 2009),
                before: String::new(),
                after: "void main() {}".to_owned(),
            })
            .expect("commit NSS");
        let output = temp.path().join("output.mod");
        assert_eq!(
            workspace
                .build_module(&output)
                .expect_err("NSS requires NCS")
                .code,
            "EDIT_NSS_COMPILATION_STALE"
        );
        let ncs_bytes = b"NCS V1.0\0";
        workspace
            .stage_resource(ResourceKey::new("start", 2010), None, ncs_bytes)
            .expect("stage NCS");
        workspace
            .apply(EditCommand::CompileScript {
                resource: ResourceKey::new("start", 2010),
                inputs: vec![ResourceContentDigest {
                    resource: ResourceKey::new("start", 2009),
                    content_sha256: sha256_bytes(b"void main() {}"),
                }],
                compiler_sha256: sha256_bytes(b"test compiler"),
                before_sha256: None,
                after_sha256: sha256_bytes(ncs_bytes),
            })
            .expect("commit exact NCS compilation");
        let report = workspace.build_module(&output).expect("build MOD");
        assert_eq!(report.resource_count, 3);
        assert!(report.source_intact);
        assert_eq!(fs::read(&source).expect("source"), source_bytes);

        let user = temp.path().join("user");
        fs::create_dir(&user).expect("user data");
        workspace
            .create_resource(ResourceKey::new("shared", 2017), b"2DA V2.0\n")
            .expect("create shared development resource");
        let deployment = workspace.deploy_development(&user).expect("deploy");
        assert_eq!(deployment.files.len(), 3);
        let mut competing_workspace = EditWorkspace::create(
            temp.path().join("competing-workspace"),
            &source,
            &sha256_bytes(&source_bytes),
            source_bytes.len() as u64,
        )
        .expect("competing workspace");
        competing_workspace
            .create_resource(ResourceKey::new("shared", 2017), b"2DA V2.0\nconflict\n")
            .expect("create competing development resource");
        let conflict = competing_workspace
            .deploy_development(&user)
            .expect_err("cross-workspace deployment collision");
        assert_eq!(conflict.code, "EDIT_DEVELOPMENT_OWNERSHIP_CONFLICT");
        fs::write(user.join("development/start.nss"), b"external change").expect("external change");
        let cleanup = workspace.clean_development(&user).expect("cleanup");
        assert_eq!(cleanup.preserved_changed, vec!["start.nss"]);
        assert_eq!(cleanup.removed, vec!["shared.2da", "start.ncs"]);
        assert!(user.join("development/start.nss").is_file());
    }

    #[test]
    fn creates_and_undoes_an_atomic_area_resource_set() {
        let (_temp, _source, mut workspace) = workspace();
        let resources = vec![
            ErfResourceInput {
                key: ResourceKey::new("town", 2012),
                bytes: b"ARE".to_vec(),
            },
            ErfResourceInput {
                key: ResourceKey::new("town", 2023),
                bytes: b"GIT".to_vec(),
            },
            ErfResourceInput {
                key: ResourceKey::new("town", 2046),
                bytes: b"GIC".to_vec(),
            },
        ];
        let created = workspace
            .create_resources_atomic(&resources)
            .expect("atomic area creation");
        assert_eq!(created.modified_resources.len(), 3);
        assert_eq!(created.cursor, 1);
        let undone = workspace.undo().expect("undo area creation");
        assert!(undone.modified_resources.is_empty());
        let redone = workspace.redo().expect("redo area creation");
        assert_eq!(redone.modified_resources.len(), 3);
    }

    #[test]
    fn creates_a_parseable_dialogue_and_preserves_structured_editing() {
        let resource = create_dialogue_resource(
            "dlg_intro",
            "npc_guide",
            "Introduction",
            &["Bienvenue".to_owned(), "Le portail est ouvert".to_owned()],
        )
        .expect("dialogue resource");
        assert_eq!(resource.key, ResourceKey::new("dlg_intro", 2029));
        let document = parse_gff(&resource.bytes, "dlg_intro.dlg").expect("parse dialogue");
        assert_eq!(document.file_type, "DLG ");
        let (edited, _) = edit_dialogue_structure(
            &resource.bytes,
            "dlg_intro.dlg",
            &DialogueStructureAction::AddNode {
                node_kind: DialogueNodeKind::Reply,
            },
        )
        .expect("add reply");
        assert!(!edited.is_empty());
    }

    #[test]
    fn updates_module_manifest_without_replacing_unknown_fields() {
        let resources = new_module_resources(&NewModuleDefinition {
            name: "Avant".to_owned(),
            tag: "BEFORE".to_owned(),
            entry_area: "entry".to_owned(),
            tileset: "tno01".to_owned(),
        })
        .expect("module resources");
        let ifo = &resources[0].bytes;
        let (output, document) = edit_module_manifest(
            ifo,
            "module.ifo",
            &ModuleManifestDefinition {
                name: "Après".to_owned(),
                tag: "AFTER".to_owned(),
                description: "Construit par plan".to_owned(),
                entry_area: "entry".to_owned(),
                areas: vec!["entry".to_owned(), "cave".to_owned()],
                hak_files: vec!["content.hak".to_owned()],
                custom_tlk: Some("dialogue.tlk".to_owned()),
            },
        )
        .expect("edit manifest");
        assert!(!output.is_empty());
        assert!(find_field(&document.root, &["Mod_ID"]).is_some());
        let areas = find_field(&document.root, &["Mod_Area_list"]).expect("areas");
        assert!(matches!(&areas.value, GenericValue::List(values) if values.len() == 2));
    }

    #[test]
    fn rejects_an_ncs_after_its_nss_changes() {
        let (_temp, _source, mut workspace) = workspace();
        let nss = ResourceKey::new("start", 2009);
        let ncs = ResourceKey::new("start", 2010);
        workspace
            .stage_resource(nss.clone(), None, b"void main() {}")
            .expect("stage NSS v1");
        workspace
            .apply(EditCommand::ReplaceText {
                resource: nss.clone(),
                before: String::new(),
                after: "void main() {}".to_owned(),
            })
            .expect("commit NSS v1");
        let ncs_bytes = b"NCS V1.0\0";
        workspace
            .stage_resource(ncs.clone(), None, ncs_bytes)
            .expect("stage NCS");
        workspace
            .apply(EditCommand::CompileScript {
                resource: ncs,
                inputs: vec![ResourceContentDigest {
                    resource: nss.clone(),
                    content_sha256: sha256_bytes(b"void main() {}"),
                }],
                compiler_sha256: sha256_bytes(b"compiler"),
                before_sha256: None,
                after_sha256: sha256_bytes(ncs_bytes),
            })
            .expect("commit NCS");
        workspace
            .stage_resource(nss.clone(), None, b"void main() { int changed = 1; }")
            .expect("stage NSS v2");
        workspace
            .apply(EditCommand::ReplaceText {
                resource: nss,
                before: "void main() {}".to_owned(),
                after: "void main() { int changed = 1; }".to_owned(),
            })
            .expect("commit NSS v2");
        assert_eq!(
            workspace
                .validate_compiled_scripts()
                .expect_err("stale NCS")
                .code,
            "EDIT_NSS_COMPILATION_STALE"
        );
    }

    #[test]
    fn edits_map_environment_audio_and_addressed_tiles() {
        let resources =
            create_area_resources("mapped", "Carte MCP", "tno01", 3, 2, 0).expect("area");
        let are = resources
            .iter()
            .find(|resource| resource.key.resource_type == 2012)
            .expect("ARE");
        let git = resources
            .iter()
            .find(|resource| resource.key.resource_type == 2023)
            .expect("GIT");
        let are = edit_area_environment(
            &are.bytes,
            "mapped.are",
            &AreaEnvironmentPatch {
                comments: Some("Une carte pilotÃ©e par MCP".to_owned()),
                chance_rain: Some(35),
                fog_clip_distance: Some(72.0),
                on_enter: Some("map_enter".to_owned()),
                ..AreaEnvironmentPatch::default()
            },
        )
        .expect("environment");
        let are = edit_area_tile_at(
            &are,
            "mapped.are",
            2,
            1,
            TileState {
                tile_id: 0,
                orientation: 0,
                height: 0,
            },
            TileState {
                tile_id: 7,
                orientation: 2,
                height: 1,
            },
        )
        .expect("tile");
        let are = parse_gff(&are, "mapped.are").expect("reopen ARE");
        assert_eq!(
            integer(&find_field(&are.root, &["ChanceRain"]).expect("rain").value),
            Some(35)
        );
        let tiles = list_mut_any(&mut are.clone().root, &["Tile_List"])
            .expect("tiles")
            .clone();
        assert_eq!(
            integer(&find_field(&tiles[5], &["Tile_ID"]).expect("id").value),
            Some(7)
        );

        let git = edit_area_audio(
            &git.bytes,
            "mapped.git",
            &AreaAudioPatch {
                ambient_sound_day: Some(12),
                ambient_sound_day_volume: Some(64),
                music_day: Some(3),
                ..AreaAudioPatch::default()
            },
        )
        .expect("audio");
        parse_gff(&git, "mapped.git").expect("reopen GIT");
    }
}
