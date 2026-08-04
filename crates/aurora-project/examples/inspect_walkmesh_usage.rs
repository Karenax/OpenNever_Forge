use aurora_project::{DependencyRoots, analyze_module_file_with_roots};
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WalkmeshUsage {
    area: String,
    object_id: String,
    kind: String,
    models: Vec<String>,
    walkmesh_available: bool,
}

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(module_path) = arguments.next() else {
        eprintln!("usage: inspect_walkmesh_usage <module.mod> <game-install> [user-data]");
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
    let roots = DependencyRoots {
        game_install_path: Some(PathBuf::from(game_install_path)),
        user_data_path,
    };
    let analysis = match analyze_module_file_with_roots(
        Path::new(&module_path),
        &roots,
        &AtomicBool::new(false),
        |_| {},
    ) {
        Ok(analysis) => analysis,
        Err(error) => {
            eprintln!("analysis failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let usages = analysis
        .world_index
        .scenes
        .iter()
        .flat_map(|scene| {
            scene
                .objects
                .iter()
                .filter(|object| matches!(object.kind.as_str(), "tile" | "door" | "placeable"))
                .map(|object| {
                    let mut models = object.model_resrefs.clone();
                    if let Some(model) = &object.model_resref
                        && !models.contains(model)
                    {
                        models.insert(0, model.clone());
                    }
                    WalkmeshUsage {
                        area: scene.area.clone(),
                        object_id: object.id.clone(),
                        kind: object.kind.clone(),
                        models,
                        walkmesh_available: object.walkmesh_available,
                    }
                })
        })
        .collect::<Vec<_>>();
    match serde_json::to_string_pretty(&usages) {
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
