use crate::{ResourceCatalog, ResourceManager};
use aurora_core::{ResourceKey, resource_extension};
use aurora_gff::{GenericStruct, GenericValue, parse_gff};
use aurora_nwscript::{
    InboundScriptReference, ScriptDiagnostic, ScriptIndex, group_documents, inspect_ncs, parse_nss,
};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn analyze_scripts(catalog: &ResourceCatalog, cancelled: &AtomicBool) -> ScriptIndex {
    let mut nss = Vec::new();
    let mut ncs = Vec::new();
    let mut read_diagnostics = Vec::<(String, ScriptDiagnostic)>::new();
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let extension = resource_extension(resource.key.resource_type);
        if !matches!(extension, Some("nss" | "ncs")) {
            continue;
        }
        let source = format!("{}::{}", resource.selected.source_path, resource.key);
        match ResourceManager::read(&resource.selected, cancelled) {
            Ok(bytes) if extension == Some("nss") => match parse_nss(&bytes, &source) {
                Ok(document) => nss.push((resource.key.resref.clone(), document)),
                Err(error) => read_diagnostics.push((
                    resource.key.resref.clone(),
                    diagnostic(&resource.key, &error),
                )),
            },
            Ok(bytes) => match inspect_ncs(&bytes, &source) {
                Ok(document) => ncs.push((resource.key.resref.clone(), document)),
                Err(error) => read_diagnostics.push((
                    resource.key.resref.clone(),
                    diagnostic(&resource.key, &error),
                )),
            },
            Err(error) => read_diagnostics.push((
                resource.key.resref.clone(),
                diagnostic(&resource.key, &error),
            )),
        }
    }
    let mut index = group_documents(nss, ncs);
    for (resref, value) in read_diagnostics {
        if let Some(document) = index.documents.iter_mut().find(|doc| doc.resref == resref) {
            document.diagnostics.push(value);
        }
    }
    resolve_includes(&mut index);
    collect_inbound_references(catalog, &mut index, cancelled);
    index.finalize();
    index
}

fn resolve_includes(index: &mut ScriptIndex) {
    let available = index
        .documents
        .iter()
        .filter(|value| value.nss.is_some())
        .map(|value| value.resref.clone())
        .collect::<BTreeSet<_>>();
    for document in &mut index.documents {
        let Some(nss) = &mut document.nss else {
            document.diagnostics.push(ScriptDiagnostic {
                code: "NSS_SOURCE_MISSING".into(),
                message: "Le bytecode NCS existe mais sa source NSS est absente.".into(),
                line: None,
                resource: format!("{}.ncs", document.resref),
            });
            continue;
        };
        for include in &mut nss.includes {
            include.resolved = available.contains(&include.resref);
            if !include.resolved {
                nss.diagnostics.push(ScriptDiagnostic {
                    code: "NSS_INCLUDE_MISSING".into(),
                    message: format!(
                        "Include {}.nss introuvable dans le catalogue résolu.",
                        include.resref
                    ),
                    line: Some(include.line),
                    resource: format!("{}.nss", document.resref),
                });
            }
        }
    }
}

fn collect_inbound_references(
    catalog: &ResourceCatalog,
    index: &mut ScriptIndex,
    cancelled: &AtomicBool,
) {
    let known = index
        .documents
        .iter()
        .map(|value| value.resref.clone())
        .collect::<BTreeSet<_>>();
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        if !is_script_bearing_gff(resource.key.resource_type) {
            continue;
        }
        let Ok(bytes) = ResourceManager::read(&resource.selected, cancelled) else {
            continue;
        };
        let source = format!("{}::{}", resource.selected.source_path, resource.key);
        let Ok(gff) = parse_gff(&bytes, &source) else {
            continue;
        };
        let mut references = Vec::new();
        visit_struct(
            &gff.root,
            "root",
            &resource.key,
            &source,
            &known,
            &mut references,
        );
        for reference in references {
            if let Some(document) = index
                .documents
                .iter_mut()
                .find(|value| value.resref == reference.script)
                && !document.inbound_references.contains(&reference)
            {
                document.inbound_references.push(reference);
            }
        }
    }
    for document in &mut index.documents {
        document.inbound_references.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| a.field_path.cmp(&b.field_path))
        });
    }
}

fn visit_struct(
    root: &GenericStruct,
    path: &str,
    resource: &ResourceKey,
    source: &str,
    known: &BTreeSet<String>,
    output: &mut Vec<InboundScriptReference>,
) {
    for field in &root.fields {
        let field_path = format!("{path}.{}", field.label);
        match &field.value {
            GenericValue::String(value) | GenericValue::ResRef(value)
                if is_script_field(&field.label) =>
            {
                let script = value
                    .trim()
                    .trim_end_matches(".nss")
                    .trim_end_matches(".ncs")
                    .to_ascii_lowercase();
                if known.contains(&script) {
                    output.push(InboundScriptReference {
                        script,
                        resource: resource.clone(),
                        field_path,
                        source: source.into(),
                    });
                }
            }
            GenericValue::Struct(value) => {
                visit_struct(value, &field_path, resource, source, known, output)
            }
            GenericValue::List(values) => {
                for (index, value) in values.iter().enumerate() {
                    visit_struct(
                        value,
                        &format!("{field_path}[{index}]"),
                        resource,
                        source,
                        known,
                        output,
                    );
                }
            }
            _ => {}
        }
    }
}

fn is_script_field(label: &str) -> bool {
    let label = label.to_ascii_lowercase();
    label.contains("script")
        || label.starts_with("on")
        || matches!(
            label.as_str(),
            "active" | "conditional" | "condition" | "action" | "startingconditional"
        )
}

fn is_script_bearing_gff(resource_type: u16) -> bool {
    matches!(
        resource_extension(resource_type),
        Some(
            "ifo"
                | "are"
                | "git"
                | "dlg"
                | "utc"
                | "utd"
                | "ute"
                | "uti"
                | "utp"
                | "uts"
                | "utt"
                | "utm"
                | "utw"
                | "jrl"
        )
    )
}

fn diagnostic(key: &ResourceKey, error: &aurora_core::AppError) -> ScriptDiagnostic {
    ScriptDiagnostic {
        code: error.code.clone(),
        message: error.technical_message.clone(),
        line: None,
        resource: key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_event_and_script_fields_without_guessing_plain_strings() {
        assert!(is_script_field("OnHeartbeat"));
        assert!(is_script_field("Script"));
        assert!(!is_script_field("Tag"));
    }
}
