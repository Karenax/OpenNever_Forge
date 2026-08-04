use crate::dialogues::DialogueTables;
use crate::{DependencyRoots, ModuleDependencyReport};
use aurora_2da::{TwoDaTable, parse_2da};
use aurora_core::ResourceKey;
use aurora_dialogue::DialogueIndex;
use aurora_gff::{GenericGff, GenericStruct, GenericValue, parse_gff};
use aurora_nwscript::ScriptIndex;
use aurora_resource::{ResourceCatalog, ResourceManager};
use aurora_world::{
    Confidence, DiagnosticSeverity, Evidence, GraphEdge, GraphNode, NarrativeRelation,
    SceneAssetMap, SceneManifest, WorldDiagnostic, WorldIndex, adapt_area, adapt_narrative,
    inspect_asset, parse_set_tile_models, scene_manifest,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_ASSET_PROBES: usize = 2_048;

pub fn analyze_world(
    catalog: &ResourceCatalog,
    scripts: &ScriptIndex,
    dialogues: &DialogueIndex,
    dependencies: &ModuleDependencyReport,
    roots: &DependencyRoots,
    cancelled: &AtomicBool,
) -> WorldIndex {
    let mut parsed = BTreeMap::<ResourceKey, GenericGff>::new();
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        if !matches!(resource.key.resource_type, 2012 | 2023 | 2038 | 2046 | 2056) {
            continue;
        }
        if let Ok(bytes) = ResourceManager::read(&resource.selected, cancelled)
            && let Ok(gff) = parse_gff(&bytes, &resource.key.to_string())
        {
            parsed.insert(resource.key.clone(), gff);
        }
    }
    let journal = parsed
        .iter()
        .find(|(key, _)| key.resource_type == 2056)
        .map(|(_, value)| value);
    let factions = parsed
        .iter()
        .find(|(key, _)| key.resource_type == 2038)
        .map(|(_, value)| value);
    let mut index = WorldIndex {
        narrative: adapt_narrative(journal, factions),
        ..WorldIndex::default()
    };
    let tables = DialogueTables::load(dependencies, roots);
    for category in &mut index.narrative.categories {
        category.name.text =
            tables.resolve_parts(category.name.string_ref, category.name.text.as_deref());
        for entry in &mut category.entries {
            entry.text.text =
                tables.resolve_parts(entry.text.string_ref, entry.text.text.as_deref());
        }
    }
    for (key, are) in parsed.iter().filter(|(key, _)| key.resource_type == 2012) {
        index.areas.push(adapt_area(
            &key.resref,
            are,
            parsed.get(&ResourceKey::new(&key.resref, 2023)),
            parsed.get(&ResourceKey::new(&key.resref, 2046)),
        ));
    }

    let mut asset_count = 0;
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        if !matches!(
            resource.key.resource_type,
            3 | 6 | 2002 | 2022 | 2033 | 2072 | 2073 | 2079 | 2080 | 2081
        ) {
            continue;
        }
        asset_count += 1;
        if index.assets.assets.len() >= MAX_ASSET_PROBES {
            continue;
        }
        match ResourceManager::read(&resource.selected, cancelled) {
            Ok(bytes) => index.assets.assets.push(inspect_asset(
                resource.key.clone(),
                resource.selected.source_path.clone(),
                &bytes,
            )),
            Err(error) => index.diagnostics.push(WorldDiagnostic {
                code: error.code.clone(),
                severity: DiagnosticSeverity::Error,
                message: error.technical_message.clone(),
                resource: resource.key.to_string(),
                evidence: None,
            }),
        }
    }
    if asset_count > MAX_ASSET_PROBES {
        index.diagnostics.push(WorldDiagnostic {
            code: "ASSET_PROBE_LIMIT".to_owned(),
            severity: DiagnosticSeverity::Info,
            message: format!(
                "{asset_count} assets découverts ; {MAX_ASSET_PROBES} inspectés dans le lot interactif"
            ),
            resource: "resource-manager".to_owned(),
            evidence: None,
        });
    }
    index.scenes = build_scene_manifests(&index.areas, catalog, cancelled);
    build_global_graph(&mut index, catalog, scripts, dialogues);
    index.finalize();
    index
}

