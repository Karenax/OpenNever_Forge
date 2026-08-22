use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_dialogue::{
    DialogueDiagnostic, DialogueGraph, DialogueLink, DialogueNode, DialogueNodeKind,
    DialogueTreeNode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const DIALOGUE_EXPORT_SCHEMA_VERSION: &str = "opennever-dialogue-export@1.0.0";
pub const DIALOGUE_EXPORT_CLASSIFICATION: &str = "local_only_proprietary";
pub const DIALOGUE_EXPORT_REDISTRIBUTION: &str = "not_redistributable_without_separate_rights";
const MAX_DIALOGUE_EXPORT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIALOGUE_SOURCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DialogueExportRevision {
    Analysis,
    Workspace,
}

#[derive(Debug, Clone)]
pub struct DialogueExportSource {
    pub graph: DialogueGraph,
    pub resource_bytes: Vec<u8>,
    pub revision: DialogueExportRevision,
    pub protected_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportPreview {
    pub schema_version: String,
    pub resref: String,
    pub revision: DialogueExportRevision,
    pub ready: bool,
    pub suggested_directory_name: String,
    pub source_resource_sha256: String,
    pub node_count: usize,
    pub entry_count: usize,
    pub reply_count: usize,
    pub link_count: usize,
    pub root_count: usize,
    pub shared_node_count: usize,
    pub unreachable_node_count: usize,
    pub cycle_count: usize,
    pub broken_link_count: usize,
    pub diagnostic_count: usize,
    pub reference_count: usize,
    pub scripts: Vec<String>,
    pub transcript_preview: Vec<String>,
    pub warnings: Vec<String>,
    pub classification: String,
    pub redistribution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportReference {
    pub resource: ResourceKey,
    pub field_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportDocument {
    pub schema_version: String,
    pub resref: String,
    pub revision: DialogueExportRevision,
    pub source_resource_sha256: String,
    pub nodes: Vec<DialogueNode>,
    pub links: Vec<DialogueLink>,
    pub roots: Vec<String>,
    pub shared_nodes: Vec<String>,
    pub unreachable_nodes: Vec<String>,
    pub cycles: Vec<Vec<String>>,
    pub diagnostics: Vec<DialogueDiagnostic>,
    pub references: Vec<DialogueExportReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportFile {
    pub path: String,
    pub role: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportManifest {
    pub schema_version: String,
    pub generator: String,
    pub classification: String,
    pub redistribution: String,
    pub resref: String,
    pub revision: DialogueExportRevision,
    pub source_resource_sha256: String,
    pub node_count: usize,
    pub link_count: usize,
    pub root_count: usize,
    pub broken_link_count: usize,
    pub cycle_count: usize,
    pub scripts: Vec<String>,
    pub warnings: Vec<String>,
    pub files: Vec<DialogueExportFile>,
    pub source_nwn_immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogueExportResult {
    pub schema_version: String,
    pub destination: String,
    pub resref: String,
    pub revision: DialogueExportRevision,
    pub source_resource_sha256: String,
    pub node_count: usize,
    pub link_count: usize,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub warnings: Vec<String>,
    pub manifest: DialogueExportManifest,
}

pub fn preview_dialogue_export(source: &DialogueExportSource) -> AppResult<DialogueExportPreview> {
    validate_source(source)?;
    let resref = normalize_resref(&source.graph.key.resref)?;
    let scripts = collect_scripts(&source.graph);
    let warnings = collect_warnings(&source.graph);
    Ok(DialogueExportPreview {
        schema_version: DIALOGUE_EXPORT_SCHEMA_VERSION.to_owned(),
        resref: resref.clone(),
        revision: source.revision,
        ready: true,
        suggested_directory_name: format!("{resref}.dialogue-export-v1"),
        source_resource_sha256: sha256(&source.resource_bytes),
        node_count: source.graph.nodes.len(),
        entry_count: source
            .graph
            .nodes
            .iter()
            .filter(|node| node.kind == DialogueNodeKind::Entry)
            .count(),
        reply_count: source
            .graph
            .nodes
            .iter()
            .filter(|node| node.kind == DialogueNodeKind::Reply)
            .count(),
        link_count: source.graph.links.len(),
        root_count: source.graph.roots.len(),
        shared_node_count: source.graph.shared_nodes.len(),
        unreachable_node_count: source.graph.unreachable_nodes.len(),
        cycle_count: source.graph.cycles.len(),
        broken_link_count: source.graph.links.iter().filter(|link| link.broken).count(),
        diagnostic_count: source.graph.diagnostics.len(),
        reference_count: source.graph.references.len(),
        scripts,
        transcript_preview: transcript_lines(&source.graph)
            .into_iter()
            .take(6)
            .collect(),
        warnings,
        classification: DIALOGUE_EXPORT_CLASSIFICATION.to_owned(),
        redistribution: DIALOGUE_EXPORT_REDISTRIBUTION.to_owned(),
    })
}

pub fn export_dialogue(
    source: &DialogueExportSource,
    destination: &Path,
) -> AppResult<DialogueExportResult> {
    let preview = preview_dialogue_export(source)?;
    let destination = validate_dialogue_export_destination(destination, &source.protected_roots)?;
    let parent = destination
        .parent()
        .expect("validated dialogue destination has a parent");
    let staging = tempfile::Builder::new()
        .prefix(".opennever-dialogue-export-")
        .tempdir_in(parent)
        .map_err(|error| {
            Box::new(AppError::io(
                "create dialogue export staging directory",
                parent.display().to_string(),
                &error,
            ))
        })?;
    let mut total_size_bytes = 0_u64;
    let mut files = Vec::new();

    let dlg_path = format!("{}.dlg", preview.resref);
    let dlg_file = write_payload(
        staging.path(),
        &dlg_path,
        "aurora_dialogue",
        &source.resource_bytes,
    )?;
    reserve_export_bytes(&mut total_size_bytes, dlg_file.size_bytes, &dlg_path)?;
    files.push(dlg_file);

    let document = portable_document(source, &preview);
    let document_bytes = serialize_pretty(&document, "dialogue.json", &preview.resref)?;
    let document_file = write_payload(
        staging.path(),
        "dialogue.json",
        "portable_dialogue",
        &document_bytes,
    )?;
    reserve_export_bytes(
        &mut total_size_bytes,
        document_file.size_bytes,
        "dialogue.json",
    )?;
    files.push(document_file);

    let transcript = render_transcript(source, &preview);
    let transcript_file = write_payload(
        staging.path(),
        "transcript.md",
        "readable_transcript",
        transcript.as_bytes(),
    )?;
    reserve_export_bytes(
        &mut total_size_bytes,
        transcript_file.size_bytes,
        "transcript.md",
    )?;
    files.push(transcript_file);

    let manifest = DialogueExportManifest {
        schema_version: DIALOGUE_EXPORT_SCHEMA_VERSION.to_owned(),
        generator: "OpenNever Forge aurora-dialogue-export 0.1".to_owned(),
        classification: DIALOGUE_EXPORT_CLASSIFICATION.to_owned(),
        redistribution: DIALOGUE_EXPORT_REDISTRIBUTION.to_owned(),
        resref: preview.resref.clone(),
        revision: source.revision,
        source_resource_sha256: preview.source_resource_sha256.clone(),
        node_count: preview.node_count,
        link_count: preview.link_count,
        root_count: preview.root_count,
        broken_link_count: preview.broken_link_count,
        cycle_count: preview.cycle_count,
        scripts: preview.scripts.clone(),
        warnings: preview.warnings.clone(),
        files: files.clone(),
        source_nwn_immutable: true,
    };
    let manifest_bytes = serialize_pretty(&manifest, "manifest.json", &preview.resref)?;
    let manifest_file =
        write_payload(staging.path(), "manifest.json", "manifest", &manifest_bytes)?;
    reserve_export_bytes(
        &mut total_size_bytes,
        manifest_file.size_bytes,
        "manifest.json",
    )?;

    let staging_path = staging.keep();
    if let Err(error) = fs::rename(&staging_path, &destination) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(Box::new(AppError::io(
            "publish dialogue export",
            destination.display().to_string(),
            &error,
        )));
    }

    Ok(DialogueExportResult {
        schema_version: DIALOGUE_EXPORT_SCHEMA_VERSION.to_owned(),
        destination: destination.display().to_string(),
        resref: preview.resref,
        revision: source.revision,
        source_resource_sha256: preview.source_resource_sha256,
        node_count: preview.node_count,
        link_count: preview.link_count,
        file_count: files.len() + 1,
        total_size_bytes,
        warnings: preview.warnings,
        manifest,
    })
}

pub fn validate_dialogue_export_destination(
    destination: &Path,
    protected_roots: &[PathBuf],
) -> AppResult<PathBuf> {
    if !destination.is_absolute() {
        return Err(path_error(destination, "destination must be absolute"));
    }
    if destination.exists() {
        return Err(path_error(destination, "destination already exists"));
    }
    if !matches!(destination.file_name(), Some(name) if matches!(Path::new(name).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]))
    {
        return Err(path_error(destination, "destination name is invalid"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| path_error(destination, "destination has no parent"))?;
    if !parent.is_dir() {
        return Err(path_error(parent, "destination parent is not a directory"));
    }
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        Box::new(AppError::io(
            "inspect dialogue destination parent",
            parent.display().to_string(),
            &error,
        ))
    })?;
    if is_link_metadata(&metadata) || contains_link_component(parent) {
        return Err(path_error(
            parent,
            "destination parent must not traverse a symbolic link or junction",
        ));
    }
    let canonical_parent = parent.canonicalize().map_err(|error| {
        Box::new(AppError::io(
            "canonicalize dialogue destination parent",
            parent.display().to_string(),
            &error,
        ))
    })?;
    let normalized = canonical_parent.join(destination.file_name().expect("validated file name"));
    let candidate = normalized_path(&normalized);
    for protected in protected_roots {
        let Ok(protected) = protected.canonicalize() else {
            continue;
        };
        if is_same_or_descendant(&candidate, &normalized_path(&protected)) {
            return Err(path_error(
                &normalized,
                "destination is inside a protected NWN source root",
            ));
        }
    }
    Ok(normalized)
}

fn portable_document(
    source: &DialogueExportSource,
    preview: &DialogueExportPreview,
) -> DialogueExportDocument {
    DialogueExportDocument {
        schema_version: DIALOGUE_EXPORT_SCHEMA_VERSION.to_owned(),
        resref: preview.resref.clone(),
        revision: source.revision,
        source_resource_sha256: preview.source_resource_sha256.clone(),
        nodes: source.graph.nodes.clone(),
        links: source.graph.links.clone(),
        roots: source.graph.roots.clone(),
        shared_nodes: source.graph.shared_nodes.clone(),
        unreachable_nodes: source.graph.unreachable_nodes.clone(),
        cycles: source.graph.cycles.clone(),
        diagnostics: source.graph.diagnostics.clone(),
        references: source
            .graph
            .references
            .iter()
            .map(|reference| DialogueExportReference {
                resource: reference.resource.clone(),
                field_path: reference.field_path.clone(),
            })
            .collect(),
    }
}

fn render_transcript(source: &DialogueExportSource, preview: &DialogueExportPreview) -> String {
    let mut output = format!(
        "# Dialogue `{}`\n\n- Version exportée : `{}`\n- Nœuds : {}\n- Liens : {}\n- SHA-256 DLG : `{}`\n\n## Conversation\n\n",
        preview.resref,
        revision_label(source.revision),
        preview.node_count,
        preview.link_count,
        preview.source_resource_sha256,
    );
    let lines = transcript_lines(&source.graph);
    if lines.is_empty() {
        output.push_str("_Aucune ligne de dialogue résolue._\n");
    } else {
        for line in lines {
            output.push_str(&line);
            output.push('\n');
        }
    }
    if !preview.scripts.is_empty() {
        output.push_str("\n## Scripts référencés\n\n");
        for script in &preview.scripts {
            output.push_str(&format!("- `{}`\n", escape_markdown(script)));
        }
    }
    if !source.graph.references.is_empty() {
        output.push_str("\n## Ressources qui référencent ce dialogue\n\n");
        for reference in &source.graph.references {
            output.push_str(&format!(
                "- `{}` · `{}`\n",
                escape_markdown(&reference.resource.file_name()),
                escape_markdown(&reference.field_path),
            ));
        }
    }
    if !preview.warnings.is_empty() {
        output.push_str("\n## Diagnostics\n\n");
        for warning in &preview.warnings {
            output.push_str(&format!("- {}\n", escape_markdown(warning)));
        }
    }
    output
}

fn transcript_lines(graph: &DialogueGraph) -> Vec<String> {
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut lines = Vec::new();
    let mut stack = graph
        .tree
        .iter()
        .rev()
        .map(|node| (node, 0_usize))
        .collect::<Vec<_>>();
    while let Some((tree_node, depth)) = stack.pop() {
        lines.push(format_transcript_line(
            tree_node,
            nodes.get(tree_node.node_id.as_str()).copied(),
            depth,
        ));
        for child in tree_node.children.iter().rev() {
            stack.push((child, depth.saturating_add(1)));
        }
    }
    if lines.is_empty() {
        for node in &graph.nodes {
            let tree_node = DialogueTreeNode {
                node_id: node.id.clone(),
                kind: node.kind,
                display_text: node.display_text.clone(),
                repeated: false,
                cycle: false,
                children: Vec::new(),
            };
            lines.push(format_transcript_line(&tree_node, Some(node), 0));
        }
    }
    lines
}

fn format_transcript_line(
    tree_node: &DialogueTreeNode,
    node: Option<&DialogueNode>,
    depth: usize,
) -> String {
    let role = match tree_node.kind {
        DialogueNodeKind::Entry => node
            .and_then(|value| value.speaker.as_deref())
            .filter(|speaker| !speaker.trim().is_empty())
            .unwrap_or("PNJ"),
        DialogueNodeKind::Reply => "Joueur",
    };
    let text = tree_node
        .display_text
        .as_deref()
        .or_else(|| node.and_then(|value| value.display_text.as_deref()))
        .filter(|text| !text.trim().is_empty())
        .unwrap_or("[texte non résolu]");
    let marker = if tree_node.cycle {
        " · cycle"
    } else if tree_node.repeated {
        " · référence partagée"
    } else {
        ""
    };
    format!(
        "{}- **{}** : {} `{}`{}",
        "  ".repeat(depth.min(64)),
        escape_markdown(role),
        escape_markdown(text),
        escape_markdown(&tree_node.node_id),
        marker,
    )
}

fn collect_scripts(graph: &DialogueGraph) -> Vec<String> {
    let mut scripts = BTreeSet::new();
    for node in &graph.nodes {
        if let Some(script) = normalized_script(node.action_script.as_deref()) {
            scripts.insert(script);
        }
    }
    for link in &graph.links {
        if let Some(script) = normalized_script(link.condition_script.as_deref()) {
            scripts.insert(script);
        }
        if let Some(script) = normalized_script(link.action_script.as_deref()) {
            scripts.insert(script);
        }
    }
    scripts.into_iter().collect()
}

fn normalized_script(value: Option<&str>) -> Option<String> {
    let value = value?.trim().trim_end_matches(".nss").to_ascii_lowercase();
    (!value.is_empty()).then_some(value)
}

fn collect_warnings(graph: &DialogueGraph) -> Vec<String> {
    let mut warnings = graph
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect::<Vec<_>>();
    if !graph.cycles.is_empty() {
        warnings.push(format!("{} cycle(s) conservé(s)", graph.cycles.len()));
    }
    let broken = graph.links.iter().filter(|link| link.broken).count();
    if broken > 0 {
        warnings.push(format!("{broken} lien(s) cassé(s) conservé(s)"));
    }
    if !graph.unreachable_nodes.is_empty() {
        warnings.push(format!(
            "{} nœud(s) inaccessible(s) conservé(s)",
            graph.unreachable_nodes.len()
        ));
    }
    warnings.sort();
    warnings.dedup();
    warnings
}

fn validate_source(source: &DialogueExportSource) -> AppResult<()> {
    if source.resource_bytes.len() > MAX_DIALOGUE_SOURCE_BYTES {
        return Err(size_error("source dialogue"));
    }
    if source.resource_bytes.is_empty() {
        return Err(Box::new(
            AppError::new(
                "DIALOGUE_EXPORT_SOURCE_EMPTY",
                "Le dialogue sélectionné est vide.",
                "dialogue export source has zero bytes",
                ErrorSeverity::Error,
            )
            .with_resource(source.graph.key.to_string())
            .with_import_stage("dialogue_export_validation"),
        ));
    }
    normalize_resref(&source.graph.key.resref)?;
    Ok(())
}

fn serialize_pretty<T: Serialize>(value: &T, label: &str, resref: &str) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|error| {
        Box::new(
            AppError::new(
                "DIALOGUE_EXPORT_SERIALIZE_FAILED",
                "Le dialogue exporté n'a pas pu être sérialisé.",
                format!("cannot serialize {label}: {error}"),
                ErrorSeverity::Error,
            )
            .with_resource(resref.to_owned())
            .with_import_stage("dialogue_export_serialization"),
        )
    })
}

fn write_payload(
    root: &Path,
    relative: &str,
    role: &str,
    bytes: &[u8],
) -> AppResult<DialogueExportFile> {
    let path = root.join(relative);
    let mut file = fs::File::create(&path).map_err(|error| {
        Box::new(AppError::io(
            "create dialogue export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    file.write_all(bytes).map_err(|error| {
        Box::new(AppError::io(
            "write dialogue export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    file.sync_all().map_err(|error| {
        Box::new(AppError::io(
            "flush dialogue export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    Ok(DialogueExportFile {
        path: relative.to_owned(),
        role: role.to_owned(),
        size_bytes: bytes.len() as u64,
        sha256: sha256(bytes),
    })
}

fn reserve_export_bytes(total: &mut u64, bytes: u64, label: &str) -> AppResult<()> {
    *total = total.checked_add(bytes).ok_or_else(|| size_error(label))?;
    if *total > MAX_DIALOGUE_EXPORT_BYTES {
        return Err(size_error(label));
    }
    Ok(())
}

fn normalize_resref(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(Box::new(
            AppError::new(
                "DIALOGUE_EXPORT_RESREF_INVALID",
                "Le ResRef du dialogue n'est pas valide.",
                format!("unsafe dialogue resref {value:?}"),
                ErrorSeverity::Warning,
            )
            .with_import_stage("dialogue_export_validation"),
        ));
    }
    Ok(value)
}

fn escape_markdown(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\r' => {}
            '\n' => escaped.push(' '),
            '\\' => escaped.push_str("\\\\"),
            '`' | '*' | '_' | '[' | ']' | '#' | '>' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn revision_label(revision: DialogueExportRevision) -> &'static str {
    match revision {
        DialogueExportRevision::Analysis => "analyse",
        DialogueExportRevision::Workspace => "workspace",
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

fn is_link_metadata(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn contains_link_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current)
            && is_link_metadata(&metadata)
        {
            return true;
        }
    }
    false
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    value
        .strip_prefix("//?/")
        .or_else(|| value.strip_prefix("//./"))
        .unwrap_or(&value)
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_same_or_descendant(candidate: &str, protected: &str) -> bool {
    candidate == protected
        || candidate
            .strip_prefix(protected)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn path_error(path: &Path, technical: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "DIALOGUE_EXPORT_DESTINATION_INVALID",
            "La destination de l'export de dialogue n'est pas valide.",
            format!("{}: {technical}", path.display()),
            ErrorSeverity::Warning,
        )
        .with_import_stage("dialogue_export_path_validation"),
    )
}

fn size_error(label: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "DIALOGUE_EXPORT_SIZE_LIMIT_EXCEEDED",
            "Le dialogue dépasse la taille maximale d'export.",
            format!("dialogue export exceeds its bounded size while writing {label}"),
            ErrorSeverity::Error,
        )
        .with_import_stage("dialogue_export_limits"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_dialogue::{
        DialogueLink, DialogueNode, DialogueNodeKind, DialogueReference, DialogueTreeNode,
        adapt_dialogue,
    };
    use aurora_gff::{GenericGff, GenericStruct};

    fn fixture_source(revision: DialogueExportRevision) -> DialogueExportSource {
        let raw = GenericGff {
            file_type: "DLG ".into(),
            file_version: "V3.2".into(),
            source: "C:\\private\\module.mod::fixture.dlg".into(),
            struct_count: 0,
            field_count: 0,
            root: GenericStruct {
                index: 0,
                struct_type: u32::MAX,
                fields: Vec::new(),
            },
        };
        let mut graph = adapt_dialogue(
            ResourceKey::new("fixture", 2029),
            "C:\\private\\module.mod::fixture.dlg".into(),
            raw,
        );
        graph.nodes = vec![
            DialogueNode {
                id: "entry:0".into(),
                kind: DialogueNodeKind::Entry,
                index: 0,
                text: None,
                display_text: Some("Bienvenue, voyageur.".into()),
                speaker: Some("Gardien".into()),
                comment: None,
                animation: None,
                animation_loop: None,
                sound: None,
                quest: None,
                action_script: Some("open_gate".into()),
            },
            DialogueNode {
                id: "reply:0".into(),
                kind: DialogueNodeKind::Reply,
                index: 0,
                text: None,
                display_text: Some("Je souhaite entrer.".into()),
                speaker: None,
                comment: None,
                animation: None,
                animation_loop: None,
                sound: None,
                quest: None,
                action_script: None,
            },
        ];
        graph.links = vec![DialogueLink {
            id: "root:0".into(),
            source: None,
            target: "entry:0".into(),
            condition_script: Some("can_enter".into()),
            action_script: None,
            comment: None,
            is_child: false,
            broken: false,
        }];
        graph.roots = vec!["entry:0".into()];
        graph.tree = vec![DialogueTreeNode {
            node_id: "entry:0".into(),
            kind: DialogueNodeKind::Entry,
            display_text: Some("Bienvenue, voyageur.".into()),
            repeated: false,
            cycle: false,
            children: vec![DialogueTreeNode {
                node_id: "reply:0".into(),
                kind: DialogueNodeKind::Reply,
                display_text: Some("Je souhaite entrer.".into()),
                repeated: false,
                cycle: false,
                children: Vec::new(),
            }],
        }];
        graph.references = vec![DialogueReference {
            resource: ResourceKey::new("guard", 2027),
            field_path: "Conversation".into(),
            source: "C:\\private\\module.mod::guard.utc".into(),
        }];
        DialogueExportSource {
            graph,
            resource_bytes: b"synthetic redistributable dlg fixture".to_vec(),
            revision,
            protected_roots: Vec::new(),
        }
    }

    #[test]
    fn exports_exact_dlg_portable_json_transcript_and_manifest() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("fixture.dialogue-export-v1");
        let source = fixture_source(DialogueExportRevision::Workspace);
        let result = export_dialogue(&source, &destination).unwrap();

        assert_eq!(
            fs::read(destination.join("fixture.dlg")).unwrap(),
            source.resource_bytes
        );
        let json = fs::read_to_string(destination.join("dialogue.json")).unwrap();
        assert!(json.contains("Bienvenue, voyageur."));
        assert!(!json.contains("C:\\\\private"));
        let transcript = fs::read_to_string(destination.join("transcript.md")).unwrap();
        assert!(transcript.contains("**Gardien**"));
        assert!(transcript.contains("Je souhaite entrer."));
        assert_eq!(result.revision, DialogueExportRevision::Workspace);
        assert_eq!(result.file_count, 4);
        assert_eq!(result.manifest.files.len(), 3);
        assert!(result.manifest.source_nwn_immutable);
    }

    #[test]
    fn preview_reports_structure_scripts_and_revision() {
        let source = fixture_source(DialogueExportRevision::Analysis);
        let preview = preview_dialogue_export(&source).unwrap();
        assert_eq!(preview.node_count, 2);
        assert_eq!(preview.entry_count, 1);
        assert_eq!(preview.reply_count, 1);
        assert_eq!(preview.scripts, vec!["can_enter", "open_gate"]);
        assert_eq!(preview.revision, DialogueExportRevision::Analysis);
        assert!(preview.transcript_preview[0].contains("Gardien"));
    }

    #[test]
    fn protected_source_root_rejects_destination() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("forbidden.dialogue-export-v1");
        let error =
            validate_dialogue_export_destination(&destination, &[root.path().to_path_buf()])
                .unwrap_err();
        assert_eq!(error.code, "DIALOGUE_EXPORT_DESTINATION_INVALID");
    }
}
