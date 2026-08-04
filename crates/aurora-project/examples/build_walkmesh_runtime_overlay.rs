use aurora_core::ResourceKey;
use aurora_edit::{
    WalkmeshKind, WalkmeshOperation, apply_walkmesh_operation, inspect_walkmesh,
    serialize_walkmesh_ascii,
};
use aurora_project::{DependencyRoots, ResourceManager, analyze_module_file_with_roots};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlayEntry {
    resource: String,
    source_sha256: String,
    output_sha256: String,
    vertices: usize,
    faces: usize,
    variants: usize,
    hooks: usize,
}

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() < 4 {
        eprintln!(
            "usage: build_walkmesh_runtime_overlay <module.mod> <game-install> <user-data> <output-development> [wok-resref] [pwk-resref] [dwk-resref]"
        );
        return ExitCode::from(2);
    }
    let module_path = PathBuf::from(&arguments[0]);
    let roots = DependencyRoots {
        game_install_path: Some(PathBuf::from(&arguments[1])),
        user_data_path: Some(PathBuf::from(&arguments[2])),
    };
    let output = PathBuf::from(&arguments[3]);
    let targets = [
        (
            WalkmeshKind::Wok,
            arguments
                .get(4)
                .map(String::as_str)
                .unwrap_or("tin01_o20_01"),
        ),
        (
            WalkmeshKind::Pwk,
            arguments.get(5).map(String::as_str).unwrap_or("plc_t06"),
        ),
        (
            WalkmeshKind::Dwk,
            arguments.get(6).map(String::as_str).unwrap_or("t_door01"),
        ),
    ];
    let cancelled = AtomicBool::new(false);
    let analysis = match analyze_module_file_with_roots(&module_path, &roots, &cancelled, |_| {}) {
        Ok(analysis) => analysis,
        Err(error) => {
            eprintln!("analysis failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = fs::create_dir_all(&output) {
        eprintln!("cannot create {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    let mut manifest = Vec::new();
    for (kind, resref) in targets {
        let key = ResourceKey::new(resref, kind.resource_type());
        let Some(resource) = analysis.resource_catalog.get(&key) else {
            eprintln!("resource not resolved: {key}");
            return ExitCode::FAILURE;
        };
        let bytes = match ResourceManager::read(&resource.selected, &cancelled) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("resource read failed for {key}: {error}");
                return ExitCode::FAILURE;
            }
        };
        let mut document = match inspect_walkmesh(resref, kind, &bytes) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("walkmesh import failed for {key}: {error}");
                return ExitCode::FAILURE;
            }
        };
        if !document.draft.faces.is_empty()
            && let Err(error) = apply_walkmesh_operation(
                &mut document.draft,
                &WalkmeshOperation::SplitFace { face_index: 0 },
            )
        {
            eprintln!("walkmesh split failed for {key}: {error}");
            return ExitCode::FAILURE;
        }
        let output_bytes = match serialize_walkmesh_ascii(resref, kind, &document.draft) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("walkmesh serialization failed for {key}: {error}");
                return ExitCode::FAILURE;
            }
        };
        let reopened = match inspect_walkmesh(resref, kind, &output_bytes) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("generated walkmesh reopen failed for {key}: {error}");
                return ExitCode::FAILURE;
            }
        };
        let extension = match kind {
            WalkmeshKind::Wok => "wok",
            WalkmeshKind::Pwk => "pwk",
            WalkmeshKind::Dwk => "dwk",
        };
        let path = output.join(format!("{resref}.{extension}"));
        if let Err(error) = fs::write(&path, &output_bytes) {
            eprintln!("cannot write {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
        manifest.push(OverlayEntry {
            resource: format!("{resref}.{extension}"),
            source_sha256: document.source_sha256,
            output_sha256: reopened.source_sha256,
            vertices: reopened.draft.vertices.len(),
            faces: reopened.draft.faces.len(),
            variants: reopened.draft.variants.len(),
            hooks: reopened.draft.hooks.len(),
        });
    }
    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("JSON failed: {error}");
            ExitCode::FAILURE
        }
    }
}
