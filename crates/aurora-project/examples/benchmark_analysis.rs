use aurora_project::{DependencyRoots, analyze_module_file_with_cache};
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkResult {
    run: usize,
    elapsed_ms: u128,
    cache_state: String,
    resources: usize,
    versions: usize,
    scripts: usize,
    dialogues: usize,
    areas: usize,
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 || args.len() > 5 {
        eprintln!(
            "usage: benchmark_analysis <module.mod> <game-root> <cache.json> [user-root] [runs]"
        );
        std::process::exit(2);
    }
    let module = PathBuf::from(&args[0]);
    let roots = DependencyRoots {
        game_install_path: Some(PathBuf::from(&args[1])),
        user_data_path: args.get(3).map(PathBuf::from),
    };
    let cache = Path::new(&args[2]);
    let runs = args
        .get(4)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2)
        .clamp(1, 10);

    for run in 1..=runs {
        let started = Instant::now();
        let analysis = analyze_module_file_with_cache(
            &module,
            &roots,
            Some(cache),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_or_else(|error| {
            eprintln!("{}: {}", error.code, error.technical_message);
            std::process::exit(1);
        });
        let result = BenchmarkResult {
            run,
            elapsed_ms: started.elapsed().as_millis(),
            cache_state: format!("{:?}", analysis.resource_catalog_cache.state).to_lowercase(),
            resources: analysis.resource_catalog_summary.resource_count,
            versions: analysis.resource_catalog_summary.version_count,
            scripts: analysis.script_index_summary.scripts,
            dialogues: analysis.dialogue_index_summary.dialogues,
            areas: analysis.world_summary.areas,
        };
        println!(
            "{}",
            serde_json::to_string(&result).expect("benchmark result serialization")
        );
    }
}
