use crate::{DependencyRoots, ModuleDependencyKind, ModuleDependencyReport};
use aurora_2da::parse_2da;
use aurora_core::{ResourceKey, resource_extension};
use aurora_gff::{
    GenericField, GenericStruct, GenericValue, LocalizedString, ModuleInfo, parse_gff,
};
use aurora_resource::{ResourceCatalog, ResourceManager};
use aurora_tlk::{
    EmbeddedString, Gender, LocalizedStringRequest, LocalizedStringResolver,
    ResolvedLocalizedString, TalkTable,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GffValidationSummary {
    pub discovered: usize,
    pub parsed: usize,
    pub failed: usize,
    pub struct_count: u64,
    pub field_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableSummary {
    pub key: ResourceKey,
    pub source: String,
    pub columns: usize,
    pub rows: usize,
    pub shadowed_versions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TlkTableSummary {
    pub kind: String,
    pub source: String,
    pub language_id: u32,
    pub entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaDefinition {
    pub resref: String,
    pub name: Option<LocalizedString>,
    pub tag: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tileset: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintSummary {
    pub key: ResourceKey,
    pub tag: Option<String>,
    pub name: Option<LocalizedString>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceCategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaInstances {
    pub resref: String,
    pub total_instances: usize,
    pub categories: Vec<InstanceCategoryCount>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaToolsetData {
    pub resref: String,
    pub root_fields: usize,
    pub struct_count: u32,
    pub field_count: u32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredDiagnostic {
    pub code: String,
    pub resource: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredResourceSummary {
    pub gff: GffValidationSummary,
    pub two_da_tables: Vec<TableSummary>,
    pub talk_tables: Vec<TlkTableSummary>,
    pub resolved_module_name: Option<ResolvedLocalizedString>,
    pub areas: Vec<AreaDefinition>,
    pub area_instances: Vec<AreaInstances>,
    pub area_toolset_data: Vec<AreaToolsetData>,
    pub blueprints: Vec<BlueprintSummary>,
    pub diagnostics: Vec<StructuredDiagnostic>,
}

pub fn analyze_structured_resources(
    catalog: &ResourceCatalog,
    module_info: &ModuleInfo,
    dependencies: &ModuleDependencyReport,
    roots: &DependencyRoots,
    cancelled: &AtomicBool,
) -> StructuredResourceSummary {
    let mut summary = StructuredResourceSummary::default();
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        if is_gff(resource.key.resource_type) {
            summary.gff.discovered += 1;
            match ResourceManager::read(&resource.selected, cancelled)
                .and_then(|bytes| parse_gff(&bytes, &resource.key.to_string()))
            {
                Ok(gff) => {
                    summary.gff.parsed += 1;
                    summary.gff.struct_count += u64::from(gff.struct_count);
                    summary.gff.field_count += u64::from(gff.field_count);
                    if resource.key.resource_type == 2012 {
                        summary.areas.push(area_definition(
                            resource.key.resref.clone(),
                            &gff.root,
                            resource.selected.source_path.clone(),
                        ));
                    }
                    if resource.key.resource_type == 2023 {
                        summary.area_instances.push(area_instances(
                            resource.key.resref.clone(),
                            &gff.root,
                            resource.selected.source_path.clone(),
                        ));
                    }
                    if resource.key.resource_type == 2046 {
                        summary.area_toolset_data.push(AreaToolsetData {
                            resref: resource.key.resref.clone(),
                            root_fields: gff.root.fields.len(),
                            struct_count: gff.struct_count,
                            field_count: gff.field_count,
                            source: resource.selected.source_path.clone(),
                        });
                    }
                    if is_blueprint(resource.key.resource_type) {
                        summary.blueprints.push(blueprint_summary(
                            resource.key.clone(),
                            &gff.root,
                            resource.selected.source_path.clone(),
                        ));
                    }
                }
                Err(error) => {
                    summary.gff.failed += 1;
                    summary.diagnostics.push(StructuredDiagnostic {
                        code: error.code.clone(),
                        resource: resource.key.to_string(),
                        source: resource.selected.source_path.clone(),
                        message: error.technical_message.clone(),
                    });
                }
            }
        } else if resource.key.resource_type == 2017 {
            match ResourceManager::read(&resource.selected, cancelled)
                .and_then(|bytes| parse_2da(&bytes, &resource.key.to_string()))
            {
                Ok(table) => summary.two_da_tables.push(TableSummary {
                    key: resource.key.clone(),
                    source: resource.selected.source_path.clone(),
                    columns: table.columns.len(),
                    rows: table.rows.len(),
                    shadowed_versions: resource.shadowed.len(),
                }),
                Err(error) => summary.diagnostics.push(StructuredDiagnostic {
                    code: error.code.clone(),
                    resource: resource.key.to_string(),
                    source: resource.selected.source_path.clone(),
                    message: error.technical_message.clone(),
                }),
            }
        }
    }

    let dialog = load_tlk(find_dialog_tlk(roots).as_deref(), "dialog", &mut summary);
    let dialog_female_path = dialog
        .as_ref()
        .and_then(|table| female_variant(Path::new(&table.source)));
    let dialog_female = load_tlk(dialog_female_path.as_deref(), "dialog_female", &mut summary);
    let custom_path = dependencies
        .dependencies
        .iter()
        .find(|dependency| dependency.kind == ModuleDependencyKind::CustomTlk)
        .and_then(|dependency| dependency.selected_path.as_deref())
        .map(PathBuf::from);
    let custom = load_tlk(custom_path.as_deref(), "custom", &mut summary);
    let custom_female_path = custom
        .as_ref()
        .and_then(|table| female_variant(Path::new(&table.source)));
    let custom_female = load_tlk(custom_female_path.as_deref(), "custom_female", &mut summary);
    summary.resolved_module_name = Some(
        LocalizedStringResolver {
            dialog: dialog.as_ref(),
            dialog_female: dialog_female.as_ref(),
            custom: custom.as_ref(),
            custom_female: custom_female.as_ref(),
        }
        .resolve(&LocalizedStringRequest {
            string_ref: module_info.name.string_ref,
            embedded: module_info
                .name
                .values
                .iter()
                .map(|value| EmbeddedString {
                    language_id: value.language_id,
                    text: value.text.clone(),
                })
                .collect(),
            language_id: 0,
            gender: Gender::Male,
        }),
    );
    summary
}

fn female_variant(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let candidate = path.with_file_name(format!("{stem}f.tlk"));
    candidate.is_file().then_some(candidate)
}

fn load_tlk(
    path: Option<&Path>,
    kind: &str,
    summary: &mut StructuredResourceSummary,
) -> Option<TalkTable> {
    let path = path?;
    match TalkTable::from_file(path) {
        Ok(table) => {
            summary.talk_tables.push(TlkTableSummary {
                kind: kind.to_owned(),
                source: path.display().to_string(),
                language_id: table.language_id,
                entries: table.entries.len(),
            });
            Some(table)
        }
        Err(error) => {
            summary.diagnostics.push(StructuredDiagnostic {
                code: error.code.clone(),
                resource: format!("{kind}.tlk"),
                source: path.display().to_string(),
                message: error.technical_message.clone(),
            });
            None
        }
    }
}

fn find_dialog_tlk(roots: &DependencyRoots) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(user) = &roots.user_data_path {
        candidates.extend([user.join("tlk/dialog.tlk"), user.join("dialog.tlk")]);
    }
    if let Some(game) = &roots.game_install_path {
        candidates.extend([
            game.join("lang/en/data/dialog.tlk"),
            game.join("lang/fr/data/dialog.tlk"),
            game.join("data/tlk/dialog.tlk"),
            game.join("tlk/dialog.tlk"),
            game.join("dialog.tlk"),
        ]);
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn area_definition(resref: String, root: &GenericStruct, source: String) -> AreaDefinition {
    AreaDefinition {
        resref,
        name: locstring(root, &["Name", "LocalizedName"]),
        tag: string(root, &["Tag"]),
        width: unsigned(root, &["Width"]),
        height: unsigned(root, &["Height"]),
        tileset: string(root, &["Tileset"]),
        source,
    }
}

fn blueprint_summary(key: ResourceKey, root: &GenericStruct, source: String) -> BlueprintSummary {
    BlueprintSummary {
        key,
        tag: string(root, &["Tag"]),
        name: locstring(root, &["LocalizedName", "FirstName", "LocName"]),
        source,
    }
}

fn area_instances(resref: String, root: &GenericStruct, source: String) -> AreaInstances {
    let mut categories = root
        .fields
        .iter()
        .filter_map(|field| match &field.value {
            GenericValue::List(values) if !values.is_empty() => Some(InstanceCategoryCount {
                category: field.label.clone(),
                count: values.len(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    categories.sort_by(|left, right| left.category.cmp(&right.category));
    let total_instances = categories.iter().map(|value| value.count).sum();
    AreaInstances {
        resref,
        total_instances,
        categories,
        source,
    }
}

fn field<'a>(root: &'a GenericStruct, names: &[&str]) -> Option<&'a GenericField> {
    root.fields.iter().find(|field| {
        names
            .iter()
            .any(|name| field.label.eq_ignore_ascii_case(name))
    })
}
fn string(root: &GenericStruct, names: &[&str]) -> Option<String> {
    match &field(root, names)?.value {
        GenericValue::String(value) | GenericValue::ResRef(value) => Some(value.clone()),
        _ => None,
    }
}
fn locstring(root: &GenericStruct, names: &[&str]) -> Option<LocalizedString> {
    match &field(root, names)?.value {
        GenericValue::LocalizedString(value) => Some(value.clone()),
        _ => None,
    }
}
fn unsigned(root: &GenericStruct, names: &[&str]) -> Option<u32> {
    match field(root, names)?.value {
        GenericValue::Byte(value) => Some(u32::from(value)),
        GenericValue::Word(value) => Some(u32::from(value)),
        GenericValue::Dword(value) => Some(value),
        _ => None,
    }
}

fn is_gff(resource_type: u16) -> bool {
    matches!(
        resource_extension(resource_type),
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
    )
}
fn is_blueprint(resource_type: u16) -> bool {
    matches!(
        resource_extension(resource_type),
        Some("uti" | "utc" | "utt" | "uts" | "ute" | "utd" | "utp" | "utm" | "utw")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_gff_and_blueprint_resource_types() {
        assert!(is_gff(2012));
        assert!(is_blueprint(2027));
        assert!(!is_gff(2017));
    }
}
