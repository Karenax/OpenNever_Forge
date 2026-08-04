use aurora_core::ResourceKey;
use aurora_gff::{GenericField, GenericGff, GenericStruct, GenericValue, LocalizedString};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DialogueNodeKind {
    Entry,
    Reply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueNode {
    pub id: String,
    pub kind: DialogueNodeKind,
    pub index: usize,
    pub text: Option<LocalizedString>,
    pub display_text: Option<String>,
    pub speaker: Option<String>,
    pub comment: Option<String>,
    pub animation: Option<u32>,
    pub animation_loop: Option<bool>,
    pub sound: Option<String>,
    pub quest: Option<String>,
    pub action_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueLink {
    pub id: String,
    pub source: Option<String>,
    pub target: String,
    pub condition_script: Option<String>,
    pub action_script: Option<String>,
    pub comment: Option<String>,
    pub is_child: bool,
    pub broken: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueDiagnostic {
    pub code: String,
    pub message: String,
    pub node_id: Option<String>,
    pub link_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueIndexDiagnostic {
    pub code: String,
    pub resource: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueReference {
    pub resource: ResourceKey,
    pub field_path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueTreeNode {
    pub node_id: String,
    pub kind: DialogueNodeKind,
    pub display_text: Option<String>,
    pub repeated: bool,
    pub cycle: bool,
    pub children: Vec<DialogueTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueGraph {
    pub key: ResourceKey,
    pub source: String,
    pub nodes: Vec<DialogueNode>,
    pub links: Vec<DialogueLink>,
    pub roots: Vec<String>,
    pub shared_nodes: Vec<String>,
    pub unreachable_nodes: Vec<String>,
    pub cycles: Vec<Vec<String>>,
    pub diagnostics: Vec<DialogueDiagnostic>,
    pub references: Vec<DialogueReference>,
    pub tree: Vec<DialogueTreeNode>,
    pub raw: GenericGff,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueIndexSummary {
    pub dialogues: usize,
    pub nodes: usize,
    pub links: usize,
    pub shared_nodes: usize,
    pub cycles: usize,
    pub unreachable_nodes: usize,
    pub broken_links: usize,
    pub script_links: usize,
    pub references: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueIndex {
    pub dialogues: Vec<DialogueGraph>,
    pub diagnostics: Vec<DialogueIndexDiagnostic>,
    pub summary: DialogueIndexSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueSearchHit {
    pub resref: String,
    pub node_count: usize,
    pub link_count: usize,
    pub cycle_count: usize,
    pub diagnostic_count: usize,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialoguePage {
    pub items: Vec<DialogueSearchHit>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

impl DialogueIndex {
    pub fn finalize(&mut self) {
        self.dialogues.sort_by(|a, b| a.key.cmp(&b.key));
        self.summary = DialogueIndexSummary {
            dialogues: self.dialogues.len(),
            nodes: self.dialogues.iter().map(|v| v.nodes.len()).sum(),
            links: self.dialogues.iter().map(|v| v.links.len()).sum(),
            shared_nodes: self.dialogues.iter().map(|v| v.shared_nodes.len()).sum(),
            cycles: self.dialogues.iter().map(|v| v.cycles.len()).sum(),
            unreachable_nodes: self
                .dialogues
                .iter()
                .map(|v| v.unreachable_nodes.len())
                .sum(),
            broken_links: self
                .dialogues
                .iter()
                .flat_map(|v| &v.links)
                .filter(|v| v.broken)
                .count(),
            script_links: self
                .dialogues
                .iter()
                .flat_map(|v| &v.nodes)
                .filter(|v| v.action_script.is_some())
                .count()
                + self
                    .dialogues
                    .iter()
                    .flat_map(|v| &v.links)
                    .filter(|v| v.condition_script.is_some() || v.action_script.is_some())
                    .count(),
            references: self.dialogues.iter().map(|v| v.references.len()).sum(),
            diagnostics: self.diagnostics.len()
                + self
                    .dialogues
                    .iter()
                    .map(|v| v.diagnostics.len())
                    .sum::<usize>(),
        };
    }
    pub fn get(&self, resref: &str) -> Option<&DialogueGraph> {
        self.dialogues
            .iter()
            .find(|v| v.key.resref.eq_ignore_ascii_case(resref))
    }
    pub fn search(&self, query: &str, offset: usize, limit: usize) -> DialoguePage {
        let query = query.trim().to_ascii_lowercase();
        let mut hits = self
            .dialogues
            .iter()
            .filter(|dialogue| {
                query.is_empty()
                    || dialogue.key.resref.contains(&query)
                    || dialogue.nodes.iter().any(|node| {
                        node.display_text
                            .as_ref()
                            .is_some_and(|text| text.to_ascii_lowercase().contains(&query))
                            || node
                                .speaker
                                .as_ref()
                                .is_some_and(|text| text.to_ascii_lowercase().contains(&query))
                    })
                    || dialogue
                        .references
                        .iter()
                        .any(|reference| reference.resource.resref.contains(&query))
            })
            .map(|dialogue| DialogueSearchHit {
                resref: dialogue.key.resref.clone(),
                node_count: dialogue.nodes.len(),
                link_count: dialogue.links.len(),
                cycle_count: dialogue.cycles.len(),
                diagnostic_count: dialogue.diagnostics.len(),
                preview: dialogue
                    .nodes
                    .iter()
                    .find_map(|node| node.display_text.clone()),
            })
            .collect::<Vec<_>>();
        hits.sort_by(|a, b| a.resref.cmp(&b.resref));
        let total = hits.len();
        let limit = limit.clamp(1, 100);
        DialoguePage {
            items: hits.into_iter().skip(offset).take(limit).collect(),
            offset,
            limit,
            total,
        }
    }
}

pub fn adapt_dialogue(key: ResourceKey, source: String, raw: GenericGff) -> DialogueGraph {
    let entries = struct_list(&raw.root, &["EntryList", "EntriesList"]);
    let replies = struct_list(&raw.root, &["ReplyList", "RepliesList"]);
    let mut nodes = Vec::new();
    for (index, node) in entries.iter().enumerate() {
        nodes.push(adapt_node(node, DialogueNodeKind::Entry, index));
    }
    for (index, node) in replies.iter().enumerate() {
        nodes.push(adapt_node(node, DialogueNodeKind::Reply, index));
    }
    let known = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let mut links = Vec::new();
    for (index, link) in struct_list(&raw.root, &["StartingList"]).iter().enumerate() {
        links.push(adapt_link(
            link,
            None,
            DialogueNodeKind::Entry,
            index,
            &known,
        ));
    }
    for (node_index, node) in entries.iter().enumerate() {
        let source_id = node_id(DialogueNodeKind::Entry, node_index);
        for (index, link) in struct_list(node, &["RepliesList", "ReplyList"])
            .iter()
            .enumerate()
        {
            links.push(adapt_link(
                link,
                Some(source_id.clone()),
                DialogueNodeKind::Reply,
                index,
                &known,
            ));
        }
    }
    for (node_index, node) in replies.iter().enumerate() {
        let source_id = node_id(DialogueNodeKind::Reply, node_index);
        for (index, link) in struct_list(node, &["EntriesList", "EntryList"])
            .iter()
            .enumerate()
        {
            links.push(adapt_link(
                link,
                Some(source_id.clone()),
                DialogueNodeKind::Entry,
                index,
                &known,
            ));
        }
    }
    let roots = links
        .iter()
        .filter(|link| link.source.is_none() && !link.broken)
        .map(|link| link.target.clone())
        .collect::<Vec<_>>();
    let mut diagnostics = links
        .iter()
        .filter(|link| link.broken)
        .map(|link| DialogueDiagnostic {
            code: "DLG_LINK_BROKEN".into(),
            message: format!("La cible {} n'existe pas.", link.target),
            node_id: link.source.clone(),
            link_id: Some(link.id.clone()),
        })
        .collect::<Vec<_>>();
    let inbound = inbound_counts(&links);
    let shared_nodes = inbound
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let cycles = detect_cycles(&nodes, &links);
    for cycle in &cycles {
        diagnostics.push(DialogueDiagnostic {
            code: "DLG_CYCLE_DETECTED".into(),
            message: format!("Cycle détecté : {}", cycle.join(" → ")),
            node_id: cycle.first().cloned(),
            link_id: None,
        });
    }
    let reachable = reachable_nodes(&roots, &links);
    let unreachable_nodes = nodes
        .iter()
        .filter(|node| !reachable.contains(&node.id))
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    for id in &unreachable_nodes {
        diagnostics.push(DialogueDiagnostic {
            code: "DLG_NODE_UNREACHABLE".into(),
            message: "Nœud inaccessible depuis StartingList.".into(),
            node_id: Some(id.clone()),
            link_id: None,
        });
    }
    let tree = build_tree(&roots, &nodes, &links);
    DialogueGraph {
        key,
        source,
        nodes,
        links,
        roots,
        shared_nodes,
        unreachable_nodes,
        cycles,
        diagnostics,
        references: Vec::new(),
        tree,
        raw,
    }
}

fn adapt_node(value: &GenericStruct, kind: DialogueNodeKind, index: usize) -> DialogueNode {
    let text = locstring(value, &["Text"]);
    let display_text = text.as_ref().and_then(primary_text);
    DialogueNode {
        id: node_id(kind, index),
        kind,
        index,
        text,
        display_text,
        speaker: string(value, &["Speaker"]),
        comment: string(value, &["Comment"]),
        animation: unsigned(value, &["Animation"]),
        animation_loop: boolean(value, &["AnimLoop"]),
        sound: string(value, &["Sound"]),
        quest: string(value, &["Quest"]),
        action_script: string(value, &["Script", "ActionScript"]).map(normalize_resref),
    }
}

fn adapt_link(
    value: &GenericStruct,
    source: Option<String>,
    target_kind: DialogueNodeKind,
    position: usize,
    known: &BTreeSet<String>,
) -> DialogueLink {
    let index = unsigned(value, &["Index"])
        .map(|v| v as usize)
        .unwrap_or(usize::MAX);
    let target = node_id(target_kind, index);
    let id = format!(
        "{}:{}:{position}",
        source.as_deref().unwrap_or("start"),
        target
    );
    DialogueLink {
        id,
        source,
        target: target.clone(),
        condition_script: string(value, &["Active", "Conditional"]).map(normalize_resref),
        action_script: string(value, &["Script", "ActionScript"]).map(normalize_resref),
        comment: string(value, &["LinkComment", "Comment"]),
        is_child: boolean(value, &["IsChild"]).unwrap_or(false),
        broken: !known.contains(&target),
    }
}

fn build_tree(
    roots: &[String],
    nodes: &[DialogueNode],
    links: &[DialogueLink],
) -> Vec<DialogueTreeNode> {
    let map = nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    roots
        .iter()
        .filter_map(|root| expand_tree(root, &map, links, &mut seen, &mut Vec::new(), 0))
        .collect()
}
fn expand_tree(
    id: &str,
    nodes: &BTreeMap<String, &DialogueNode>,
    links: &[DialogueLink],
    seen: &mut BTreeSet<String>,
    path: &mut Vec<String>,
    depth: usize,
) -> Option<DialogueTreeNode> {
    let node = *nodes.get(id)?;
    let cycle = path.iter().any(|value| value == id);
    let repeated = !cycle && !seen.insert(id.to_owned());
    if cycle || repeated || depth >= 256 {
        return Some(DialogueTreeNode {
            node_id: id.into(),
            kind: node.kind,
            display_text: node.display_text.clone(),
            repeated,
            cycle: cycle || depth >= 256,
            children: Vec::new(),
        });
    }
    path.push(id.into());
    let children = links
        .iter()
        .filter(|link| link.source.as_deref() == Some(id) && !link.broken)
        .filter_map(|link| expand_tree(&link.target, nodes, links, seen, path, depth + 1))
        .collect();
    path.pop();
    Some(DialogueTreeNode {
        node_id: id.into(),
        kind: node.kind,
        display_text: node.display_text.clone(),
        repeated: false,
        cycle: false,
        children,
    })
}

fn detect_cycles(nodes: &[DialogueNode], links: &[DialogueLink]) -> Vec<Vec<String>> {
    fn visit(
        id: &str,
        links: &[DialogueLink],
        colors: &mut BTreeMap<String, u8>,
        stack: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        colors.insert(id.into(), 1);
        stack.push(id.into());
        for target in links
            .iter()
            .filter(|link| link.source.as_deref() == Some(id) && !link.broken)
            .map(|link| &link.target)
        {
            match colors.get(target).copied().unwrap_or(0) {
                0 => visit(target, links, colors, stack, cycles),
                1 => {
                    if let Some(start) = stack.iter().position(|value| value == target) {
                        let mut cycle = stack[start..].to_vec();
                        cycle.push(target.clone());
                        if !cycles.contains(&cycle) {
                            cycles.push(cycle);
                        }
                    }
                }
                _ => {}
            }
        }
        stack.pop();
        colors.insert(id.into(), 2);
    }
    let mut colors = BTreeMap::new();
    let mut cycles = Vec::new();
    for node in nodes {
        if !colors.contains_key(&node.id) {
            visit(&node.id, links, &mut colors, &mut Vec::new(), &mut cycles);
        }
    }
    cycles
}
fn reachable_nodes(roots: &[String], links: &[DialogueLink]) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let mut queue = VecDeque::from(roots.to_vec());
    while let Some(id) = queue.pop_front() {
        if !result.insert(id.clone()) {
            continue;
        }
        for target in links
            .iter()
            .filter(|link| link.source.as_deref() == Some(&id) && !link.broken)
            .map(|link| link.target.clone())
        {
            queue.push_back(target);
        }
    }
    result
}
fn inbound_counts(links: &[DialogueLink]) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for link in links.iter().filter(|v| !v.broken) {
        *result.entry(link.target.clone()).or_default() += 1;
    }
    result
}
fn node_id(kind: DialogueNodeKind, index: usize) -> String {
    format!(
        "{}:{index}",
        match kind {
            DialogueNodeKind::Entry => "entry",
            DialogueNodeKind::Reply => "reply",
        }
    )
}
fn normalize_resref(value: String) -> String {
    value
        .trim()
        .trim_end_matches(".nss")
        .trim_end_matches(".ncs")
        .to_ascii_lowercase()
}
fn primary_text(value: &LocalizedString) -> Option<String> {
    value
        .values
        .iter()
        .find(|v| v.language_id == 0)
        .or_else(|| value.values.first())
        .map(|v| v.text.clone())
        .or_else(|| value.string_ref.map(|v| format!("StrRef #{v}")))
}
fn field<'a>(root: &'a GenericStruct, names: &[&str]) -> Option<&'a GenericField> {
    root.fields.iter().find(|field| {
        names
            .iter()
            .any(|name| field.label.eq_ignore_ascii_case(name))
    })
}
fn struct_list<'a>(root: &'a GenericStruct, names: &[&str]) -> &'a [GenericStruct] {
    match field(root, names).map(|v| &v.value) {
        Some(GenericValue::List(value)) => value,
        _ => &[],
    }
}
fn string(root: &GenericStruct, names: &[&str]) -> Option<String> {
    match &field(root, names)?.value {
        GenericValue::String(v) | GenericValue::ResRef(v) if !v.trim().is_empty() => {
            Some(v.clone())
        }
        _ => None,
    }
}
fn locstring(root: &GenericStruct, names: &[&str]) -> Option<LocalizedString> {
    match &field(root, names)?.value {
        GenericValue::LocalizedString(v) => Some(v.clone()),
        _ => None,
    }
}
fn unsigned(root: &GenericStruct, names: &[&str]) -> Option<u32> {
    match field(root, names)?.value {
        GenericValue::Byte(v) => Some(u32::from(v)),
        GenericValue::Word(v) => Some(u32::from(v)),
        GenericValue::Dword(v) => Some(v),
        _ => None,
    }
}
fn boolean(root: &GenericStruct, names: &[&str]) -> Option<bool> {
    unsigned(root, names).map(|v| v != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn list(label: &str, values: Vec<GenericStruct>) -> GenericField {
        GenericField {
            label: label.into(),
            field_type: 15,
            value: GenericValue::List(values),
        }
    }
    fn dword(label: &str, value: u32) -> GenericField {
        GenericField {
            label: label.into(),
            field_type: 4,
            value: GenericValue::Dword(value),
        }
    }
    fn node(index: u32, outgoing: &str, targets: &[u32]) -> GenericStruct {
        GenericStruct {
            index,
            struct_type: 0,
            fields: vec![list(
                outgoing,
                targets
                    .iter()
                    .enumerate()
                    .map(|(i, target)| GenericStruct {
                        index: i as u32,
                        struct_type: 0,
                        fields: vec![dword("Index", *target)],
                    })
                    .collect(),
            )],
        }
    }
    #[test]
    fn detects_cycles_shared_unreachable_and_broken_links_without_looping_tree() {
        let raw = GenericGff {
            file_type: "DLG ".into(),
            file_version: "V3.2".into(),
            source: "fixture".into(),
            struct_count: 0,
            field_count: 0,
            root: GenericStruct {
                index: 0,
                struct_type: u32::MAX,
                fields: vec![
                    list(
                        "EntryList",
                        vec![node(0, "RepliesList", &[0, 1]), node(1, "RepliesList", &[])],
                    ),
                    list(
                        "ReplyList",
                        vec![
                            node(0, "EntriesList", &[0]),
                            node(1, "EntriesList", &[0, 99]),
                        ],
                    ),
                    list(
                        "StartingList",
                        vec![GenericStruct {
                            index: 0,
                            struct_type: 0,
                            fields: vec![dword("Index", 0)],
                        }],
                    ),
                ],
            },
        };
        let graph = adapt_dialogue(ResourceKey::new("fixture", 2029), "fixture".into(), raw);
        assert!(!graph.cycles.is_empty());
        assert!(graph.shared_nodes.contains(&"entry:0".into()));
        assert!(graph.links.iter().any(|v| v.broken));
        assert!(graph.unreachable_nodes.contains(&"entry:1".into()));
        assert!(!graph.tree.is_empty());
    }
}