fn build_scene_manifests(
    areas: &[aurora_world::AreaMap],
    catalog: &ResourceCatalog,
    cancelled: &AtomicBool,
) -> Vec<SceneManifest> {
    let known_models = catalog
        .entries
        .iter()
        .filter(|value| value.key.resource_type == 2002)
        .map(|value| value.key.resref.clone())
        .collect::<BTreeSet<_>>();
    let walkmesh_models = catalog
        .entries
        .iter()
        .filter(|value| matches!(value.key.resource_type, 2016 | 2052 | 2053))
        .map(|value| value.key.resref.clone())
        .collect::<BTreeSet<_>>();
    let tables = [
        "appearance",
        "placeables",
        "genericdoors",
        "doortypes",
        "racialtypes",
    ]
    .into_iter()
    .filter_map(|name| load_2da(catalog, name, cancelled).map(|table| (name, table)))
    .collect::<BTreeMap<_, _>>();
    let mut blueprints = BTreeMap::<ResourceKey, Option<GenericGff>>::new();
    let mut scenes = Vec::with_capacity(areas.len());
    for area in areas {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let mut assets = SceneAssetMap {
            known_models: known_models.clone(),
            walkmesh_models: walkmesh_models.clone(),
            ..SceneAssetMap::default()
        };
        let mut diagnostics = Vec::new();
        match area.tileset.as_deref() {
            Some(tileset) => {
                let key = ResourceKey::new(tileset, 2013);
                match catalog.get(&key) {
                    Some(resource) => {
                        match ResourceManager::read(&resource.selected, cancelled) {
                            Ok(bytes) => {
                                assets.tile_models = parse_set_tile_models(&bytes);
                                if assets.tile_models.is_empty() {
                                    diagnostics.push(scene_diagnostic(
                                    "AREA_TILESET_INVALID",
                                    DiagnosticSeverity::Warning,
                                    format!("Le SET {tileset} ne contient aucun modèle de tuile lisible"),
                                    key.to_string(),
                                    "SET.Tile.Model",
                                ));
                                }
                            }
                            Err(error) => diagnostics.push(scene_diagnostic(
                                &error.code,
                                DiagnosticSeverity::Error,
                                error.technical_message.clone(),
                                key.to_string(),
                                "ResourceManager.read",
                            )),
                        }
                    }
                    None => diagnostics.push(scene_diagnostic(
                        "AREA_TILESET_MISSING",
                        DiagnosticSeverity::Warning,
                        format!("Tileset {tileset}.set non résolu par le Resource Manager"),
                        key.to_string(),
                        "ARE.Tileset",
                    )),
                }
            }
            None => diagnostics.push(scene_diagnostic(
                "AREA_TILESET_MISSING",
                DiagnosticSeverity::Warning,
                "La zone ne déclare aucun tileset".to_owned(),
                area.resref.clone(),
                "ARE.Tileset",
            )),
        }
        for instance in &area.instances {
            let Some((resource_type, table_names)) = instance_model_source(&instance.category)
            else {
                continue;
            };
            let blueprint_key = instance
                .template_resref
                .as_ref()
                .map(|resref| ResourceKey::new(resref, resource_type));
            let blueprint = blueprint_key.as_ref().and_then(|key| {
                blueprints
                    .entry(key.clone())
                    .or_insert_with(|| {
                        catalog.get(key).and_then(|resource| {
                            ResourceManager::read(&resource.selected, cancelled)
                                .ok()
                                .and_then(|bytes| parse_gff(&bytes, &key.to_string()).ok())
                        })
                    })
                    .as_ref()
            });
            let direct_model = blueprint.and_then(|value| {
                struct_string(&value.root, &["ModelName", "Model", "ModelResRef"])
            });
            let appearance = blueprint
                .and_then(|value| {
                    struct_unsigned(
                        &value.root,
                        &[
                            "Appearance_Type",
                            "AppearanceType",
                            "Appearance",
                            "GenericType",
                        ],
                    )
                })
                .or(instance.appearance);
            let model = direct_model.or_else(|| {
                appearance.and_then(|row| {
                    table_names.iter().find_map(|table_name| {
                        tables
                            .get(table_name)
                            .and_then(|table| model_from_2da(table, row))
                    })
                })
            });
            let mut models = model
                .as_deref()
                .map(normalize_model_resref)
                .into_iter()
                .collect::<Vec<_>>();
            if instance.category == "creature"
                && !models
                    .iter()
                    .all(|model| assets.known_models.contains(model))
                && let Some(blueprint) = blueprint
            {
                let parts = creature_part_models(
                    &blueprint.root,
                    model.as_deref(),
                    tables.get("racialtypes"),
                    &assets.known_models,
                );
                if !parts.is_empty() {
                    models = parts;
                }
            }
            if !models.is_empty() {
                assets.instance_models.insert(instance.id.clone(), models);
            } else {
                diagnostics.push(scene_diagnostic(
                    "INSTANCE_MODEL_UNRESOLVED",
                    DiagnosticSeverity::Warning,
                    format!(
                        "Aucun modèle {} résolu depuis le blueprint et les 2DA",
                        instance.category
                    ),
                    instance
                        .template_resref
                        .clone()
                        .unwrap_or_else(|| instance.id.clone()),
                    "Blueprint.Appearance -> 2DA.ModelName",
                ));
            }
        }
        let mut manifest = scene_manifest(area, &assets);
        for object in manifest
            .objects
            .iter()
            .filter(|value| !value.model_resrefs.is_empty() && value.marker)
        {
            diagnostics.push(scene_diagnostic(
                "AREA_MODEL_MISSING",
                DiagnosticSeverity::Warning,
                format!(
                    "Modèle {} absent du catalogue résolu",
                    object.model_resrefs.join(", ")
                ),
                object.label.clone(),
                "SceneManifest.modelResref",
            ));
        }
        diagnostics.sort_by(|left, right| {
            (&left.code, &left.resource, &left.message).cmp(&(
                &right.code,
                &right.resource,
                &right.message,
            ))
        });
        diagnostics.dedup();
        manifest.diagnostics.extend(diagnostics);
        scenes.push(manifest);
    }
    scenes
}

