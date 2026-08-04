use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use aurora_erf::{ContainerReader, ErfReader};
use aurora_gff::{GenericValue, parse_gff};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let module_path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: inspect_module_gff <module.mod> <resref>")?;
    let requested_resref = env::args()
        .nth(2)
        .ok_or("usage: inspect_module_gff <module.mod> <resref>")?
        .to_ascii_lowercase();
    let detail = env::args().nth(3);
    let root_summary = detail.as_deref() == Some("--root-summary");
    let requested_field = detail
        .as_deref()
        .and_then(|value| value.strip_prefix("--field="));
    let requested_index = env::args()
        .nth(4)
        .and_then(|value| value.strip_prefix("--index=").map(str::to_owned))
        .and_then(|value| value.parse::<usize>().ok());
    let list_summary = env::args().nth(4).as_deref() == Some("--list-summary");
    let cancelled = AtomicBool::new(false);
    let reader = ErfReader::default();
    let inventory = reader.read_inventory(&module_path, &cancelled)?;
    let mut found = false;
    for resource in inventory.resources.iter().filter(|resource| {
        (requested_resref == "*" || resource.key.resref == requested_resref)
            && matches!(
                resource.key.resource_type,
                2012 | 2014
                    | 2023
                    | 2025
                    | 2027
                    | 2029
                    | 2032
                    | 2035
                    | 2038
                    | 2040
                    | 2042
                    | 2044
                    | 2046
                    | 2051
                    | 2055
                    | 2056
                    | 2058
            )
    }) {
        let bytes = reader.read_resource(&module_path, resource, &cancelled)?;
        let document = parse_gff(&bytes, &resource.key.file_name())?;
        if let Some(requested_field) = requested_field {
            let Some(field) = document
                .root
                .fields
                .iter()
                .find(|field| field.label.eq_ignore_ascii_case(requested_field))
            else {
                continue;
            };
            println!("{}", resource.key);
            if list_summary {
                let GenericValue::List(values) = &field.value else {
                    return Err(
                        format!("{} field {requested_field} is not a list", resource.key).into(),
                    );
                };
                for (index, value) in values.iter().enumerate() {
                    let fields = value
                        .fields
                        .iter()
                        .filter_map(|field| match &field.value {
                            GenericValue::List(values) => {
                                Some(format!("{}=list[{}]", field.label, values.len()))
                            }
                            GenericValue::String(value) | GenericValue::ResRef(value) => matches!(
                                field.label.as_str(),
                                "Tag" | "TemplateResRef" | "LocName" | "LocalizedName"
                            )
                            .then(|| format!("{}={value:?}", field.label)),
                            GenericValue::Byte(value) if field.label == "HasInventory" => {
                                Some(format!("HasInventory={value}"))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("[{index}] {fields}");
                }
            } else if let (Some(index), GenericValue::List(values)) =
                (requested_index, &field.value)
            {
                let value = values.get(index).ok_or_else(|| {
                    format!(
                        "{} field {requested_field} has no list entry {index}",
                        resource.key
                    )
                })?;
                println!("{}", serde_json::to_string_pretty(value)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&field.value)?);
            }
        } else if root_summary {
            println!("{}", resource.key);
            for field in &document.root.fields {
                let detail = match &field.value {
                    GenericValue::List(values) => format!("list[{}]", values.len()),
                    GenericValue::Struct(value) => {
                        let fields = value
                            .fields
                            .iter()
                            .map(|field| {
                                format!(
                                    "{}:{}={}",
                                    field.label,
                                    field.field_type,
                                    serde_json::to_string(&field.value)
                                        .unwrap_or_else(|_| "?".to_owned())
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("struct(type={}, fields=[{}])", value.struct_type, fields)
                    }
                    _ => serde_json::to_string(&field.value)?,
                };
                println!("{}\t{}\t{}", field.label, field.field_type, detail);
            }
        } else {
            println!("{}", serde_json::to_string_pretty(&document)?);
        }
        found = true;
    }
    if !found {
        return Err(format!("no supported GFF resource named {requested_resref}").into());
    }
    Ok(())
}
