use super::{InstancePlacement, edit_error, validate_resref};
use aurora_core::AppResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};

pub const MAP_GENERATION_SCHEMA_VERSION: u32 = 1;
pub const MAP_MAX_WIDTH: u32 = 32;
pub const MAP_MAX_HEIGHT: u32 = 32;
pub const MAP_MAX_TILES: usize = (MAP_MAX_WIDTH * MAP_MAX_HEIGHT) as usize;
pub const MAP_MAX_DENSITY_RULES: usize = 16;
pub const MAP_MAX_BLUEPRINTS_PER_RULE: usize = 128;
pub const MAP_MAX_PLACEMENTS: usize = 2_048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapDensityRule {
    pub category: String,
    pub per_hundred_tiles: u16,
    pub min_spacing_tiles: u16,
    #[serde(default)]
    pub template_resrefs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapGenerationSpec {
    pub schema_version: u32,
    pub brief: String,
    pub resref: String,
    pub name: String,
    pub tileset: String,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub base_tile_id: u32,
    #[serde(default)]
    pub variant_tile_ids: Vec<u32>,
    pub border_margin: u16,
    pub reserved_percent: u8,
    #[serde(default)]
    pub densities: Vec<MapDensityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapTilePlan {
    pub x: u32,
    pub y: u32,
    pub tile_id: u32,
    pub orientation: u8,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapGenerationMetrics {
    pub total_tiles: usize,
    pub buildable_tiles: usize,
    pub reserved_tiles: usize,
    pub placement_count: usize,
    pub occupied_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapCompatibilityReport {
    pub tileset_resolved: bool,
    pub tileset_sha256: Option<String>,
    pub resolved_tile_count: usize,
    #[serde(default)]
    pub selected_tile_ids: Vec<u32>,
    pub tile_ids_verified: bool,
    pub edge_compatibility_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapGenerationPlan {
    pub plan_sha256: String,
    pub spec: MapGenerationSpec,
    pub tiles: Vec<MapTilePlan>,
    pub placements: Vec<InstancePlacement>,
    pub metrics: MapGenerationMetrics,
    #[serde(default)]
    pub compatibility: MapCompatibilityReport,
    pub warnings: Vec<String>,
}

pub fn generate_map_plan(spec: &MapGenerationSpec) -> AppResult<MapGenerationPlan> {
    generate_map_plan_with_compatibility(spec, MapCompatibilityReport::default())
}

pub fn generate_map_plan_with_compatibility(
    spec: &MapGenerationSpec,
    compatibility: MapCompatibilityReport,
) -> AppResult<MapGenerationPlan> {
    validate_spec(spec)?;
    let margin = u32::from(spec.border_margin);
    let mut tiles = Vec::with_capacity((spec.width * spec.height) as usize);
    let mut buildable = Vec::new();
    let variants = normalized_tile_ids(spec);

    for y in 0..spec.height {
        for x in 0..spec.width {
            let border = x < margin
                || y < margin
                || x >= spec.width.saturating_sub(margin)
                || y >= spec.height.saturating_sub(margin);
            let tile_hash = coordinate_hash(spec.seed, x, y, 0x5449_4c45);
            let tile_id = if border {
                spec.base_tile_id
            } else {
                variants[(tile_hash as usize) % variants.len()]
            };
            tiles.push(MapTilePlan {
                x,
                y,
                tile_id,
                // Rotating a tile without proving its SET connector compatibility can create
                // impassable seams. Generated plans therefore start at the conservative
                // orientation; explicit MCP/UI tile edits may rotate a verified tile later.
                orientation: 0,
                height: 0,
            });
            if !border {
                buildable.push((x, y));
            }
        }
    }

    let mut reserved = HashSet::new();
    for &(x, y) in &buildable {
        if coordinate_hash(spec.seed, x, y, 0x5245_5356) % 100 < u64::from(spec.reserved_percent) {
            reserved.insert((x, y));
        }
    }

    let mut placements = Vec::new();
    let mut occupied = HashSet::new();
    let mut warnings = Vec::new();
    for (rule_index, rule) in spec.densities.iter().enumerate() {
        if rule.per_hundred_tiles == 0 {
            continue;
        }
        if rule.template_resrefs.is_empty() {
            warnings.push(format!(
                "{} : aucun blueprint sélectionné, densité ignorée.",
                category_label(&rule.category)
            ));
            continue;
        }
        let category_salt = text_hash(&rule.category) ^ rule_index as u64;
        let mut candidates = buildable
            .iter()
            .copied()
            .filter(|cell| !reserved.contains(cell))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|&(x, y)| coordinate_hash(spec.seed, x, y, category_salt));
        let desired =
            ((buildable.len() as u64 * u64::from(rule.per_hundred_tiles) + 50) / 100) as usize;
        let spacing_squared = u32::from(rule.min_spacing_tiles).pow(2);
        let mut category_cells = Vec::new();
        for (x, y) in candidates {
            if placements.len() >= MAP_MAX_PLACEMENTS || category_cells.len() >= desired {
                break;
            }
            if occupied.contains(&(x, y)) {
                continue;
            }
            if spacing_squared > 0
                && category_cells.iter().any(|&(other_x, other_y)| {
                    let dx = x.abs_diff(other_x);
                    let dy = y.abs_diff(other_y);
                    dx * dx + dy * dy < spacing_squared
                })
            {
                continue;
            }
            let placement_hash = coordinate_hash(spec.seed, x, y, category_salt ^ 0x504c_4143);
            let template = rule.template_resrefs
                [(placement_hash as usize) % rule.template_resrefs.len()]
            .clone();
            let number = category_cells.len() + 1;
            placements.push(InstancePlacement {
                category: rule.category.clone(),
                template_resref: template,
                tag: format!("{}_{}", short_category(&rule.category), number),
                x: f64::from(x) * 10.0 + 5.0,
                y: f64::from(y) * 10.0 + 5.0,
                z: 0.0,
                bearing: ((placement_hash >> 24) % 6_284) as f64 / 1_000.0,
                linked_to: None,
            });
            occupied.insert((x, y));
            category_cells.push((x, y));
        }
        if category_cells.len() < desired {
            warnings.push(format!(
                "{} : {} placement(s) sur {} demandés après contraintes d’espace.",
                category_label(&rule.category),
                category_cells.len(),
                desired
            ));
        }
    }

    let occupied_percent = if buildable.is_empty() {
        0
    } else {
        ((placements.len() * 100) / buildable.len()).min(100) as u8
    };
    if !spec.variant_tile_ids.is_empty() {
        warnings.push(
            "Les variantes de tuiles sont déterministes, mais leur compatibilité visuelle dépend du SET du tileset chargé."
                .to_owned(),
        );
    }
    warnings.push(
        "Les orientations automatiques restent à 0 tant que les raccords du SET ne sont pas validés ; utilisez l’édition de tuile après inspection pour une rotation explicite."
            .to_owned(),
    );
    let mut plan = MapGenerationPlan {
        plan_sha256: String::new(),
        spec: spec.clone(),
        tiles,
        placements,
        metrics: MapGenerationMetrics {
            total_tiles: (spec.width * spec.height) as usize,
            buildable_tiles: buildable.len(),
            reserved_tiles: reserved.len(),
            placement_count: occupied.len(),
            occupied_percent,
        },
        compatibility,
        warnings,
    };
    plan.plan_sha256 = hash_plan(&plan);
    Ok(plan)
}

fn validate_spec(spec: &MapGenerationSpec) -> AppResult<()> {
    if spec.schema_version != MAP_GENERATION_SCHEMA_VERSION {
        return Err(edit_error(
            "EDIT_MAP_SCHEMA_UNSUPPORTED",
            format!("unsupported map generation schema {}", spec.schema_version),
        ));
    }
    validate_resref(&spec.resref)?;
    validate_resref(&spec.tileset)?;
    if spec.name.trim().is_empty() || spec.name.len() > 1_024 || spec.brief.len() > 64 * 1_024 {
        return Err(edit_error(
            "EDIT_MAP_IDENTITY_INVALID",
            "map name is empty or map metadata exceeds its bounded size",
        ));
    }
    if spec.width == 0
        || spec.height == 0
        || spec.width > MAP_MAX_WIDTH
        || spec.height > MAP_MAX_HEIGHT
    {
        return Err(edit_error(
            "EDIT_MAP_DIMENSIONS_INVALID",
            format!(
                "map dimensions {}x{} exceed the compatible {}x{} limit",
                spec.width, spec.height, MAP_MAX_WIDTH, MAP_MAX_HEIGHT
            ),
        ));
    }
    if u32::from(spec.border_margin) * 2 >= spec.width.min(spec.height) {
        return Err(edit_error(
            "EDIT_MAP_MARGIN_INVALID",
            "border margin leaves no buildable tile",
        ));
    }
    if spec.seed > u64::from(u32::MAX)
        || spec.reserved_percent > 90
        || spec.variant_tile_ids.len() > 128
        || spec.densities.len() > MAP_MAX_DENSITY_RULES
    {
        return Err(edit_error(
            "EDIT_MAP_LIMIT_EXCEEDED",
            "map seed, variants, densities, or reserved percentage exceed the bounded limits",
        ));
    }
    let supported = [
        "creature",
        "door",
        "encounter",
        "item",
        "placeable",
        "sound",
        "store",
        "trigger",
        "waypoint",
    ];
    let mut categories = BTreeSet::new();
    for rule in &spec.densities {
        if !supported.contains(&rule.category.as_str())
            || rule.per_hundred_tiles > 100
            || rule.min_spacing_tiles > 64
            || rule.template_resrefs.len() > MAP_MAX_BLUEPRINTS_PER_RULE
            || !categories.insert(rule.category.to_ascii_lowercase())
        {
            return Err(edit_error(
                "EDIT_MAP_DENSITY_INVALID",
                format!("invalid or duplicate density rule for {:?}", rule.category),
            ));
        }
        for resref in &rule.template_resrefs {
            validate_resref(resref)?;
        }
    }
    Ok(())
}

fn normalized_tile_ids(spec: &MapGenerationSpec) -> Vec<u32> {
    let mut values = vec![spec.base_tile_id];
    for value in &spec.variant_tile_ids {
        if !values.contains(value) {
            values.push(*value);
        }
    }
    values
}

fn coordinate_hash(seed: u64, x: u32, y: u32, salt: u64) -> u64 {
    let mut value = seed
        ^ (u64::from(x).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ (u64::from(y).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ salt;
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn text_hash(value: &str) -> u64 {
    let digest = Sha256::digest(value.as_bytes());
    u64::from_le_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 prefix has eight bytes"),
    )
}

fn hash_plan(plan: &MapGenerationPlan) -> String {
    let mut normalized = plan.clone();
    normalized.plan_sha256.clear();
    hex::encode(Sha256::digest(
        serde_json::to_vec(&normalized).expect("map plan serializes"),
    ))
}

fn short_category(category: &str) -> &str {
    match category {
        "creature" => "cre",
        "door" => "door",
        "encounter" => "enc",
        "item" => "item",
        "placeable" => "plc",
        "sound" => "snd",
        "store" => "store",
        "trigger" => "trg",
        "waypoint" => "wp",
        _ => "obj",
    }
}

fn category_label(category: &str) -> &str {
    match category {
        "creature" => "Créatures",
        "door" => "Portes",
        "encounter" => "Rencontres",
        "item" => "Objets",
        "placeable" => "Plaçables",
        "sound" => "Sons",
        "store" => "Marchands",
        "trigger" => "Déclencheurs",
        "waypoint" => "Points de passage",
        _ => "Éléments",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MapGenerationSpec {
        MapGenerationSpec {
            schema_version: MAP_GENERATION_SCHEMA_VERSION,
            brief: "Une place de village lisible".to_owned(),
            resref: "village".to_owned(),
            name: "Village".to_owned(),
            tileset: "tno01".to_owned(),
            width: 10,
            height: 8,
            seed: 42,
            base_tile_id: 0,
            variant_tile_ids: vec![1, 2],
            border_margin: 1,
            reserved_percent: 20,
            densities: vec![
                MapDensityRule {
                    category: "placeable".to_owned(),
                    per_hundred_tiles: 25,
                    min_spacing_tiles: 1,
                    template_resrefs: vec!["plc_table".to_owned(), "plc_chair".to_owned()],
                },
                MapDensityRule {
                    category: "creature".to_owned(),
                    per_hundred_tiles: 10,
                    min_spacing_tiles: 2,
                    template_resrefs: vec!["villager".to_owned()],
                },
            ],
        }
    }

    #[test]
    fn identical_specs_produce_identical_maps() {
        let first = generate_map_plan(&fixture()).expect("first map plan");
        let second = generate_map_plan(&fixture()).expect("second map plan");
        assert_eq!(first, second);
        assert_eq!(first.tiles.len(), 80);
        assert!(!first.placements.is_empty());
        assert_eq!(first.plan_sha256.len(), 64);
    }

    #[test]
    fn seed_changes_the_layout_without_changing_the_contract() {
        let first = generate_map_plan(&fixture()).expect("first map plan");
        let mut changed = fixture();
        changed.seed = 43;
        let second = generate_map_plan(&changed).expect("second map plan");
        assert_ne!(first.plan_sha256, second.plan_sha256);
        assert_ne!(first.placements, second.placements);
        assert_eq!(first.metrics.total_tiles, second.metrics.total_tiles);
    }

    #[test]
    fn borders_and_reserved_space_bound_density() {
        let plan = generate_map_plan(&fixture()).expect("map plan");
        assert_eq!(plan.metrics.buildable_tiles, 48);
        assert!(plan.metrics.reserved_tiles <= plan.metrics.buildable_tiles);
        assert!(plan.metrics.occupied_percent <= 100);
        assert!(plan.placements.iter().all(|placement| {
            placement.x >= 15.0 && placement.y >= 15.0 && placement.x <= 85.0 && placement.y <= 65.0
        }));
    }

    #[test]
    fn rejects_invalid_density_and_margin() {
        let mut invalid = fixture();
        invalid.border_margin = 4;
        assert!(generate_map_plan(&invalid).is_err());
        invalid.border_margin = 1;
        invalid.densities[0].per_hundred_tiles = 101;
        assert!(generate_map_plan(&invalid).is_err());

        let mut oversized = fixture();
        oversized.width = MAP_MAX_WIDTH + 1;
        assert!(generate_map_plan(&oversized).is_err());
    }

    #[test]
    fn materialized_are_and_git_are_reopenable_and_reproducible() {
        let plan = generate_map_plan(&fixture()).expect("map plan");
        let first = crate::create_generated_map_resources(&plan).expect("first resource set");
        let second = crate::create_generated_map_resources(&plan).expect("second resource set");
        assert_eq!(first, second);
        let are = first
            .iter()
            .find(|resource| resource.key.resource_type == 2012)
            .expect("ARE");
        let git = first
            .iter()
            .find(|resource| resource.key.resource_type == 2023)
            .expect("GIT");
        let are_document = aurora_gff::parse_gff(&are.bytes, "generated.are").expect("parse ARE");
        let git_document = aurora_gff::parse_gff(&git.bytes, "generated.git").expect("parse GIT");
        let tile_count = are_document
            .root
            .fields
            .iter()
            .find(|field| field.label == "Tile_List")
            .and_then(|field| match &field.value {
                aurora_gff::GenericValue::List(values) => Some(values.len()),
                _ => None,
            });
        let placeable_count = git_document
            .root
            .fields
            .iter()
            .find(|field| field.label == "Placeable List")
            .and_then(|field| match &field.value {
                aurora_gff::GenericValue::List(values) => Some(values.len()),
                _ => None,
            });
        assert_eq!(tile_count, Some(plan.metrics.total_tiles));
        assert_eq!(
            placeable_count,
            Some(
                plan.placements
                    .iter()
                    .filter(|placement| placement.category == "placeable")
                    .count()
            )
        );
    }
}