fn load_2da(catalog: &ResourceCatalog, resref: &str, cancelled: &AtomicBool) -> Option<TwoDaTable> {
    let key = ResourceKey::new(resref, 2017);
    let resource = catalog.get(&key)?;
    let bytes = ResourceManager::read(&resource.selected, cancelled).ok()?;
    parse_2da(&bytes, &key.to_string()).ok()
}

fn instance_model_source(category: &str) -> Option<(u16, &'static [&'static str])> {
    match category {
        "creature" => Some((2027, &["appearance"])),
        "placeable" => Some((2044, &["placeables"])),
        "door" => Some((2042, &["genericdoors", "doortypes"])),
        _ => None,
    }
}

fn model_from_2da(table: &TwoDaTable, row: u32) -> Option<String> {
    let row_index = table
        .rows
        .iter()
        .position(|value| value.label.parse::<u32>().ok() == Some(row))
        .or_else(|| {
            usize::try_from(row)
                .ok()
                .filter(|value| *value < table.rows.len())
        })?;
    ["ModelName", "MODELNAME", "Model", "RACE", "ResRef"]
        .into_iter()
        .find_map(|column| table.cell(row_index, column).map(str::to_owned))
        .filter(|value| !value.trim().is_empty() && value != "****")
}

fn normalize_model_resref(value: &str) -> String {
    let value = value.trim().trim_matches('"').to_ascii_lowercase();
    value.strip_suffix(".mdl").unwrap_or(&value).to_owned()
}

