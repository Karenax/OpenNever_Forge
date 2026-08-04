use crate::{
    DependencyRoots, ModuleDependencyKind, ModuleDependencyReport, ResourceCatalog, ResourceManager,
};
use aurora_core::{ResourceKey, resource_extension};
use aurora_dialogue::{DialogueIndex, DialogueIndexDiagnostic, DialogueReference, adapt_dialogue};
use aurora_gff::{GenericStruct, GenericValue, LocalizedString, parse_gff};
use aurora_tlk::{
    EmbeddedString, Gender, LocalizedStringRequest, LocalizedStringResolver, TalkTable,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn analyze_dialogues(
    catalog: &ResourceCatalog,
    dependencies: &ModuleDependencyReport,
    roots: &DependencyRoots,
    cancelled: &AtomicBool,
) -> DialogueIndex {
    let mut index = DialogueIndex::default();
    let tables = DialogueTables::load(dependencies, roots);
    for resource in catalog
        .entries
        .iter()
        .filter(|value| value.key.resource_type == 2029)
    {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let source = format!("{}::{}", resource.selected.source_path, resource.key);
        match ResourceManager::read(&resource.selected, cancelled)
            .and_then(|bytes| parse_gff(&bytes, &source))
        {
            Ok(raw) => {
                let mut graph = adapt_dialogue(resource.key.clone(), source, raw);
                for node in &mut graph.nodes {
                    if let Some(text) = &node.text {
                        node.display_text = tables.resolve(text);
                    }
                }
                index.dialogues.push(graph);
            }
            Err(error) => index.diagnostics.push(DialogueIndexDiagnostic {
                code: error.code.clone(),
                resource: resource.key.to_string(),
                source: resource.selected.source_path.clone(),
                message: error.technical_message.clone(),
            }),
        }
    }
    collect_references(catalog, &mut index, cancelled);
    index.finalize();
    index
}

pub(crate) struct DialogueTables {
    dialog: Option<TalkTable>,
    custom: Option<TalkTable>,
}
impl DialogueTables {
    pub(crate) fn load(dependencies: &ModuleDependencyReport, roots: &DependencyRoots) -> Self {
        let dialog = find_dialog_tlk(roots).and_then(|path| TalkTable::from_file(&path).ok());
        let custom = dependencies
            .dependencies
            .iter()
            .find(|value| value.kind == ModuleDependencyKind::CustomTlk)
            .and_then(|value| value.selected_path.as_deref())
            .and_then(|path| TalkTable::from_file(PathBuf::from(path).as_path()).ok());
        Self { dialog, custom }
    }
    fn resolve(&self, text: &LocalizedString) -> Option<String> {
        LocalizedStringResolver {
            dialog: self.dialog.as_ref(),
            dialog_female: None,
            custom: self.custom.as_ref(),
            custom_female: None,
        }
        .resolve(&LocalizedStringRequest {
            string_ref: text.string_ref,
            embedded: text
                .values
                .iter()
                .map(|value| EmbeddedString {
                    language_id: value.language_id,
                    text: value.text.clone(),
                })
                .collect(),
            language_id: 0,
            gender: Gender::Male,
        })
        .text
    }

    pub(crate) fn resolve_parts(
        &self,
        string_ref: Option<u32>,
        embedded_text: Option<&str>,
    ) -> Option<String> {
        LocalizedStringResolver {
            dialog: self.dialog.as_ref(),
            dialog_female: None,
            custom: self.custom.as_ref(),
            custom_female: None,
        }
        .resolve(&LocalizedStringRequest {
            string_ref,
            embedded: embedded_text
                .map(|text| {
                    vec![EmbeddedString {
                        language_id: 0,
                        text: text.to_owned(),
                    }]
                })
                .unwrap_or_default(),
            language_id: 0,
            gender: Gender::Male,
        })
        .text
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

fn collect_references(
    catalog: &ResourceCatalog,
    index: &mut DialogueIndex,
    cancelled: &AtomicBool,
) {
    let known = index
        .dialogues
        .iter()
        .map(|value| value.key.resref.clone())
        .collect::<BTreeSet<_>>();
    for resource in &catalog.entries {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        if !is_dialogue_bearing_gff(resource.key.resource_type) {
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
        visit(
            &gff.root,
            "root",
            &resource.key,
            &source,
            &known,
            &mut references,
        );
        for (dialogue_resref, reference) in references {
            if let Some(dialogue) = index
                .dialogues
                .iter_mut()
                .find(|value| value.key.resref == dialogue_resref)
            {
                dialogue.references.push(reference);
            }
        }
    }
    for dialogue in &mut index.dialogues {
        dialogue.references.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| a.field_path.cmp(&b.field_path))
        });
        dialogue.references.dedup();
    }
}

fn visit(
    root: &GenericStruct,
    path: &str,
    resource: &ResourceKey,
    source: &str,
    known: &BTreeSet<String>,
    output: &mut Vec<(String, DialogueReference)>,
) {
    for field in &root.fields {
        let field_path = format!("{path}.{}", field.label);
        match &field.value {
            GenericValue::String(value) | GenericValue::ResRef(value)
                if is_dialogue_field(&field.label) =>
            {
                let dialogue = value.trim().trim_end_matches(".dlg").to_ascii_lowercase();
                if known.contains(&dialogue) {
                    output.push((
                        dialogue,
                        DialogueReference {
                            resource: resource.clone(),
                            field_path,
                            source: source.into(),
                        },
                    ));
                }
            }
            GenericValue::Struct(value) => {
                visit(value, &field_path, resource, source, known, output)
            }
            GenericValue::List(values) => {
                for (index, value) in values.iter().enumerate() {
                    visit(
                        value,
                        &format!("{field_path}[{index}]"),
                        resource,
                        source,
                        known,
                        output,
                    )
                }
            }
            _ => {}
        }
    }
}
fn is_dialogue_field(label: &str) -> bool {
    matches!(
        label.to_ascii_lowercase().as_str(),
        "conversation" | "dialogue" | "dialog" | "dlg"
    )
}
fn is_dialogue_bearing_gff(resource_type: u16) -> bool {
    matches!(
        resource_extension(resource_type),
        Some("ifo" | "are" | "git" | "utc" | "utd" | "ute" | "utp" | "utt" | "utm" | "utw")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_conversation_fields_only() {
        assert!(is_dialogue_field("Conversation"));
        assert!(!is_dialogue_field("Description"));
    }
}
