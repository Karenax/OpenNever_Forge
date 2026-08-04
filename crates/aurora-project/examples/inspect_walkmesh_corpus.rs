use aurora_mdl::parse_mdl;
use aurora_project::{DependencyRoots, ResourceManager, analyze_module_file_with_roots};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(module_path) = arguments.next() else {
        eprintln!(
            "usage: inspect_walkmesh_corpus <module.mod> <game-install> [user-data] [limit] [wok|pwk|dwk] [resref]"
        );
        return ExitCode::from(2);
    };
    let Some(game_install_path) = arguments.next() else {
        eprintln!("game installation path is required");
        return ExitCode::from(2);
    };
    let user_data_path = arguments
        .next()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let limit = arguments
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(25)
        .min(250);
    let selected_type = arguments.next().and_then(|value| match value.as_str() {
        "wok" => Some(2016),
        "dwk" => Some(2052),
        "pwk" => Some(2053),
        _ => None,
    });
    let selected_resref = arguments.next().map(|value| value.to_ascii_lowercase());
    let roots = DependencyRoots {
        game_install_path: Some(PathBuf::from(game_install_path)),
        user_data_path,
    };
    let cancelled = AtomicBool::new(false);
    let analysis =
        match analyze_module_file_with_roots(Path::new(&module_path), &roots, &cancelled, |_| {}) {
            Ok(analysis) => analysis,
            Err(error) => {
                eprintln!("analysis failed: {error}");
                return ExitCode::FAILURE;
            }
        };
    let mut inspected = 0_usize;
    let mut attempted = 0_usize;
    for resource in analysis.resource_catalog.entries.iter().filter(|resource| {
        selected_type
            .map(|resource_type| resource.key.resource_type == resource_type)
            .unwrap_or_else(|| matches!(resource.key.resource_type, 2016 | 2052 | 2053))
            && selected_resref
                .as_ref()
                .map(|resref| resource.key.resref.eq_ignore_ascii_case(resref))
                .unwrap_or(true)
    }) {
        if attempted >= limit {
            break;
        }
        attempted += 1;
        let bytes = match ResourceManager::read(&resource.selected, &cancelled) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{} READ_ERROR {}", resource.key, error.code);
                continue;
            }
        };
        match parse_mdl(&bytes) {
            Ok(model) => {
                let nodes = model
                    .nodes
                    .iter()
                    .map(|node| {
                        let faces = node
                            .mesh
                            .as_ref()
                            .map(|mesh| mesh.indices.len() / 3)
                            .unwrap_or_default();
                        format!("{}:{:?}:{faces}", node.name, node.kinds)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let surfaces = model
                    .nodes
                    .iter()
                    .filter_map(|node| node.mesh.as_ref())
                    .flat_map(|mesh| mesh.surface_ids.iter().copied())
                    .collect::<std::collections::BTreeSet<_>>();
                println!(
                    "{} {:?} bytes={} nodes=[{}] surfaces={:?} diagnostics={}",
                    resource.key,
                    model.format,
                    bytes.len(),
                    nodes,
                    surfaces,
                    model.diagnostics.len()
                );
                inspected += 1;
            }
            Err(error) => println!(
                "{} PARSE_ERROR {} prefix={} text={:?}",
                resource.key,
                error.code,
                bytes
                    .iter()
                    .take(48)
                    .map(|value| format!("{value:02X}"))
                    .collect::<Vec<_>>()
                    .join(""),
                String::from_utf8_lossy(&bytes[..bytes.len().min(2048)])
            ),
        }
    }
    println!("attempted={attempted} inspected={inspected} limit={limit}");
    ExitCode::SUCCESS
}