fn creature_part_models(
    root: &GenericStruct,
    appearance_model: Option<&str>,
    racialtypes: Option<&TwoDaTable>,
    known_models: &BTreeSet<String>,
) -> Vec<String> {
    let gender = match struct_unsigned(root, &["Gender"]).unwrap_or(0) {
        1 => 'f',
        _ => 'm',
    };
    let phenotype = struct_unsigned(root, &["Phenotype"]).unwrap_or(0).min(9);
    let racial_abbreviation = struct_unsigned(root, &["Race", "RacialType"])
        .and_then(|row| racialtypes.and_then(|table| table_text(table, row, &["Abbreviation"])))
        .map(|value| value.to_ascii_lowercase());
    let appearance = appearance_model
        .map(normalize_model_resref)
        .filter(|value| !value.is_empty());
    let mut prefixes = BTreeSet::new();
    if let Some(race) = racial_abbreviation {
        prefixes.insert(format!("p{gender}{race}{phenotype}"));
        if let Some(initial) = race.chars().next() {
            prefixes.insert(format!("p{gender}{initial}{phenotype}"));
        }
    }
    if let Some(race) = appearance {
        prefixes.insert(format!("p{gender}{race}{phenotype}"));
    }
    let fields = [
        (&["Appearance_Head", "Head"][..], "head"),
        (&["BodyPart_Torso"][..], "chest"),
        (&["BodyPart_Belt"][..], "belt"),
        (&["BodyPart_Neck"][..], "neck"),
        (&["BodyPart_Pelvis"][..], "pelvis"),
        (&["BodyPart_LBicep"][..], "bicepl"),
        (&["BodyPart_RBicep"][..], "bicepr"),
        (&["BodyPart_LFArm"][..], "forearml"),
        (&["BodyPart_RFArm"][..], "forearmr"),
        (&["BodyPart_LFoot"][..], "footl"),
        (&["BodyPart_RFoot"][..], "footr"),
        (&["BodyPart_LHand"][..], "handl"),
        (&["BodyPart_RHand"][..], "handr"),
        (&["BodyPart_LShin"][..], "shinl"),
        (&["BodyPart_RShin"][..], "shinr"),
        (&["BodyPart_LShoul"][..], "shoulderl"),
        (&["BodyPart_RShoul"][..], "shoulderr"),
        (&["BodyPart_LThigh"][..], "thighl"),
        (&["BodyPart_RThigh"][..], "thighr"),
    ];
    let mut models = BTreeSet::new();
    for (names, part) in fields {
        let Some(variation) = struct_unsigned(root, names).filter(|value| *value < 1_000) else {
            continue;
        };
        for prefix in &prefixes {
            let candidate = format!("{prefix}_{part}{variation:03}");
            if known_models.contains(&candidate) {
                models.insert(candidate);
                break;
            }
        }
    }
    models.into_iter().collect()
}

fn table_text(table: &TwoDaTable, row: u32, columns: &[&str]) -> Option<String> {
    let row_index = table
        .rows
        .iter()
        .position(|value| value.label.parse::<u32>().ok() == Some(row))
        .or_else(|| {
            usize::try_from(row)
                .ok()
                .filter(|value| *value < table.rows.len())
        })?;
    columns
        .iter()
        .find_map(|column| table.cell(row_index, column).map(str::to_owned))
        .filter(|value| !value.trim().is_empty() && value != "****")
}

fn struct_value<'a>(root: &'a GenericStruct, names: &[&str]) -> Option<&'a GenericValue> {
    root.fields
        .iter()
        .find(|field| {
            names
                .iter()
                .any(|name| field.label.eq_ignore_ascii_case(name))
        })
        .map(|field| &field.value)
}

fn struct_string(root: &GenericStruct, names: &[&str]) -> Option<String> {
    match struct_value(root, names)? {
        GenericValue::String(value) | GenericValue::ResRef(value) => Some(value.clone()),
        _ => None,
    }
}

fn struct_unsigned(root: &GenericStruct, names: &[&str]) -> Option<u32> {
    match struct_value(root, names)? {
        GenericValue::Byte(value) => Some((*value).into()),
        GenericValue::Word(value) => Some((*value).into()),
        GenericValue::Dword(value) => Some(*value),
        GenericValue::Int(value) => u32::try_from(*value).ok(),
        _ => None,
    }
}

fn scene_diagnostic(
    code: &str,
    severity: DiagnosticSeverity,
    message: String,
    resource: String,
    field_path: &str,
) -> WorldDiagnostic {
    WorldDiagnostic {
        code: code.to_owned(),
        severity,
        message,
        resource: resource.clone(),
        evidence: Some(Evidence {
            resource: basename(&resource),
            field_path: field_path.to_owned(),
        }),
    }
}

