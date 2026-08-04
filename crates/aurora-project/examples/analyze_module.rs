use aurora_project::{
    DependencyRoots, analyze_module_file_with_roots, build_asset_preview, cached_model_preview,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!(
            "usage: analyze_module <module.mod> [game-install] [user-data] [preview-resref|auto|auto-skin|auto-walkmesh|auto-reference|auto-plt|scene-auto]"
        );
        return ExitCode::from(2);
    };
    let roots = DependencyRoots {
        game_install_path: arguments.next().map(PathBuf::from),
        user_data_path: arguments.next().map(PathBuf::from),
    };
    let preview_resref = arguments.next();

    match analyze_module_file_with_roots(Path::new(&path), &roots, &AtomicBool::new(false), |_| {})
    {
        Ok(analysis) => {
            let name = analysis
                .module_info
                .name
                .primary_text()
                .unwrap_or("<no embedded name>");
            println!("Name: {name}");
            println!("Tag: {}", analysis.module_info.tag);
            println!(
                "Minimum NWN: {}",
                analysis
                    .module_info
                    .minimum_game_version
                    .as_deref()
                    .unwrap_or("<unspecified>")
            );
            println!("Entry area: {}", analysis.module_info.entry_area);
            println!("HAKs: {}", analysis.module_info.hak_files.len());
            println!(
                "Custom TLK: {}",
                analysis
                    .module_info
                    .custom_tlk
                    .as_deref()
                    .unwrap_or("<none>")
            );
            println!("Resources: {}", analysis.inventory.resource_count);
            println!(
                "Resolved catalog: {} resources, {} versions, {} shadowed, {} diagnostics",
                analysis.resource_catalog_summary.resource_count,
                analysis.resource_catalog_summary.version_count,
                analysis.resource_catalog_summary.shadowed_count,
                analysis.resource_catalog_summary.diagnostic_count,
            );
            println!(
                "Structured: {}/{} GFF, {} failed, {} 2DA, {} TLK, {} areas, {} blueprints",
                analysis.structured_summary.gff.parsed,
                analysis.structured_summary.gff.discovered,
                analysis.structured_summary.gff.failed,
                analysis.structured_summary.two_da_tables.len(),
                analysis.structured_summary.talk_tables.len(),
                analysis.structured_summary.areas.len(),
                analysis.structured_summary.blueprints.len(),
            );
            println!(
                "Scripts: {} logical, {} NSS, {} NCS, {} missing source, {} symbols, {} inbound references, {} diagnostics",
                analysis.script_index_summary.scripts,
                analysis.script_index_summary.nss,
                analysis.script_index_summary.ncs,
                analysis.script_index_summary.missing_source,
                analysis.script_index_summary.symbols,
                analysis.script_index_summary.inbound_references,
                analysis.script_index_summary.diagnostics,
            );
            println!(
                "Dialogues: {} graphs, {} nodes, {} links, {} shared, {} cycles, {} unreachable, {} broken, {} references, {} diagnostics",
                analysis.dialogue_index_summary.dialogues,
                analysis.dialogue_index_summary.nodes,
                analysis.dialogue_index_summary.links,
                analysis.dialogue_index_summary.shared_nodes,
                analysis.dialogue_index_summary.cycles,
                analysis.dialogue_index_summary.unreachable_nodes,
                analysis.dialogue_index_summary.broken_links,
                analysis.dialogue_index_summary.references,
                analysis.dialogue_index_summary.diagnostics,
            );
            println!(
                "World: {} quests / {} entries, {} factions, {} areas / {} tiles / {} instances, {} assets / {} previewable, {} scene objects, {} graph nodes / {} edges, {} diagnostics",
                analysis.world_summary.journal_categories,
                analysis.world_summary.journal_entries,
                analysis.world_summary.factions,
                analysis.world_summary.areas,
                analysis.world_summary.tiles,
                analysis.world_summary.instances,
                analysis.world_summary.assets,
                analysis.world_summary.previewable_assets,
                analysis.world_summary.scene_objects,
                analysis.world_summary.graph_nodes,
                analysis.world_summary.graph_edges,
                analysis.world_summary.diagnostics,
            );
            for scene in &analysis.world_index.scenes {
                println!(
                    "Scene {}: {} resolved / {} objects, {} unique models, {} walkmeshes, {} degraded markers, {} diagnostics",
                    scene.area,
                    scene.resolved_assets,
                    scene.objects.len() + scene.overlays.len(),
                    scene.unique_models,
                    scene.walkmesh_assets,
                    scene.missing_assets,
                    scene.diagnostics.len(),
                );
                let mut kinds = BTreeMap::<&str, (usize, usize)>::new();
                for object in scene.objects.iter().chain(&scene.overlays) {
                    let entry = kinds.entry(&object.kind).or_default();
                    entry.0 += 1;
                    entry.1 += usize::from(!object.marker);
                }
                println!(
                    "Scene kinds {}: {}",
                    scene.area,
                    kinds
                        .into_iter()
                        .map(|(kind, (total, resolved))| format!("{kind}={resolved}/{total}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                for diagnostic in &scene.diagnostics {
                    println!(
                        "Scene diagnostic {}: {} ({})",
                        diagnostic.code, diagnostic.message, diagnostic.resource
                    );
                }
            }
            let models = analysis
                .world_index
                .assets
                .assets
                .iter()
                .filter(|asset| asset.key.resource_type == 2002)
                .collect::<Vec<_>>();
            println!(
                "Models: {} inspected, {} GLB-ready, {} meshes, {} triangles, {} skins, {} walkmeshes, {} supermodels, {} references",
                models.len(),
                models.iter().filter(|asset| asset.glb_preview).count(),
                models.iter().map(|asset| asset.mesh_count).sum::<usize>(),
                models
                    .iter()
                    .map(|asset| asset.triangle_count)
                    .sum::<usize>(),
                models.iter().map(|asset| asset.skin_count).sum::<usize>(),
                models
                    .iter()
                    .map(|asset| asset.walkmesh_count)
                    .sum::<usize>(),
                models
                    .iter()
                    .filter(|asset| asset.supermodel.is_some())
                    .count(),
                models
                    .iter()
                    .map(|asset| asset.referenced_models.len())
                    .sum::<usize>(),
            );
            for asset in models.iter().filter(|asset| !asset.glb_preview).take(10) {
                println!(
                    "Model diagnostic {}: {}",
                    asset.key,
                    asset
                        .diagnostics
                        .first()
                        .map(|value| format!("{} / {}", value.code, value.message))
                        .unwrap_or_else(|| "no geometry".to_owned())
                );
            }
            for diagnostic in analysis.resource_catalog.diagnostics.iter().take(20) {
                println!(
                    "Resource diagnostic {}: {} ({})",
                    diagnostic.code, diagnostic.message, diagnostic.source
                );
            }
            for diagnostic in analysis.structured_summary.diagnostics.iter().take(20) {
                println!(
                    "Structured diagnostic {}: {} ({})",
                    diagnostic.code, diagnostic.message, diagnostic.resource
                );
            }
            if let Some(requested) = preview_resref.as_deref() {
                if requested.eq_ignore_ascii_case("scene-auto") {
                    let scene_models = analysis
                        .world_index
                        .scenes
                        .iter()
                        .flat_map(|scene| scene.objects.iter())
                        .filter(|object| !object.marker)
                        .flat_map(|object| object.model_resrefs.iter().cloned())
                        .collect::<BTreeSet<_>>();
                    let cache_root = Path::new(".tmp").join("model-cache");
                    let mut converted = 0;
                    let mut failed = Vec::new();
                    let mut byte_length = 0_usize;
                    for resref in &scene_models {
                        match cached_model_preview(
                            &analysis.resource_catalog,
                            resref,
                            &cache_root,
                            &AtomicBool::new(false),
                        ) {
                            Ok(result) => {
                                converted += 1;
                                byte_length += result.artifact.byte_length;
                            }
                            Err(error) => failed.push(format!("{resref}: {}", error.code)),
                        }
                    }
                    println!(
                        "Scene GLB corpus: {converted}/{} unique models, {} bytes, failures={}",
                        scene_models.len(),
                        byte_length,
                        failed.join(", ")
                    );
                }
                if requested.eq_ignore_ascii_case("auto-plt") {
                    if let Some(asset) = analysis
                        .resource_catalog
                        .entries
                        .iter()
                        .find(|asset| asset.key.resource_type == 6)
                    {
                        match build_asset_preview(
                            &analysis.resource_catalog,
                            &asset.key.resref,
                            6,
                            &AtomicBool::new(false),
                        ) {
                            Ok(preview) => println!(
                                "PLT preview {}: {} bytes PNG, {}x{}",
                                asset.key.resref,
                                preview.bytes.len(),
                                preview.width.unwrap_or_default(),
                                preview.height.unwrap_or_default(),
                            ),
                            Err(error) => println!("PLT preview failed: {error}"),
                        }
                    } else {
                        println!("PLT preview: no PLT in the bounded world sample");
                    }
                }
                let selected = match requested.to_ascii_lowercase().as_str() {
                    "auto" => models.iter().find(|asset| asset.glb_preview),
                    "auto-skin" => models
                        .iter()
                        .find(|asset| asset.glb_preview && asset.skin_count > 0),
                    "auto-walkmesh" => models
                        .iter()
                        .find(|asset| asset.glb_preview && asset.walkmesh_count > 0),
                    "auto-reference" => models
                        .iter()
                        .find(|asset| asset.glb_preview && !asset.referenced_models.is_empty()),
                    _ => None,
                }
                .map(|asset| asset.key.resref.as_str())
                .or_else(|| {
                    (!requested.to_ascii_lowercase().starts_with("auto")
                        && !requested.eq_ignore_ascii_case("scene-auto"))
                    .then_some(requested)
                });
                if let Some(selected) = selected {
                    let cache_root = Path::new(".tmp").join("model-cache");
                    match cached_model_preview(
                        &analysis.resource_catalog,
                        selected,
                        &cache_root,
                        &AtomicBool::new(false),
                    ) {
                        Ok(first) => {
                            let second = cached_model_preview(
                                &analysis.resource_catalog,
                                selected,
                                &cache_root,
                                &AtomicBool::new(false),
                            )
                            .expect("cached real-model preview");
                            println!(
                                "GLB preview {}: {} bytes, {} nodes, {} meshes, {} skins, {} animations, first-hit={}, second-hit={}, cache={}",
                                selected,
                                first.artifact.byte_length,
                                first.artifact.node_count,
                                first.artifact.mesh_count,
                                first.artifact.skin_count,
                                first.artifact.animation_count,
                                first.cache_hit,
                                second.cache_hit,
                                first.cache_path.display(),
                            );
                        }
                        Err(error) => println!("GLB preview {selected} failed: {error}"),
                    }
                } else if !requested.eq_ignore_ascii_case("auto-plt")
                    && !requested.eq_ignore_ascii_case("scene-auto")
                {
                    println!("GLB preview: no convertible model in the bounded world sample");
                }
            }
            println!(
                "Dependencies: {} resolved, {} missing, {} unchecked, {} invalid",
                analysis.dependency_report.resolved_count,
                analysis.dependency_report.missing_count,
                analysis.dependency_report.unchecked_count,
                analysis.dependency_report.invalid_count,
            );
            for dependency in &analysis.dependency_report.dependencies {
                let fingerprint = dependency
                    .fingerprint
                    .as_ref()
                    .map(|value| format!("{} / {} bytes", value.sha256, value.size_bytes))
                    .unwrap_or_else(|| "<no fingerprint>".to_owned());
                println!(
                    "- {:?} {}: {:?}, {fingerprint}",
                    dependency.kind, dependency.logical_name, dependency.change
                );
            }
            println!("SHA-256: {}", analysis.fingerprint.sha256);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
