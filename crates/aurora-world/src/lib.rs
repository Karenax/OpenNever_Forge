use aurora_core::{ResourceKey, decode_nwn_text};
use aurora_gff::{GenericGff, GenericStruct, GenericValue, LocalizedString};
use aurora_mdl::{MdlFormat, parse_mdl};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Certain,
    Probable,
    Possible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub resource: String,
    pub field_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedText {
    pub string_ref: Option<u32>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub id: u32,
    pub text: LocalizedText,
    pub final_state: bool,
    pub delay: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JournalCategory {
    pub tag: String,
    pub name: LocalizedText,
    pub priority: u32,
    pub xp: u32,
    pub entries: Vec<JournalEntry>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Faction {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactionReputation {
    pub source_id: u32,
    pub target_id: u32,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeRelation {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub confidence: Confidence,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeModel {
    pub categories: Vec<JournalCategory>,
    pub factions: Vec<Faction>,
    pub reputations: Vec<FactionReputation>,
    pub relations: Vec<NarrativeRelation>,
    pub diagnostics: Vec<WorldDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaTile {
    pub x: u32,
    pub y: u32,
    pub tile_id: u32,
    pub orientation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaSpawnPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaInventoryItem {
    pub resref: String,
    pub tag: Option<String>,
    pub stack_size: u32,
    pub x: u32,
    pub y: u32,
    pub infinite: bool,
    pub category_index: Option<usize>,
    pub item_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaInstance {
    pub id: String,
    pub category: String,
    pub tag: Option<String>,
    pub template_resref: Option<String>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub bearing: Option<f32>,
    pub appearance: Option<u32>,
    pub transition_destination: Option<String>,
    pub transition_flags: Option<u32>,
    pub load_screen_id: Option<u32>,
    pub geometry: Vec<AreaPoint>,
    pub spawn_points: Vec<AreaSpawnPoint>,
    pub inventory: Vec<AreaInventoryItem>,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaMap {
    pub resref: String,
    pub name: LocalizedText,
    pub width: u32,
    pub height: u32,
    pub tileset: Option<String>,
    pub tiles: Vec<AreaTile>,
    pub instances: Vec<AreaInstance>,
    pub diagnostics: Vec<WorldDiagnostic>,
    pub are_source: String,
    pub git_source: Option<String>,
    pub gic_source: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSupport {
    Preview,
    Metadata,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecord {
    pub key: ResourceKey,
    pub source: String,
    pub format: String,
    pub support: AssetSupport,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub model_nodes: Vec<String>,
    pub animations: Vec<String>,
    pub textures: Vec<String>,
    pub referenced_models: Vec<String>,
    pub supermodel: Option<String>,
    pub mesh_count: usize,
    pub triangle_count: usize,
    pub skin_count: usize,
    pub walkmesh_count: usize,
    pub glb_preview: bool,
    pub sha256: String,
    pub diagnostics: Vec<WorldDiagnostic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub assets: Vec<AssetRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: f32,
    pub marker: bool,
    pub model_resref: Option<String>,
    pub model_resrefs: Vec<String>,
    pub walkmesh_available: bool,
    pub source_path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneAssetMap {
    pub tile_models: BTreeMap<u32, String>,
    pub instance_models: BTreeMap<String, Vec<String>>,
    pub known_models: BTreeSet<String>,
    pub walkmesh_models: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SceneManifest {
    pub area: String,
    pub width: u32,
    pub height: u32,
    pub tileset: Option<String>,
    pub objects: Vec<SceneObject>,
    pub overlays: Vec<SceneObject>,
    pub resolved_assets: usize,
    pub unique_models: usize,
    pub walkmesh_assets: usize,
    pub missing_assets: usize,
    pub memory_budget_bytes: u64,
    pub diagnostics: Vec<WorldDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub confidence: Confidence,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub resource: String,
    pub evidence: Option<Evidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldSummary {
    pub journal_categories: usize,
    pub journal_entries: usize,
    pub factions: usize,
    pub faction_relations: usize,
    pub areas: usize,
    pub tiles: usize,
    pub instances: usize,
    pub transitions: usize,
    pub assets: usize,
    pub previewable_assets: usize,
    pub scene_objects: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldIndex {
    pub narrative: NarrativeModel,
    pub areas: Vec<AreaMap>,
    pub assets: AssetIndex,
    pub scenes: Vec<SceneManifest>,
    pub graph_nodes: Vec<GraphNode>,
    pub graph_edges: Vec<GraphEdge>,
    pub diagnostics: Vec<WorldDiagnostic>,
    pub summary: WorldSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub schema_version: u32,
    pub module_sha256: String,
    pub summary: WorldSummary,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub diagnostics: Vec<WorldDiagnostic>,
}

pub fn adapt_narrative(
    journal: Option<&GenericGff>,
    factions: Option<&GenericGff>,
) -> NarrativeModel {
    let mut model = NarrativeModel::default();
    if let Some(jrl) = journal {
        for (index, category) in list(&jrl.root, &["Categories", "CategoryList"])
            .iter()
            .enumerate()
        {
            let tag = string(category, &["Tag"]).unwrap_or_else(|| format!("category_{index}"));
            let entries = list(category, &["EntryList", "Entries"])
                .iter()
                .enumerate()
                .map(|(entry_index, entry)| JournalEntry {
                    id: unsigned(entry, &["ID", "Id"]).unwrap_or(entry_index as u32),
                    text: localized(entry, &["Text"]),
                    final_state: boolean(entry, &["End", "IsEnd"]),
                    delay: unsigned(entry, &["Delay"]).unwrap_or(0),
                })
                .collect();
            model.categories.push(JournalCategory {
                tag,
                name: localized(category, &["Name"]),
                priority: unsigned(category, &["Priority"]).unwrap_or(0),
                xp: unsigned(category, &["XP", "Xp"]).unwrap_or(0),
                entries,
                source: jrl.source.clone(),
            });
        }
    } else {
        model.diagnostics.push(diagnostic(
            "JRL_NOT_FOUND",
            DiagnosticSeverity::Warning,
            "Aucun journal JRL sélectionné",
            "module.jrl",
        ));
    }
    if let Some(fac) = factions {
        for (index, faction) in list(&fac.root, &["FactionList", "Factions"])
            .iter()
            .enumerate()
        {
            model.factions.push(Faction {
                id: unsigned(faction, &["FactionID", "ID"]).unwrap_or(index as u32),
                name: string(faction, &["FactionName", "Name"])
                    .unwrap_or_else(|| format!("Faction {index}")),
                parent_id: unsigned(faction, &["FactionParentID", "ParentID"])
                    .filter(|value| *value != u32::MAX),
                global: boolean(faction, &["FactionGlobal", "Global"]),
            });
        }
        for reputation in list(&fac.root, &["RepList", "ReputationList"]) {
            model.reputations.push(FactionReputation {
                source_id: unsigned(reputation, &["FactionID1", "SourceID"]).unwrap_or(0),
                target_id: unsigned(reputation, &["FactionID2", "TargetID"]).unwrap_or(0),
                value: signed(reputation, &["FactionRep", "Reputation"]).unwrap_or(0),
            });
        }
    } else {
        model.diagnostics.push(diagnostic(
            "FAC_NOT_FOUND",
            DiagnosticSeverity::Warning,
            "Aucune matrice FAC sélectionnée",
            "repute.fac",
        ));
    }
    model.categories.sort_by(|a, b| a.tag.cmp(&b.tag));
    model.factions.sort_by_key(|value| value.id);
    model
        .reputations
        .sort_by_key(|value| (value.source_id, value.target_id));
    model
}

pub fn adapt_area(
    resref: &str,
    are: &GenericGff,
    git: Option<&GenericGff>,
    gic: Option<&GenericGff>,
) -> AreaMap {
    let width = unsigned(&are.root, &["Width"]).unwrap_or(0);
    let height = unsigned(&are.root, &["Height"]).unwrap_or(0);
    let tiles = list(&are.root, &["Tile_List", "TileList"])
        .iter()
        .enumerate()
        .map(|(index, tile)| AreaTile {
            x: (index as u32).checked_rem(width).unwrap_or(index as u32),
            y: (index as u32).checked_div(width).unwrap_or(0),
            tile_id: unsigned(tile, &["Tile_ID", "TileID"]).unwrap_or(0),
            orientation: unsigned(tile, &["Tile_Orientation", "Orientation"]).unwrap_or(0),
        })
        .collect::<Vec<_>>();
    let mut instances = Vec::new();
    if let Some(document) = git {
        for field in &document.root.fields {
            let GenericValue::List(values) = &field.value else {
                continue;
            };
            if !is_instance_list(&field.label) {
                continue;
            }
            for (index, value) in values.iter().enumerate() {
                let x = float(value, &["XPosition", "X"]).unwrap_or(0.0);
                let y = float(value, &["YPosition", "Y"]).unwrap_or(0.0);
                let z = float(value, &["ZPosition", "Z"]).unwrap_or(0.0);
                let geometry = list(value, &["Geometry"])
                    .iter()
                    .map(|point| AreaPoint {
                        x: float(point, &["PointX", "X"]).unwrap_or(0.0),
                        y: float(point, &["PointY", "Y"]).unwrap_or(0.0),
                        z: float(point, &["PointZ", "Z"]).unwrap_or(0.0),
                    })
                    .collect();
                let spawn_points = list(value, &["SpawnPointList"])
                    .iter()
                    .map(|point| AreaSpawnPoint {
                        x: float(point, &["X"]).unwrap_or(0.0),
                        y: float(point, &["Y"]).unwrap_or(0.0),
                        z: float(point, &["Z"]).unwrap_or(0.0),
                        orientation: float(point, &["Orientation"]).unwrap_or(0.0),
                    })
                    .collect();
                let inventory = area_inventory(value, &field.label);
                instances.push(AreaInstance {
                    id: format!("{}:{}:{index}", resref, field.label),
                    category: normalize_category(&field.label),
                    tag: string(value, &["Tag"]),
                    template_resref: string(value, &["TemplateResRef", "ResRef"]),
                    x,
                    y,
                    z,
                    bearing: float(value, &["Bearing", "XOrientation"]),
                    appearance: unsigned(
                        value,
                        &[
                            "Appearance_Type",
                            "AppearanceType",
                            "Appearance",
                            "GenericType",
                        ],
                    ),
                    transition_destination: string(
                        value,
                        &["LinkedTo", "TransitionDestin", "TransitionDestination"],
                    ),
                    transition_flags: unsigned(value, &["LinkedToFlags"]),
                    load_screen_id: unsigned(value, &["LoadScreenID"]),
                    geometry,
                    spawn_points,
                    inventory,
                    source_path: format!("{}::{}[{index}]", document.source, field.label),
                });
            }
        }
    }
    instances.sort_by(|a, b| a.id.cmp(&b.id));
    let mut diagnostics = Vec::new();
    if width == 0 || height == 0 {
        diagnostics.push(diagnostic(
            "AREA_DIMENSIONS_INVALID",
            DiagnosticSeverity::Error,
            "Dimensions ARE nulles ou absentes",
            &are.source,
        ));
    }
    if width.saturating_mul(height) as usize != tiles.len() {
        diagnostics.push(diagnostic(
            "AREA_TILE_COUNT_MISMATCH",
            DiagnosticSeverity::Warning,
            "Le nombre de tuiles ne correspond pas aux dimensions ARE",
            &are.source,
        ));
    }
    if git.is_none() {
        diagnostics.push(diagnostic(
            "AREA_GIT_NOT_FOUND",
            DiagnosticSeverity::Warning,
            "Aucune ressource GIT résolue pour la zone",
            resref,
        ));
    }
    for instance in &instances {
        if !instance.x.is_finite() || !instance.y.is_finite() || !instance.z.is_finite() {
            diagnostics.push(diagnostic(
                "INSTANCE_COORDINATE_INVALID",
                DiagnosticSeverity::Error,
                "Coordonnée non finie",
                &instance.source_path,
            ));
        }
    }
    AreaMap {
        resref: resref.to_owned(),
        name: localized(&are.root, &["Name", "LocalizedName"]),
        width,
        height,
        tileset: string(&are.root, &["Tileset"]),
        tiles,
        instances,
        diagnostics,
        are_source: are.source.clone(),
        git_source: git.map(|value| value.source.clone()),
        gic_source: gic.map(|value| value.source.clone()),
    }
}

fn area_inventory(instance: &GenericStruct, list_label: &str) -> Vec<AreaInventoryItem> {
    fn append_items(
        output: &mut Vec<AreaInventoryItem>,
        owner: &GenericStruct,
        category_index: Option<usize>,
    ) {
        for (item_index, item) in list(owner, &["ItemList"]).iter().enumerate() {
            let Some(resref) = string(item, &["TemplateResRef", "ResRef"]) else {
                continue;
            };
            output.push(AreaInventoryItem {
                resref,
                tag: string(item, &["Tag"]),
                stack_size: unsigned(item, &["StackSize"]).unwrap_or(1),
                x: unsigned(item, &["Repos_PosX"]).unwrap_or(0),
                y: unsigned(item, &["Repos_Posy", "Repos_PosY"]).unwrap_or(0),
                infinite: boolean(item, &["Infinite"]),
                category_index,
                item_index,
            });
        }
    }

    let mut output = Vec::new();
    match list_label {
        "Placeable List" => append_items(&mut output, instance, None),
        "StoreList" => {
            for (category_index, category) in list(instance, &["StoreList"]).iter().enumerate() {
                append_items(&mut output, category, Some(category_index));
            }
        }
        _ => {}
    }
    output
}

pub fn inspect_asset(key: ResourceKey, source: String, bytes: &[u8]) -> AssetRecord {
    let sha256 = hex_digest(bytes);
    let mut record = AssetRecord {
        key,
        source,
        format: "unknown".to_owned(),
        support: AssetSupport::Unsupported,
        width: None,
        height: None,
        model_nodes: Vec::new(),
        animations: Vec::new(),
        textures: Vec::new(),
        referenced_models: Vec::new(),
        supermodel: None,
        mesh_count: 0,
        triangle_count: 0,
        skin_count: 0,
        walkmesh_count: 0,
        glb_preview: false,
        sha256,
        diagnostics: Vec::new(),
    };
    match record.key.resource_type {
        2002 => inspect_mdl(&mut record, bytes),
        3 => inspect_tga(&mut record, bytes),
        2033 => inspect_dds(&mut record, bytes),
        6 => inspect_plt(&mut record, bytes),
        2022 => {
            record.format = "txi".to_owned();
            record.support = AssetSupport::Metadata;
        }
        2072 => inspect_mtr(&mut record, bytes),
        2073 => inspect_ktx(&mut record, bytes),
        2079 => {
            record.format = "gif".to_owned();
            record.support = AssetSupport::Preview;
        }
        2080 => inspect_png(&mut record, bytes),
        2081 => {
            record.format = "jpg".to_owned();
            record.support = AssetSupport::Preview;
        }
        _ => {}
    }
    record.model_nodes.sort();
    record.model_nodes.dedup();
    record.animations.sort();
    record.animations.dedup();
    record.textures.sort();
    record.textures.dedup();
    record.referenced_models.sort();
    record.referenced_models.dedup();
    record
}

pub fn parse_set_tile_models(bytes: &[u8]) -> BTreeMap<u32, String> {
    let text = decode_nwn_text(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut section = None;
    let mut models = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line
            .split_once(';')
            .map_or(raw_line, |(value, _)| value)
            .trim();
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            section = name
                .strip_prefix("TILE")
                .or_else(|| name.strip_prefix("Tile"))
                .or_else(|| name.strip_prefix("tile"))
                .and_then(|value| value.parse::<u32>().ok());
            continue;
        }
        let Some(tile_id) = section else { continue };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("model") {
            let model = value.trim().trim_matches('"').to_ascii_lowercase();
            if !model.is_empty() {
                models.insert(tile_id, model);
            }
        }
    }
    models
}

pub fn scene_manifest(area: &AreaMap, assets: &SceneAssetMap) -> SceneManifest {
    let mut objects = Vec::new();
    let mut overlays = Vec::new();
    let mut missing_assets = 0;
    for tile in &area.tiles {
        let model_resref = assets.tile_models.get(&tile.tile_id).cloned();
        let available = model_resref
            .as_ref()
            .is_some_and(|value| assets.known_models.contains(value));
        if !available {
            missing_assets += 1;
        }
        objects.push(SceneObject {
            id: format!("tile:{}:{}", tile.x, tile.y),
            kind: "tile".to_owned(),
            label: format!("Tuile {}", tile.tile_id),
            x: tile.x as f32 * 10.0 + 5.0,
            y: 0.0,
            z: tile.y as f32 * 10.0 + 5.0,
            rotation: tile.orientation as f32 * std::f32::consts::FRAC_PI_2,
            marker: !available,
            walkmesh_available: model_resref
                .as_ref()
                .is_some_and(|value| assets.walkmesh_models.contains(value)),
            model_resrefs: model_resref.clone().into_iter().collect(),
            model_resref,
            source_path: area.are_source.clone(),
        });
    }
    for instance in &area.instances {
        let requires_model = matches!(
            instance.category.as_str(),
            "creature" | "door" | "placeable"
        );
        let model_resrefs = assets
            .instance_models
            .get(&instance.id)
            .cloned()
            .unwrap_or_default();
        let available = !model_resrefs.is_empty()
            && model_resrefs
                .iter()
                .all(|value| assets.known_models.contains(value));
        if requires_model && !available {
            missing_assets += 1;
        }
        let object = SceneObject {
            id: instance.id.clone(),
            kind: instance.category.clone(),
            label: instance
                .tag
                .clone()
                .or_else(|| instance.template_resref.clone())
                .unwrap_or_else(|| instance.category.clone()),
            x: instance.x,
            y: instance.z,
            z: instance.y,
            rotation: instance.bearing.unwrap_or(0.0),
            marker: !available,
            walkmesh_available: model_resrefs
                .iter()
                .any(|value| assets.walkmesh_models.contains(value)),
            model_resref: model_resrefs.first().cloned(),
            model_resrefs,
            source_path: instance.source_path.clone(),
        };
        if matches!(
            instance.category.as_str(),
            "trigger" | "encounter" | "waypoint" | "sound"
        ) {
            overlays.push(object);
        } else {
            objects.push(object);
        }
    }
    let resolved_assets = objects
        .iter()
        .chain(&overlays)
        .filter(|value| !value.marker)
        .count();
    let unique_models = objects
        .iter()
        .chain(&overlays)
        .flat_map(|value| value.model_resrefs.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();
    let walkmesh_assets = objects
        .iter()
        .chain(&overlays)
        .filter(|value| value.walkmesh_available)
        .count();
    SceneManifest {
        area: area.resref.clone(),
        width: area.width,
        height: area.height,
        tileset: area.tileset.clone(),
        objects,
        overlays,
        resolved_assets,
        unique_models,
        walkmesh_assets,
        missing_assets,
        memory_budget_bytes: 256 * 1024 * 1024,
        diagnostics: area.diagnostics.clone(),
    }
}

impl WorldIndex {
    pub fn finalize(&mut self) {
        self.areas.sort_by(|a, b| a.resref.cmp(&b.resref));
        self.assets.assets.sort_by(|a, b| a.key.cmp(&b.key));
        self.scenes.sort_by(|a, b| a.area.cmp(&b.area));
        self.graph_nodes.sort_by(|a, b| a.id.cmp(&b.id));
        self.graph_nodes.dedup_by(|a, b| a.id == b.id);
        self.graph_edges.sort_by(|a, b| a.id.cmp(&b.id));
        self.graph_edges.dedup_by(|a, b| a.id == b.id);
        self.diagnostics.extend(self.narrative.diagnostics.clone());
        self.diagnostics.extend(
            self.areas
                .iter()
                .flat_map(|value| value.diagnostics.clone()),
        );
        self.diagnostics.extend(
            self.assets
                .assets
                .iter()
                .flat_map(|value| value.diagnostics.clone()),
        );
        self.diagnostics.sort_by(|a, b| {
            (&a.code, &a.resource, &a.message).cmp(&(&b.code, &b.resource, &b.message))
        });
        self.diagnostics.dedup();
        self.summary = WorldSummary {
            journal_categories: self.narrative.categories.len(),
            journal_entries: self
                .narrative
                .categories
                .iter()
                .map(|value| value.entries.len())
                .sum(),
            factions: self.narrative.factions.len(),
            faction_relations: self.narrative.reputations.len(),
            areas: self.areas.len(),
            tiles: self.areas.iter().map(|value| value.tiles.len()).sum(),
            instances: self.areas.iter().map(|value| value.instances.len()).sum(),
            transitions: self
                .areas
                .iter()
                .flat_map(|value| &value.instances)
                .filter(|value| value.transition_destination.is_some())
                .count(),
            assets: self.assets.assets.len(),
            previewable_assets: self
                .assets
                .assets
                .iter()
                .filter(|value| value.support == AssetSupport::Preview)
                .count(),
            scene_objects: self
                .scenes
                .iter()
                .map(|value| value.objects.len() + value.overlays.len())
                .sum(),
            graph_nodes: self.graph_nodes.len(),
            graph_edges: self.graph_edges.len(),
            diagnostics: self.diagnostics.len(),
        };
    }

    pub fn report(&self, module_sha256: impl Into<String>) -> DiagnosticReport {
        DiagnosticReport {
            schema_version: 1,
            module_sha256: module_sha256.into(),
            summary: self.summary.clone(),
            nodes: self.graph_nodes.clone(),
            edges: self.graph_edges.clone(),
            diagnostics: self
                .diagnostics
                .iter()
                .cloned()
                .map(|mut value| {
                    value.resource = basename(&value.resource);
                    if let Some(evidence) = &mut value.evidence {
                        evidence.resource = basename(&evidence.resource);
                    }
                    value
                })
                .collect(),
        }
    }
}

impl DiagnosticReport {
    pub fn stable_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializable report")
    }
    pub fn standalone_html(&self) -> String {
        let rows = self
            .diagnostics
            .iter()
            .map(|value| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                    html(&value.code),
                    html(&value.resource),
                    html(&value.message)
                )
            })
            .collect::<String>();
        format!(
            "<!doctype html><html lang=\"fr\"><meta charset=\"utf-8\"><title>OpenNever Forge — diagnostic</title><style>body{{font:14px system-ui;background:#10161d;color:#dce7f2;padding:2rem}}table{{border-collapse:collapse;width:100%}}td,th{{border:1px solid #33404d;padding:.5rem;text-align:left}}</style><h1>Rapport de diagnostic</h1><p>Module <code>{}</code> · {} nœuds · {} relations</p><table><thead><tr><th>Code</th><th>Ressource</th><th>Message</th></tr></thead><tbody>{rows}</tbody></table></html>",
            html(&self.module_sha256),
            self.summary.graph_nodes,
            self.summary.graph_edges
        )
    }
}

fn inspect_mdl(record: &mut AssetRecord, bytes: &[u8]) {
    match parse_mdl(bytes) {
        Ok(model) => {
            record.format = match model.format {
                MdlFormat::Ascii => "mdl_ascii",
                MdlFormat::Binary => "mdl_binary",
            }
            .to_owned();
            record.supermodel = model.supermodel.clone();
            for node in &model.nodes {
                if let Some(reference) = &node.reference_model {
                    record.referenced_models.push(reference.clone());
                }
                record.model_nodes.extend(
                    node.kinds
                        .iter()
                        .map(|kind| format!("{kind:?}").to_ascii_lowercase()),
                );
                if let Some(mesh) = &node.mesh {
                    record.mesh_count += 1;
                    record.triangle_count += mesh.indices.len() / 3;
                    record.skin_count += usize::from(mesh.skin.is_some());
                    record.walkmesh_count += usize::from(mesh.walkmesh);
                    record.textures.extend(mesh.material.textures.clone());
                }
            }
            record.animations.extend(
                model
                    .animations
                    .iter()
                    .map(|animation| animation.name.clone()),
            );
            record.glb_preview = record.mesh_count > 0;
            record.support = if record.glb_preview {
                AssetSupport::Preview
            } else {
                AssetSupport::Metadata
            };
            record
                .diagnostics
                .extend(model.diagnostics.into_iter().map(|value| {
                    diagnostic(
                        &value.code,
                        DiagnosticSeverity::Warning,
                        &value.message,
                        &record.key.to_string(),
                    )
                }));
            if !record.glb_preview {
                record.diagnostics.push(diagnostic(
                    "MDL_GEOMETRY_EMPTY",
                    DiagnosticSeverity::Info,
                    "Le modèle est valide mais ne contient aucun mesh prévisualisable",
                    &record.key.to_string(),
                ));
            }
        }
        Err(error) => {
            record.format = "mdl_unknown".to_owned();
            record.support = AssetSupport::Unsupported;
            record.diagnostics.push(diagnostic(
                &error.code,
                DiagnosticSeverity::Error,
                &error.message,
                &record.key.to_string(),
            ));
        }
    }
}

fn inspect_tga(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "tga".to_owned();
    if bytes.len() >= 18 {
        record.width = Some(u16::from_le_bytes([bytes[12], bytes[13]]) as u32);
        record.height = Some(u16::from_le_bytes([bytes[14], bytes[15]]) as u32);
        record.support = AssetSupport::Preview;
    } else {
        record.diagnostics.push(diagnostic(
            "TGA_HEADER_TRUNCATED",
            DiagnosticSeverity::Error,
            "En-tête TGA tronqué",
            &record.key.to_string(),
        ));
    }
}

fn inspect_dds(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "dds".to_owned();
    if bytes.len() >= 20 && &bytes[0..4] == b"DDS " {
        record.height = Some(u32::from_le_bytes(bytes[12..16].try_into().expect("slice")));
        record.width = Some(u32::from_le_bytes(bytes[16..20].try_into().expect("slice")));
        record.support = AssetSupport::Preview;
    } else if bytes.len() >= 20 {
        let width = u32::from_le_bytes(bytes[0..4].try_into().expect("slice"));
        let height = u32::from_le_bytes(bytes[4..8].try_into().expect("slice"));
        let encoding = u32::from_le_bytes(bytes[8..12].try_into().expect("slice"));
        if width > 0
            && height > 0
            && width.is_power_of_two()
            && height.is_power_of_two()
            && matches!(encoding, 3 | 4)
        {
            record.width = Some(width);
            record.height = Some(height);
            record.support = AssetSupport::Preview;
            return;
        }
        record.diagnostics.push(diagnostic(
            "DDS_HEADER_INVALID",
            DiagnosticSeverity::Error,
            "En-tête DDS standard ou BioWare invalide",
            &record.key.to_string(),
        ));
    } else {
        record.diagnostics.push(diagnostic(
            "DDS_HEADER_INVALID",
            DiagnosticSeverity::Error,
            "En-tête DDS standard ou BioWare absent ou tronqué",
            &record.key.to_string(),
        ));
    }
}

fn inspect_plt(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "plt".to_owned();
    if bytes.len() >= 24 && &bytes[..8] == b"PLT V1  " {
        let width = u32::from_le_bytes(bytes[16..20].try_into().expect("slice"));
        let height = u32::from_le_bytes(bytes[20..24].try_into().expect("slice"));
        let payload = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(2));
        record.width = Some(width);
        record.height = Some(height);
        if width > 0
            && height > 0
            && payload.is_some_and(|payload| bytes.len().saturating_sub(24) >= payload)
        {
            record.support = AssetSupport::Preview;
            record.diagnostics.push(diagnostic(
                "PLT_LAYER_PREVIEW",
                DiagnosticSeverity::Info,
                "Aperçu local par couches ; les couleurs d'apparence finales dépendent du blueprint",
                &record.key.to_string(),
            ));
        } else {
            record.diagnostics.push(diagnostic(
                "PLT_PAYLOAD_INVALID",
                DiagnosticSeverity::Error,
                "Dimensions ou données PLT invalides",
                &record.key.to_string(),
            ));
        }
    } else {
        record.diagnostics.push(diagnostic(
            "PLT_HEADER_INVALID",
            DiagnosticSeverity::Error,
            "Signature PLT absente ou tronquée",
            &record.key.to_string(),
        ));
    }
}

fn inspect_mtr(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "mtr".to_owned();
    record.support = AssetSupport::Metadata;
    for raw in decode_nwn_text(bytes).lines() {
        let line = raw.split("//").next().unwrap_or_default().trim();
        let values = line.split_whitespace().collect::<Vec<_>>();
        if values.len() >= 2 && values[0].to_ascii_lowercase().starts_with("texture") {
            record
                .textures
                .push(values[1].trim_matches('"').to_ascii_lowercase());
        }
    }
}

fn inspect_ktx(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "ktx".to_owned();
    const KTX1: &[u8; 12] = b"\xABKTX 11\xBB\r\n\x1A\n";
    const KTX2: &[u8; 12] = b"\xABKTX 20\xBB\r\n\x1A\n";
    if bytes.len() >= 44 && &bytes[..12] == KTX1 {
        record.width = Some(u32::from_le_bytes(bytes[36..40].try_into().expect("slice")));
        record.height = Some(u32::from_le_bytes(bytes[40..44].try_into().expect("slice")));
        record.support = AssetSupport::Preview;
    } else if bytes.len() >= 32 && &bytes[..12] == KTX2 {
        record.width = Some(u32::from_le_bytes(bytes[24..28].try_into().expect("slice")));
        record.height = Some(u32::from_le_bytes(bytes[28..32].try_into().expect("slice")));
        record.support = AssetSupport::Preview;
    } else {
        record.diagnostics.push(diagnostic(
            "KTX_HEADER_INVALID",
            DiagnosticSeverity::Error,
            "Signature KTX absente ou tronquée",
            &record.key.to_string(),
        ));
    }
}

fn inspect_png(record: &mut AssetRecord, bytes: &[u8]) {
    record.format = "png".to_owned();
    if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1A\n" {
        record.width = Some(u32::from_be_bytes(bytes[16..20].try_into().expect("slice")));
        record.height = Some(u32::from_be_bytes(bytes[20..24].try_into().expect("slice")));
        record.support = AssetSupport::Preview;
    } else {
        record.diagnostics.push(diagnostic(
            "PNG_HEADER_INVALID",
            DiagnosticSeverity::Error,
            "Signature PNG absente ou tronquée",
            &record.key.to_string(),
        ));
    }
}

fn list<'a>(root: &'a GenericStruct, names: &[&str]) -> &'a [GenericStruct] {
    for field in &root.fields {
        if names
            .iter()
            .any(|name| field.label.eq_ignore_ascii_case(name))
            && let GenericValue::List(values) = &field.value
        {
            return values;
        }
    }
    &[]
}
fn value<'a>(root: &'a GenericStruct, names: &[&str]) -> Option<&'a GenericValue> {
    root.fields
        .iter()
        .find(|field| {
            names
                .iter()
                .any(|name| field.label.eq_ignore_ascii_case(name))
        })
        .map(|field| &field.value)
}
fn string(root: &GenericStruct, names: &[&str]) -> Option<String> {
    match value(root, names)? {
        GenericValue::String(v) | GenericValue::ResRef(v) => Some(v.clone()),
        _ => None,
    }
}
fn unsigned(root: &GenericStruct, names: &[&str]) -> Option<u32> {
    match value(root, names)? {
        GenericValue::Byte(v) => Some((*v).into()),
        GenericValue::Word(v) => Some((*v).into()),
        GenericValue::Dword(v) => Some(*v),
        GenericValue::Int(v) => u32::try_from(*v).ok(),
        _ => None,
    }
}
fn signed(root: &GenericStruct, names: &[&str]) -> Option<i32> {
    match value(root, names)? {
        GenericValue::Byte(v) => Some((*v).into()),
        GenericValue::Char(v) => Some((*v).into()),
        GenericValue::Word(v) => Some((*v).into()),
        GenericValue::Short(v) => Some((*v).into()),
        GenericValue::Dword(v) => i32::try_from(*v).ok(),
        GenericValue::Int(v) => Some(*v),
        _ => None,
    }
}
fn float(root: &GenericStruct, names: &[&str]) -> Option<f32> {
    match value(root, names)? {
        GenericValue::Float(v) => Some(*v),
        GenericValue::Double(v) => Some(*v as f32),
        GenericValue::Int(v) => Some(*v as f32),
        GenericValue::Dword(v) => Some(*v as f32),
        _ => None,
    }
}
fn boolean(root: &GenericStruct, names: &[&str]) -> bool {
    unsigned(root, names).is_some_and(|value| value != 0)
}
fn localized(root: &GenericStruct, names: &[&str]) -> LocalizedText {
    match value(root, names) {
        Some(GenericValue::LocalizedString(value)) => localized_value(value),
        Some(GenericValue::String(value)) => LocalizedText {
            string_ref: None,
            text: Some(value.clone()),
        },
        _ => LocalizedText::default(),
    }
}
fn localized_value(value: &LocalizedString) -> LocalizedText {
    LocalizedText {
        string_ref: value.string_ref,
        text: value.primary_text().map(str::to_owned),
    }
}
fn is_instance_list(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "creature",
        "door",
        "placeable",
        "trigger",
        "encounter",
        "waypoint",
        "sound",
        "store",
        "item",
        "list",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
fn normalize_category(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    [
        "creature",
        "door",
        "placeable",
        "trigger",
        "encounter",
        "waypoint",
        "sound",
        "store",
        "item",
    ]
    .into_iter()
    .find(|needle| lower.contains(needle))
    .unwrap_or("instance")
    .to_owned()
}
fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn diagnostic(
    code: &str,
    severity: DiagnosticSeverity,
    message: &str,
    resource: &str,
) -> WorldDiagnostic {
    WorldDiagnostic {
        code: code.to_owned(),
        severity,
        message: message.to_owned(),
        resource: resource.to_owned(),
        evidence: None,
    }
}
fn html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn basename(value: &str) -> String {
    value.rsplit(['/', '\\']).next().unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_gff::{GenericField, GenericStruct, GenericValue};

    fn structure(fields: Vec<(&str, GenericValue)>) -> GenericStruct {
        GenericStruct {
            index: 0,
            struct_type: 0,
            fields: fields
                .into_iter()
                .map(|(label, value)| GenericField {
                    label: label.to_owned(),
                    field_type: 0,
                    value,
                })
                .collect(),
        }
    }
    fn document(kind: &str, root: GenericStruct) -> GenericGff {
        GenericGff {
            file_type: kind.to_owned(),
            file_version: "V3.2".to_owned(),
            source: format!("fixture.{kind}"),
            struct_count: 1,
            field_count: root.fields.len() as u32,
            root,
        }
    }

    #[test]
    fn journal_and_factions_keep_explicit_structure() {
        let entry = structure(vec![
            ("ID", GenericValue::Dword(7)),
            ("End", GenericValue::Byte(1)),
            ("Text", GenericValue::String("Terminé".to_owned())),
        ]);
        let category = structure(vec![
            ("Tag", GenericValue::String("quest".to_owned())),
            ("EntryList", GenericValue::List(vec![entry])),
        ]);
        let jrl = document(
            "JRL ",
            structure(vec![("Categories", GenericValue::List(vec![category]))]),
        );
        let fac = document(
            "FAC ",
            structure(vec![(
                "FactionList",
                GenericValue::List(vec![structure(vec![(
                    "FactionName",
                    GenericValue::String("Commoner".to_owned()),
                )])]),
            )]),
        );
        let model = adapt_narrative(Some(&jrl), Some(&fac));
        assert_eq!(model.categories[0].entries[0].id, 7);
        assert!(model.categories[0].entries[0].final_state);
        assert_eq!(model.factions[0].name, "Commoner");
    }

    #[test]
    fn area_projection_preserves_tiles_instances_and_coordinates() {
        let tile = structure(vec![
            ("Tile_ID", GenericValue::Dword(12)),
            ("Tile_Orientation", GenericValue::Byte(2)),
        ]);
        let are = document(
            "ARE ",
            structure(vec![
                ("Width", GenericValue::Dword(1)),
                ("Height", GenericValue::Dword(1)),
                ("Tile_List", GenericValue::List(vec![tile])),
            ]),
        );
        let creature = structure(vec![
            ("Tag", GenericValue::String("guard".to_owned())),
            ("XPosition", GenericValue::Float(4.0)),
            ("YPosition", GenericValue::Float(8.0)),
        ]);
        let git = document(
            "GIT ",
            structure(vec![("Creature List", GenericValue::List(vec![creature]))]),
        );
        let area = adapt_area("town", &are, Some(&git), None);
        assert_eq!(area.tiles[0].tile_id, 12);
        assert_eq!(area.instances[0].x, 4.0);
        assert_eq!(area.instances[0].category, "creature");
    }

    #[test]
    fn area_projection_exposes_geometry_spawns_transitions_and_inventory() {
        let are = document(
            "ARE ",
            structure(vec![
                ("Width", GenericValue::Dword(1)),
                ("Height", GenericValue::Dword(1)),
                ("Tile_List", GenericValue::List(vec![structure(Vec::new())])),
            ]),
        );
        let trigger = structure(vec![
            (
                "Geometry",
                GenericValue::List(vec![structure(vec![
                    ("PointX", GenericValue::Float(1.0)),
                    ("PointY", GenericValue::Float(2.0)),
                    ("PointZ", GenericValue::Float(3.0)),
                ])]),
            ),
            ("LinkedTo", GenericValue::String("wp_exit".to_owned())),
            ("LinkedToFlags", GenericValue::Byte(2)),
            ("LoadScreenID", GenericValue::Word(7)),
        ]);
        let placeable = structure(vec![(
            "ItemList",
            GenericValue::List(vec![structure(vec![
                ("TemplateResRef", GenericValue::ResRef("potion".to_owned())),
                ("StackSize", GenericValue::Word(3)),
                ("Repos_PosX", GenericValue::Word(2)),
                ("Repos_Posy", GenericValue::Word(1)),
            ])]),
        )]);
        let git = document(
            "GIT ",
            structure(vec![
                ("TriggerList", GenericValue::List(vec![trigger])),
                ("Placeable List", GenericValue::List(vec![placeable])),
            ]),
        );
        let area = adapt_area("town", &are, Some(&git), None);
        let trigger = area
            .instances
            .iter()
            .find(|instance| instance.category == "trigger")
            .expect("trigger");
        assert_eq!(trigger.geometry[0].y, 2.0);
        assert_eq!(trigger.transition_flags, Some(2));
        assert_eq!(trigger.load_screen_id, Some(7));
        let placeable = area
            .instances
            .iter()
            .find(|instance| instance.category == "placeable")
            .expect("placeable");
        assert_eq!(placeable.inventory[0].resref, "potion");
        assert_eq!(placeable.inventory[0].stack_size, 3);
    }

    #[test]
    fn set_models_drive_real_scene_assets_and_keep_missing_markers_explicit() {
        let models = parse_set_tile_models(
            b"[GENERAL]\nName=Fixture\n[TILE0]\nModel=tno01_a01_01\n[TILE1]\nModel=missing_tile ; comment\n",
        );
        assert_eq!(models.get(&0).map(String::as_str), Some("tno01_a01_01"));
        let are = document(
            "ARE ",
            structure(vec![
                ("Width", GenericValue::Dword(2)),
                ("Height", GenericValue::Dword(1)),
                (
                    "Tile_List",
                    GenericValue::List(vec![
                        structure(vec![("Tile_ID", GenericValue::Dword(0))]),
                        structure(vec![("Tile_ID", GenericValue::Dword(1))]),
                    ]),
                ),
            ]),
        );
        let area = adapt_area("town", &are, None, None);
        let mut assets = SceneAssetMap {
            tile_models: models,
            ..SceneAssetMap::default()
        };
        assets.known_models.insert("tno01_a01_01".to_owned());
        assets.walkmesh_models.insert("tno01_a01_01".to_owned());
        let scene = scene_manifest(&area, &assets);
        assert_eq!(scene.resolved_assets, 1);
        assert_eq!(scene.missing_assets, 1);
        assert_eq!(scene.unique_models, 2);
        assert!(scene.objects[0].walkmesh_available);
        assert!(!scene.objects[0].marker);
        assert!(scene.objects[1].marker);
    }

    #[test]
    fn asset_probe_and_reports_are_deterministic() {
        let asset = inspect_asset(
            ResourceKey::new("hero", 2002),
            "hero.mdl".to_owned(),
            b"newmodel hero\nnode trimesh body\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nbitmap hero_diff\nendnode\nnewanim walk\ndoneanim walk\n",
        );
        assert_eq!(asset.support, AssetSupport::Preview);
        assert_eq!(asset.model_nodes, vec!["trimesh"]);
        assert_eq!(asset.animations, vec!["walk"]);
        let mut index = WorldIndex::default();
        index.assets.assets.push(asset);
        index.diagnostics.push(WorldDiagnostic {
            code: "PATH".to_owned(),
            severity: DiagnosticSeverity::Info,
            message: "preuve".to_owned(),
            resource: "C:\\private\\module.mod".to_owned(),
            evidence: None,
        });
        index.finalize();
        let report = index.report("abc");
        assert_eq!(report.stable_json(), report.stable_json());
        assert!(report.standalone_html().contains("abc"));
        assert!(!report.stable_json().contains("C:\\\\private"));
    }

    #[test]
    fn plt_dds_and_mtr_keep_dimensions_layers_and_texture_references_explicit() {
        let mut plt = b"PLT V1  \0\0\0\0\0\0\0\0".to_vec();
        plt.extend(2_u32.to_le_bytes());
        plt.extend(1_u32.to_le_bytes());
        plt.extend([255, 0, 128, 4]);
        let plt = inspect_asset(ResourceKey::new("body", 6), "body.plt".to_owned(), &plt);
        assert_eq!(plt.support, AssetSupport::Preview);
        assert_eq!((plt.width, plt.height), (Some(2), Some(1)));
        assert!(
            plt.diagnostics
                .iter()
                .any(|value| value.code == "PLT_LAYER_PREVIEW")
        );

        let mut dds = Vec::new();
        dds.extend(512_u32.to_le_bytes());
        dds.extend(256_u32.to_le_bytes());
        dds.extend(3_u32.to_le_bytes());
        dds.extend(65_536_u32.to_le_bytes());
        dds.extend(1_f32.to_le_bytes());
        let dds = inspect_asset(
            ResourceKey::new("floor", 2033),
            "floor.dds".to_owned(),
            &dds,
        );
        assert_eq!(dds.support, AssetSupport::Preview);
        assert_eq!((dds.width, dds.height), (Some(512), Some(256)));
        assert!(dds.diagnostics.is_empty());

        let mtr = inspect_asset(
            ResourceKey::new("metal", 2072),
            "metal.mtr".to_owned(),
            b"// material\ntexture0 metal_d\ntexture1 \"metal_n\"\n",
        );
        assert_eq!(mtr.support, AssetSupport::Metadata);
        assert_eq!(mtr.textures, vec!["metal_d", "metal_n"]);
    }
}