fn build_global_graph(
    index: &mut WorldIndex,
    catalog: &ResourceCatalog,
    scripts: &ScriptIndex,
    dialogues: &DialogueIndex,
) {
    let known_resrefs = catalog
        .entries
        .iter()
        .map(|value| value.key.resref.clone())
        .collect::<BTreeSet<_>>();
    for category in &index.narrative.categories {
        let category_id = format!("journal:{}", category.tag.to_ascii_lowercase());
        index
            .graph_nodes
            .push(node(&category_id, "journal", &category.tag, None));
        for entry in &category.entries {
            let entry_id = format!("{category_id}:{}", entry.id);
            index.graph_nodes.push(node(
                &entry_id,
                "journal_entry",
                entry.text.text.as_deref().unwrap_or("Étape"),
                None,
            ));
            index.graph_edges.push(edge(
                format!("contains:{category_id}:{entry_id}"),
                &category_id,
                &entry_id,
                "contains",
                Confidence::Certain,
                &category.source,
                "EntryList",
            ));
        }
    }
    for faction in &index.narrative.factions {
        let id = format!("faction:{}", faction.id);
        index
            .graph_nodes
            .push(node(&id, "faction", &faction.name, None));
        if let Some(parent) = faction.parent_id {
            index.graph_edges.push(edge(
                format!("parent:{id}:{parent}"),
                &id,
                &format!("faction:{parent}"),
                "parent",
                Confidence::Certain,
                "repute.fac",
                "FactionParentID",
            ));
        }
    }
    for reputation in &index.narrative.reputations {
        index.graph_edges.push(edge(
            format!(
                "reputation:{}:{}",
                reputation.source_id, reputation.target_id
            ),
            &format!("faction:{}", reputation.source_id),
            &format!("faction:{}", reputation.target_id),
            &format!("reputation:{}", reputation.value),
            Confidence::Certain,
            "repute.fac",
            "RepList",
        ));
    }
    let areas = index.areas.clone();
    for area in &areas {
        let area_id = format!("area:{}", area.resref);
        index.graph_nodes.push(node(
            &area_id,
            "area",
            area.name.text.as_deref().unwrap_or(&area.resref),
            Some(format!("{}.are", area.resref)),
        ));
        for instance in &area.instances {
            let instance_id = format!("instance:{}", instance.id);
            let label = instance
                .tag
                .as_deref()
                .or(instance.template_resref.as_deref())
                .unwrap_or(&instance.category);
            index.graph_nodes.push(node(
                &instance_id,
                &instance.category,
                label,
                instance.template_resref.clone(),
            ));
            index.graph_edges.push(edge(
                format!("contains:{area_id}:{instance_id}"),
                &area_id,
                &instance_id,
                "contains",
                Confidence::Certain,
                &instance.source_path,
                "instance-list",
            ));
            add_instance_links(index, &known_resrefs, &instance_id, instance);
        }
    }
    for document in &scripts.documents {
        let id = format!("script:{}", document.resref);
        index.graph_nodes.push(node(
            &id,
            "script",
            &document.resref,
            Some(format!("{}.nss", document.resref)),
        ));
        for reference in &document.inbound_references {
            let source = format!("resource:{}", reference.resource.resref);
            index.graph_nodes.push(node(
                &source,
                "resource",
                &reference.resource.to_string(),
                Some(reference.resource.to_string()),
            ));
            index.graph_edges.push(edge(
                format!("script-ref:{source}:{id}:{}", reference.field_path),
                &source,
                &id,
                "script_reference",
                Confidence::Certain,
                &reference.source,
                &reference.field_path,
            ));
        }
        if let Some(nss) = &document.nss {
            let categories = index.narrative.categories.clone();
            for category in categories {
                let tag = category.tag.to_ascii_lowercase();
                let Some((line_index, line)) = nss
                    .text
                    .lines()
                    .enumerate()
                    .find(|(_, line)| line.to_ascii_lowercase().contains(&tag))
                else {
                    continue;
                };
                let confidence = if line.to_ascii_lowercase().contains("journalquest") {
                    Confidence::Probable
                } else {
                    Confidence::Possible
                };
                let target = format!("journal:{tag}");
                let field_path = format!("line:{}", line_index + 1);
                index.graph_edges.push(edge(
                    format!("script-journal:{id}:{target}:{line_index}"),
                    &id,
                    &target,
                    "journal_reference",
                    confidence,
                    &nss.source,
                    &field_path,
                ));
                index.narrative.relations.push(NarrativeRelation {
                    source: id.clone(),
                    target,
                    kind: "journal_reference".to_owned(),
                    confidence,
                    evidence: Evidence {
                        resource: basename(&nss.source),
                        field_path,
                    },
                });
            }
        }
    }
    add_dialogue_graph(index, dialogues);
    for asset in &index.assets.assets {
        let id = format!("asset:{}", asset.key);
        index.graph_nodes.push(node(
            &id,
            "asset",
            &asset.key.to_string(),
            Some(asset.key.to_string()),
        ));
        for texture in &asset.textures {
            index.graph_edges.push(edge(
                format!("texture:{id}:{texture}"),
                &id,
                &format!("resource:{texture}"),
                "texture",
                Confidence::Probable,
                &asset.source,
                "bitmap",
            ));
        }
        if let Some(supermodel) = &asset.supermodel {
            index.graph_edges.push(edge(
                format!("supermodel:{id}:{supermodel}"),
                &id,
                &format!("asset:{supermodel}.mdl"),
                "supermodel",
                Confidence::Certain,
                &asset.source,
                "setsupermodel",
            ));
        }
        for reference in &asset.referenced_models {
            index.graph_edges.push(edge(
                format!("model-reference:{id}:{reference}"),
                &id,
                &format!("asset:{reference}.mdl"),
                "model_reference",
                Confidence::Certain,
                &asset.source,
                "refmodel",
            ));
        }
    }
    for resource in &catalog.entries {
        if !resource.shadowed.is_empty() {
            index.diagnostics.push(WorldDiagnostic {
                code: "RESOURCE_SHADOWED".to_owned(),
                severity: DiagnosticSeverity::Info,
                message: format!(
                    "{} version(s) masquée(s) par la priorité",
                    resource.shadowed.len()
                ),
                resource: resource.key.to_string(),
                evidence: Some(Evidence {
                    resource: basename(&resource.selected.source_path),
                    field_path: "ResourceManager.priority".to_owned(),
                }),
            });
        }
    }
}

