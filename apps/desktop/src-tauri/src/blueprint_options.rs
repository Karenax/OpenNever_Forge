use aurora_2da::{TwoDaTable, parse_2da};
use aurora_core::ResourceKey;
use aurora_project::{ModuleAnalysis, ResourceManager};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintFieldOptionsRequest {
    pub job_id: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintFieldOption {
    pub value: i64,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintFieldOptions {
    pub fields: BTreeMap<String, Vec<BlueprintFieldOption>>,
}

pub fn build_blueprint_field_options(
    analysis: &ModuleAnalysis,
    file_type: &str,
) -> BlueprintFieldOptions {
    let mut fields = BTreeMap::new();
    if file_type.trim() == "UTC" {
        fields.insert(
            "Gender".to_owned(),
            ["Masculin", "Féminin", "Les deux", "Autre"]
                .into_iter()
                .enumerate()
                .map(|(value, label)| BlueprintFieldOption {
                    value: value as i64,
                    label: label.to_owned(),
                    source: "règle Aurora Gender".to_owned(),
                })
                .collect(),
        );
        fields.insert(
            "FactionID".to_owned(),
            analysis
                .world_index
                .narrative
                .factions
                .iter()
                .map(|faction| BlueprintFieldOption {
                    value: i64::from(faction.id),
                    label: faction.name.clone(),
                    source: "factions du module".to_owned(),
                })
                .collect(),
        );
    }

    let specs: Vec<(&str, &[&str])> = match file_type.trim() {
        "UTC" => vec![
            ("Appearance_Type", &["appearance"]),
            ("Appearance", &["appearance"]),
            ("Race", &["racialtypes"]),
            ("PortraitId", &["portraits"]),
            ("SoundSetFile", &["soundset"]),
        ],
        "UTI" => vec![("BaseItem", &["baseitems"])],
        "UTP" => vec![("Appearance", &["placeables"])],
        "UTD" => vec![("Appearance", &["genericdoors", "doortypes"])],
        _ => Vec::new(),
    };
    for (field, table_names) in specs {
        let Some((table_name, table)) = table_names.iter().find_map(|table_name| {
            let key = ResourceKey::new(*table_name, 2017);
            let resource = analysis.resource_catalog.get(&key)?;
            let bytes = ResourceManager::read(&resource.selected, &AtomicBool::new(false)).ok()?;
            parse_2da(
                &bytes,
                &format!("{}::{}", resource.selected.source_path, key),
            )
            .ok()
            .map(|table| (*table_name, table))
        }) else {
            continue;
        };
        let options = table
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let value = row
                    .label
                    .parse::<i64>()
                    .ok()
                    .or_else(|| i64::try_from(index).ok())?;
                Some(BlueprintFieldOption {
                    value,
                    label: two_da_label(&table, index, &row.label),
                    source: format!("{table_name}.2da · ligne {}", row.label),
                })
            })
            .collect::<Vec<_>>();
        if !options.is_empty() {
            fields.insert(field.to_owned(), options);
        }
    }
    BlueprintFieldOptions { fields }
}

fn two_da_label(table: &TwoDaTable, row_index: usize, row_label: &str) -> String {
    const PREFERRED_COLUMNS: &[&str] = &[
        "Label",
        "LABEL",
        "Name",
        "NAME",
        "ItemClass",
        "ITEMCLASS",
        "BaseResRef",
        "BASERESREF",
        "ModelName",
        "MODELNAME",
        "Abbreviation",
        "Description",
    ];
    PREFERRED_COLUMNS
        .iter()
        .filter_map(|column| table.cell(row_index, column))
        .chain(
            table.rows[row_index]
                .cells
                .iter()
                .filter_map(Option::as_deref),
        )
        .map(str::trim)
        .find(|value| {
            !value.is_empty()
                && *value != "****"
                && value.parse::<i64>().is_err()
                && !value.eq_ignore_ascii_case(row_label)
        })
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Ligne {row_label}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_a_readable_symbol_instead_of_a_numeric_strref() {
        let table = parse_2da(
            b"2DA V2.0\nName ItemClass\n0 1234 longsword\n1 **** ****\n",
            "baseitems.2da",
        )
        .expect("valid 2DA");
        assert_eq!(two_da_label(&table, 0, "0"), "longsword");
        assert_eq!(two_da_label(&table, 1, "1"), "Ligne 1");
    }
}
