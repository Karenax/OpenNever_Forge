use crate::MODULE_BLUEPRINT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleRequirement {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaBlueprint {
    pub resref: String,
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub tileset: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub connects_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptBlueprint {
    pub resref: String,
    pub event: String,
    pub purpose: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueBlueprint {
    pub resref: String,
    pub owner_tag: String,
    pub purpose: String,
    #[serde(default)]
    pub required_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleBlueprint {
    pub schema_version: u32,
    pub name: String,
    pub tag: String,
    pub synopsis: String,
    pub entry_area: String,
    pub default_tileset: String,
    #[serde(default)]
    pub requirements: Vec<ModuleRequirement>,
    #[serde(default)]
    pub areas: Vec<AreaBlueprint>,
    #[serde(default)]
    pub scripts: Vec<ScriptBlueprint>,
    #[serde(default)]
    pub dialogues: Vec<DialogueBlueprint>,
    #[serde(default)]
    pub custom_tlk: Option<String>,
    #[serde(default)]
    pub hak_dependencies: Vec<String>,
}

impl ModuleBlueprint {
    pub fn new(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            schema_version: MODULE_BLUEPRINT_SCHEMA_VERSION,
            name: name.into(),
            tag: tag.into(),
            synopsis: String::new(),
            entry_area: "entry".to_owned(),
            default_tileset: "tno01".to_owned(),
            requirements: Vec::new(),
            areas: Vec::new(),
            scripts: Vec::new(),
            dialogues: Vec::new(),
            custom_tlk: None,
            hak_dependencies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintDiagnostic {
    UnsupportedSchema,
    MissingIdentity,
    MissingEntryArea,
    DuplicateArea,
    InvalidAreaSize,
    BrokenAreaConnection,
    DuplicateScript,
    DuplicateDialogue,
    DuplicateRequirement,
    InvalidIdentifier,
    LimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintValidation {
    pub valid: bool,
    pub diagnostics: Vec<(BlueprintDiagnostic, String)>,
    pub planned_resources: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintTask {
    pub id: String,
    pub capability_id: String,
    pub title: String,
    pub arguments: Value,
    pub depends_on: Vec<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintExecutionPlan {
    pub blueprint_sha256: String,
    pub validation: BlueprintValidation,
    pub tasks: Vec<BlueprintTask>,
}

pub fn compile_module_blueprint(blueprint: &ModuleBlueprint) -> BlueprintExecutionPlan {
    let validation = validate_module_blueprint(blueprint);
    let encoded = serde_json::to_vec(blueprint).unwrap_or_default();
    let blueprint_sha256 = hex::encode(Sha256::digest(encoded));
    let mut tasks = vec![BlueprintTask {
        id: "inspect-module".to_owned(),
        capability_id: "module.inspect".to_owned(),
        title: "Inspecter le module de travail".to_owned(),
        arguments: json!({}),
        depends_on: Vec::new(),
        required: true,
    }];
    for area in &blueprint.areas {
        tasks.push(BlueprintTask {
            id: format!("area-{}", area.resref),
            capability_id: "area.create".to_owned(),
            title: format!("Créer la zone {}", area.name),
            arguments: json!({
                "resref": area.resref,
                "name": area.name,
                "width": area.width,
                "height": area.height,
                "tileset": area.tileset,
                "tileId": 0,
            }),
            depends_on: vec!["inspect-module".to_owned()],
            required: true,
        });
    }
    for script in &blueprint.scripts {
        let create_id = format!("script-{}", script.resref);
        tasks.push(BlueprintTask {
            id: create_id.clone(),
            capability_id: "script.create".to_owned(),
            title: format!("Créer le script {}", script.resref),
            arguments: json!({
                "resref": script.resref,
                "event": script.event,
                "purpose": script.purpose,
                "source": script.source,
            }),
            depends_on: vec!["inspect-module".to_owned()],
            required: true,
        });
        tasks.push(BlueprintTask {
            id: format!("compile-{}", script.resref),
            capability_id: "script.compile".to_owned(),
            title: format!("Compiler le script {}", script.resref),
            arguments: json!({ "resref": script.resref }),
            depends_on: vec![create_id],
            required: true,
        });
    }
    for dialogue in &blueprint.dialogues {
        tasks.push(BlueprintTask {
            id: format!("dialogue-{}", dialogue.resref),
            capability_id: "dialogue.create".to_owned(),
            title: format!("Créer le dialogue {}", dialogue.resref),
            arguments: serde_json::to_value(dialogue).unwrap_or_else(|_| json!({})),
            depends_on: vec!["inspect-module".to_owned()],
            required: true,
        });
    }
    let build_dependencies = tasks
        .iter()
        .filter(|task| task.id != "inspect-module")
        .map(|task| task.id.clone())
        .collect();
    tasks.push(BlueprintTask {
        id: "validate-module".to_owned(),
        capability_id: "module.validate".to_owned(),
        title: "Valider le module construit".to_owned(),
        arguments: json!({}),
        depends_on: build_dependencies,
        required: true,
    });
    BlueprintExecutionPlan {
        blueprint_sha256,
        validation,
        tasks,
    }
}

pub fn validate_module_blueprint(blueprint: &ModuleBlueprint) -> BlueprintValidation {
    let mut diagnostics = Vec::new();
    let encoded_size = serde_json::to_vec(blueprint).map_or(usize::MAX, |bytes| bytes.len());
    if encoded_size > 4 * 1024 * 1024 {
        diagnostics.push((
            BlueprintDiagnostic::LimitExceeded,
            format!("blueprint uses {encoded_size} bytes; maximum is 4 MiB"),
        ));
    }
    if blueprint.areas.len() > 256
        || blueprint.scripts.len() > 4_096
        || blueprint.dialogues.len() > 2_048
        || blueprint.requirements.len() > 1_024
        || blueprint.hak_dependencies.len() > 256
    {
        diagnostics.push((
            BlueprintDiagnostic::LimitExceeded,
            "blueprint collection limit exceeded".to_owned(),
        ));
    }
    if blueprint.schema_version != MODULE_BLUEPRINT_SCHEMA_VERSION {
        diagnostics.push((
            BlueprintDiagnostic::UnsupportedSchema,
            format!("unsupported blueprint schema {}", blueprint.schema_version),
        ));
    }
    if blueprint.name.trim().is_empty()
        || blueprint.name.len() > 1_024
        || blueprint.tag.trim().is_empty()
        || blueprint.tag.len() > 128
        || blueprint.synopsis.len() > 1024 * 1024
    {
        diagnostics.push((
            BlueprintDiagnostic::MissingIdentity,
            "module name and tag are required".to_owned(),
        ));
    }
    for (kind, value) in [
        ("entry area", blueprint.entry_area.as_str()),
        ("default tileset", blueprint.default_tileset.as_str()),
    ] {
        if !valid_resref(value) {
            diagnostics.push((
                BlueprintDiagnostic::InvalidIdentifier,
                format!("invalid {kind} resref {value:?}"),
            ));
        }
    }
    let mut areas = BTreeSet::new();
    for area in &blueprint.areas {
        if !valid_resref(&area.resref)
            || !valid_resref(&area.tileset)
            || area.name.is_empty()
            || area.name.len() > 1_024
            || area.purpose.len() > 64 * 1024
            || area.connects_to.len() > 256
        {
            diagnostics.push((
                BlueprintDiagnostic::InvalidIdentifier,
                format!("invalid or oversized area definition {}", area.resref),
            ));
        }
        if !areas.insert(area.resref.to_ascii_lowercase()) {
            diagnostics.push((
                BlueprintDiagnostic::DuplicateArea,
                format!("duplicate area {}", area.resref),
            ));
        }
        if area.width == 0 || area.height == 0 || area.width > 32 || area.height > 32 {
            diagnostics.push((
                BlueprintDiagnostic::InvalidAreaSize,
                format!("area {} dimensions must be between 1 and 32", area.resref),
            ));
        }
    }
    if !areas.contains(&blueprint.entry_area.to_ascii_lowercase()) {
        diagnostics.push((
            BlueprintDiagnostic::MissingEntryArea,
            format!("entry area {} is not declared", blueprint.entry_area),
        ));
    }
    for area in &blueprint.areas {
        for target in &area.connects_to {
            if !valid_resref(target) {
                diagnostics.push((
                    BlueprintDiagnostic::InvalidIdentifier,
                    format!("invalid connected area resref {target:?}"),
                ));
            }
            if !areas.contains(&target.to_ascii_lowercase()) {
                diagnostics.push((
                    BlueprintDiagnostic::BrokenAreaConnection,
                    format!("area {} references missing area {target}", area.resref),
                ));
            }
        }
    }
    validate_unique(
        blueprint.scripts.iter().map(|value| value.resref.as_str()),
        BlueprintDiagnostic::DuplicateScript,
        "script",
        &mut diagnostics,
    );
    for script in &blueprint.scripts {
        if !valid_resref(&script.resref)
            || script.event.len() > 1_024
            || script.purpose.len() > 64 * 1024
            || script
                .source
                .as_ref()
                .is_some_and(|source| source.len() > 1024 * 1024)
        {
            diagnostics.push((
                BlueprintDiagnostic::InvalidIdentifier,
                format!("invalid or oversized script definition {}", script.resref),
            ));
        }
    }
    validate_unique(
        blueprint
            .dialogues
            .iter()
            .map(|value| value.resref.as_str()),
        BlueprintDiagnostic::DuplicateDialogue,
        "dialogue",
        &mut diagnostics,
    );
    for dialogue in &blueprint.dialogues {
        if !valid_resref(&dialogue.resref)
            || dialogue.owner_tag.len() > 128
            || dialogue.purpose.len() > 64 * 1024
            || dialogue.required_nodes.len() > 1_024
            || dialogue
                .required_nodes
                .iter()
                .any(|node| node.len() > 64 * 1024)
        {
            diagnostics.push((
                BlueprintDiagnostic::InvalidIdentifier,
                format!(
                    "invalid or oversized dialogue definition {}",
                    dialogue.resref
                ),
            ));
        }
    }
    validate_unique(
        blueprint.requirements.iter().map(|value| value.id.as_str()),
        BlueprintDiagnostic::DuplicateRequirement,
        "requirement",
        &mut diagnostics,
    );
    for requirement in &blueprint.requirements {
        if requirement.id.is_empty()
            || requirement.id.len() > 128
            || requirement.description.len() > 64 * 1024
            || requirement.acceptance_criteria.len() > 1_024
            || requirement
                .acceptance_criteria
                .iter()
                .any(|criterion| criterion.len() > 64 * 1024)
        {
            diagnostics.push((
                BlueprintDiagnostic::LimitExceeded,
                format!("invalid or oversized requirement {}", requirement.id),
            ));
        }
    }
    let planned_resources =
        1 + blueprint.areas.len() * 3 + blueprint.scripts.len() * 2 + blueprint.dialogues.len();
    BlueprintValidation {
        valid: diagnostics.is_empty(),
        diagnostics,
        planned_resources,
    }
}

fn valid_resref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 16
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_unique<'a>(
    values: impl Iterator<Item = &'a str>,
    diagnostic: BlueprintDiagnostic,
    kind: &str,
    diagnostics: &mut Vec<(BlueprintDiagnostic, String)>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.to_ascii_lowercase()) {
            diagnostics.push((diagnostic.clone(), format!("duplicate {kind} {value}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_a_connected_module_blueprint() {
        let mut blueprint = ModuleBlueprint::new("Test", "test_module");
        blueprint.entry_area = "entry".to_owned();
        blueprint.areas = vec![
            AreaBlueprint {
                resref: "entry".to_owned(),
                name: "Entrée".to_owned(),
                width: 2,
                height: 2,
                tileset: "tno01".to_owned(),
                purpose: "Départ".to_owned(),
                connects_to: vec!["crypt".to_owned()],
            },
            AreaBlueprint {
                resref: "crypt".to_owned(),
                name: "Crypte".to_owned(),
                width: 2,
                height: 2,
                tileset: "tno01".to_owned(),
                purpose: "Finale".to_owned(),
                connects_to: vec!["entry".to_owned()],
            },
        ];
        let validation = validate_module_blueprint(&blueprint);
        assert!(validation.valid, "{:?}", validation.diagnostics);
        assert_eq!(validation.planned_resources, 7);
    }

    #[test]
    fn rejects_missing_connections_and_duplicates() {
        let mut blueprint = ModuleBlueprint::new("Test", "test_module");
        blueprint.entry_area = "missing".to_owned();
        blueprint.areas = vec![AreaBlueprint {
            resref: "entry".to_owned(),
            name: "Entrée".to_owned(),
            width: 0,
            height: 2,
            tileset: "tno01".to_owned(),
            purpose: String::new(),
            connects_to: vec!["missing".to_owned()],
        }];
        let validation = validate_module_blueprint(&blueprint);
        assert!(!validation.valid);
        assert!(validation.diagnostics.len() >= 3);
    }

    #[test]
    fn rejects_non_aurora_resrefs_before_any_workspace_write() {
        let mut blueprint = ModuleBlueprint::new("Test", "test_module");
        blueprint.entry_area = "Entry Area With Spaces".to_owned();
        blueprint.areas.push(AreaBlueprint {
            resref: "Entry Area With Spaces".to_owned(),
            name: "Entrée".to_owned(),
            width: 1,
            height: 1,
            tileset: "tno01".to_owned(),
            purpose: String::new(),
            connects_to: Vec::new(),
        });
        let validation = validate_module_blueprint(&blueprint);
        assert!(!validation.valid);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|(kind, _)| { *kind == BlueprintDiagnostic::InvalidIdentifier })
        );
    }

    #[test]
    fn compiles_a_deterministic_task_graph() {
        let mut blueprint = ModuleBlueprint::new("Test", "test_module");
        blueprint.areas.push(AreaBlueprint {
            resref: "entry".to_owned(),
            name: "Entrée".to_owned(),
            width: 2,
            height: 2,
            tileset: "tno01".to_owned(),
            purpose: String::new(),
            connects_to: Vec::new(),
        });
        let first = compile_module_blueprint(&blueprint);
        let second = compile_module_blueprint(&blueprint);
        assert!(first.validation.valid);
        assert_eq!(first, second);
        assert_eq!(
            first.tasks.last().expect("last").capability_id,
            "module.validate"
        );
    }
}