fn add_instance_links(
    index: &mut WorldIndex,
    known_resrefs: &BTreeSet<String>,
    instance_id: &str,
    instance: &aurora_world::AreaInstance,
) {
    if let Some(template) = &instance.template_resref {
        let target = format!("resource:{}", template.to_ascii_lowercase());
        index
            .graph_nodes
            .push(node(&target, "resource", template, Some(template.clone())));
        index.graph_edges.push(edge(
            format!("template:{instance_id}:{target}"),
            instance_id,
            &target,
            "template",
            Confidence::Certain,
            &instance.source_path,
            "TemplateResRef",
        ));
        if !known_resrefs.contains(&template.to_ascii_lowercase()) {
            index.diagnostics.push(WorldDiagnostic {
                code: "BLUEPRINT_MISSING".to_owned(),
                severity: DiagnosticSeverity::Warning,
                message: format!("Blueprint {template} non résolu"),
                resource: instance.id.clone(),
                evidence: Some(Evidence {
                    resource: basename(&instance.source_path),
                    field_path: "TemplateResRef".to_owned(),
                }),
            });
        }
    }
    if let Some(destination) = &instance.transition_destination {
        index.graph_edges.push(edge(
            format!(
                "transition:{instance_id}:{}",
                destination.to_ascii_lowercase()
            ),
            instance_id,
            &format!("area:{}", destination.to_ascii_lowercase()),
            "transition",
            Confidence::Possible,
            &instance.source_path,
            "TransitionDestination",
        ));
        if !index
            .areas
            .iter()
            .any(|area| area.resref.eq_ignore_ascii_case(destination))
        {
            index.diagnostics.push(WorldDiagnostic {
                code: "TRANSITION_TARGET_UNRESOLVED".to_owned(),
                severity: DiagnosticSeverity::Warning,
                message: format!("Destination {destination} non rapprochée d’un ResRef de zone"),
                resource: instance.id.clone(),
                evidence: Some(Evidence {
                    resource: basename(&instance.source_path),
                    field_path: "TransitionDestination".to_owned(),
                }),
            });
        }
    }
}

fn add_dialogue_graph(index: &mut WorldIndex, dialogues: &DialogueIndex) {
    for dialogue in &dialogues.dialogues {
        let id = format!("dialogue:{}", dialogue.key.resref);
        index.graph_nodes.push(node(
            &id,
            "dialogue",
            &dialogue.key.resref,
            Some(dialogue.key.to_string()),
        ));
        for reference in &dialogue.references {
            let source = format!("resource:{}", reference.resource.resref);
            index.graph_nodes.push(node(
                &source,
                "resource",
                &reference.resource.to_string(),
                Some(reference.resource.to_string()),
            ));
            index.graph_edges.push(edge(
                format!("dialogue-ref:{source}:{id}:{}", reference.field_path),
                &source,
                &id,
                "dialogue_reference",
                Confidence::Certain,
                &reference.source,
                &reference.field_path,
            ));
        }
        for value in &dialogue.nodes {
            if let Some(script) = &value.action_script {
                index.graph_edges.push(edge(
                    format!("dialogue-script:{id}:{script}:{}", value.id),
                    &id,
                    &format!("script:{}", script.to_ascii_lowercase()),
                    "action_script",
                    Confidence::Certain,
                    &dialogue.source,
                    &value.id,
                ));
            }
            if let Some(quest) = &value.quest {
                let target = format!("journal:{}", quest.to_ascii_lowercase());
                index.graph_edges.push(edge(
                    format!("dialogue-journal:{id}:{target}:{}", value.id),
                    &id,
                    &target,
                    "journal_reference",
                    Confidence::Probable,
                    &dialogue.source,
                    &value.id,
                ));
                index.narrative.relations.push(NarrativeRelation {
                    source: id.clone(),
                    target,
                    kind: "journal_reference".to_owned(),
                    confidence: Confidence::Probable,
                    evidence: Evidence {
                        resource: basename(&dialogue.source),
                        field_path: value.id.clone(),
                    },
                });
            }
        }
    }
}

fn node(id: &str, kind: &str, label: &str, resource: Option<String>) -> GraphNode {
    GraphNode {
        id: id.to_owned(),
        kind: kind.to_owned(),
        label: label.to_owned(),
        resource,
    }
}

fn edge(
    id: String,
    source: &str,
    target: &str,
    kind: &str,
    confidence: Confidence,
    resource: &str,
    field_path: &str,
) -> GraphEdge {
    GraphEdge {
        id,
        source: source.to_owned(),
        target: target.to_owned(),
        kind: kind.to_owned(),
        confidence,
        evidence: Evidence {
            resource: basename(resource),
            field_path: field_path.to_owned(),
        },
    }
}

fn basename(value: &str) -> String {
    value.rsplit(['/', '\\']).next().unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_gff::GenericField;

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

    #[test]
    fn resolves_simple_and_composite_appearance_models_from_source_tables() {
        let placeables = parse_2da(b"2DA V2.0\nModelName\n0 plc_chest1\n", "placeables.2da")
            .expect("placeables");
        assert_eq!(
            model_from_2da(&placeables, 0).as_deref(),
            Some("plc_chest1")
        );

        let racialtypes =
            parse_2da(b"2DA V2.0\nAbbreviation\n6 Hu\n", "racialtypes.2da").expect("racialtypes");
        let root = structure(vec![
            ("Race", GenericValue::Byte(6)),
            ("Gender", GenericValue::Byte(0)),
            ("Phenotype", GenericValue::Int(2)),
            ("Appearance_Head", GenericValue::Byte(5)),
            ("BodyPart_Torso", GenericValue::Byte(1)),
        ]);
        let known = ["pmh2_head005", "pmh2_chest001"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            creature_part_models(&root, Some("H"), Some(&racialtypes), &known),
            vec!["pmh2_chest001", "pmh2_head005"]
        );
    }
}
