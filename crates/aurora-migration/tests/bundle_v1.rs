use aurora_core::ResourceKey;
use aurora_migration::{
    AreaMigrationExportRequest, AreaMigrationSource, BundleManifest, audit_area_migration,
    export_area_migration, validate_bundle_destination, validate_bundle_directory,
};
use aurora_project::{
    AreaInstance, AreaMap, AreaTile, LocalizedText, ModuleDependency, ModuleDependencyChange,
    ModuleDependencyKind, ModuleDependencyReport, ModuleDependencyState, ModuleFingerprint,
    ResolvedResource, ResourceCatalog, ResourceLocation, ResourceSourceKind, ResourceVersion,
    SceneManifest, SceneObject, WorldIndex,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

struct Fixture {
    _root: tempfile::TempDir,
    _safe_root: tempfile::TempDir,
    source: AreaMigrationSource,
    source_files: Vec<PathBuf>,
}

#[test]
fn exports_a_deterministic_accounted_bundle_and_preserves_sources() {
    let fixture = synthetic_fixture(true);
    let before = snapshot(&fixture.source_files);
    let first = safe_destination(&fixture, "first.area-migration-v1");
    let second = safe_destination(&fixture, "second.area-migration-v1");
    let cancelled = AtomicBool::new(false);

    let first_result = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: first.clone(),
        },
        &cancelled,
        |_| {},
    )
    .expect("first export");
    let second_result = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: second.clone(),
        },
        &cancelled,
        |_| {},
    )
    .expect("second export");

    assert_eq!(snapshot(&fixture.source_files), before);
    assert_eq!(relative_files(&first), relative_files(&second));
    for relative in relative_files(&first) {
        assert_eq!(
            fs::read(first.join(&relative)).expect("first payload"),
            fs::read(second.join(&relative)).expect("second payload"),
            "{relative} must be deterministic"
        );
    }
    assert_eq!(first_result.manifest_file, second_result.manifest_file);

    let manifest_bytes = fs::read(first.join("manifest.json")).expect("manifest");
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes).expect("manifest JSON");
    let area_bytes = fs::read(first.join("area.json")).expect("area");
    let area: aurora_migration::MigrationAreaDocument =
        serde_json::from_slice(&area_bytes).expect("area JSON");
    assert_eq!(manifest.area_resref, "area_a");
    assert_eq!(manifest.coordinate_system.mapping, "[x,y,z] -> [x,z,-y]");
    assert_eq!(manifest.counts.unique_models, 1);
    assert_eq!(manifest.counts.textures, 1);
    assert_eq!(manifest.counts.preserved_navigation, 1);
    assert_eq!(area.source_files, vec!["area_a.are", "area_a.git"]);
    assert!(
        !String::from_utf8(area_bytes)
            .expect("UTF-8 area")
            .contains(&fixture._root.path().display().to_string())
    );
    assert_eq!(
        area.instances[0].source.source_path,
        "area_a.git::WaypointList[0]"
    );
    assert!(
        manifest
            .resources
            .iter()
            .any(|resource| resource.resource_key == "tile_a.mdl" && resource.shadowed.len() == 1)
    );
    assert!(["part_a.mdl", "super_a.mdl"].into_iter().all(|key| {
        manifest.resources.iter().any(|resource| {
            resource.resource_key == key && resource.purpose == "model-resolution-dependency"
        })
    }));
    for record in &manifest.files {
        let bytes = fs::read(first.join(&record.path)).expect("accounted file");
        assert_eq!(record.size_bytes, bytes.len() as u64);
        assert_eq!(record.sha256, hex::encode(Sha256::digest(&bytes)));
    }
    let diagnostics = fs::read_to_string(first.join("diagnostics.jsonl")).expect("diagnostics");
    assert!(!diagnostics.contains(&fixture._root.path().display().to_string()));
    let parsed_diagnostics = diagnostics
        .lines()
        .map(|line| {
            serde_json::from_str::<aurora_migration::MigrationDiagnostic>(line)
                .expect("diagnostic JSONL")
        })
        .collect::<Vec<_>>();
    assert_eq!(parsed_diagnostics.len(), manifest.counts.diagnostics);
    assert!(
        parsed_diagnostics
            .iter()
            .enumerate()
            .all(|(index, diagnostic)| diagnostic.sequence == index + 1)
    );
    let navigation = manifest
        .files
        .iter()
        .find(|record| record.path.starts_with("assets/source-navigation/"))
        .expect("preserved navigation record");
    assert_eq!(
        area.assets
            .iter()
            .find(|asset| asset.path == navigation.path)
            .expect("navigation asset")
            .surface_ids,
        [3]
    );
    assert_eq!(
        fs::read(first.join(&navigation.path)).expect("navigation bytes"),
        b"# Exported from NWmax\n#NWmax WALKMESH  ASCII\nbeginwalkmeshgeom tile\nnode aabb walk\nparent tile\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 3 0 1 2 4\naabb 0 0 0 1 1 0 0\nendnode\nendwalkmeshgeom tile\n"
    );
    assert_eq!(
        manifest.files.len() + 1,
        relative_files(&first).len(),
        "manifest accounts every payload; its own digest is returned out-of-band"
    );
    let glb = fs::read(
        relative_files(&first)
            .into_iter()
            .find(|path| path.starts_with("assets/models/") && path.ends_with(".glb"))
            .map(|path| first.join(path))
            .expect("model path"),
    )
    .expect("GLB");
    let json_length = u32::from_le_bytes(glb[12..16].try_into().expect("JSON length"));
    let document: serde_json::Value =
        serde_json::from_slice(&glb[20..20 + usize::try_from(json_length).expect("length")])
            .expect("GLB JSON");
    assert!(
        document["images"][0]["uri"]
            .as_str()
            .is_some_and(|uri| uri.starts_with("../textures/"))
    );
}

#[test]
fn bundle_validation_rejects_a_tampered_payload() {
    let fixture = synthetic_fixture(true);
    let destination = safe_destination(&fixture, "tampered.area-migration-v1");
    export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect("export");
    fs::write(destination.join("area.json"), b"{}\n").expect("tamper fixture output");

    let error = validate_bundle_directory(&destination).expect_err("tampering rejected");
    assert_eq!(error.code, "MIGRATION_FILE_INTEGRITY_MISMATCH");
}

#[test]
fn bundle_validation_enforces_the_versioned_manifest_contract() {
    let fixture = synthetic_fixture(true);
    let destination = safe_destination(&fixture, "invalid-contract.area-migration-v1");
    export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect("export");
    let manifest_path = destination.join("manifest.json");
    let mut manifest: BundleManifest =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest bytes"))
            .expect("manifest");
    manifest.classification = "redistributable".to_owned();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("tampered manifest"),
    )
    .expect("write tampered manifest");

    let error = validate_bundle_directory(&destination).expect_err("contract violation rejected");
    assert_eq!(error.code, "MIGRATION_MANIFEST_SCHEMA_INVALID");
}

#[test]
fn cancellation_never_publishes_a_partial_destination() {
    let fixture = synthetic_fixture(true);
    let destination = safe_destination(&fixture, "cancelled.area-migration-v1");
    let cancelled = AtomicBool::new(true);
    let error = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &cancelled,
        |_| {},
    )
    .expect_err("cancelled");
    assert_eq!(error.code, "JOB_CANCELLED", "error: {error:?}");
    assert!(!destination.exists());
}

#[test]
fn audit_rejects_a_hostile_texture_before_materializing_payloads() {
    let fixture = synthetic_fixture(true);
    let texture_path = fixture._root.path().join("stone.tga");
    let mut hostile = vec![0_u8; 18];
    hostile[2] = 2;
    hostile[12..14].copy_from_slice(&8_192_u16.to_le_bytes());
    hostile[14..16].copy_from_slice(&4_097_u16.to_le_bytes());
    hostile[16] = 32;
    hostile[17] = 0x20;
    fs::write(&texture_path, hostile).expect("hostile texture header");

    let preview = audit_area_migration(&fixture.source, "area_a", &AtomicBool::new(false))
        .expect("audit returns a bounded refusal");

    assert!(!preview.ready);
    assert!(preview.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "MIGRATION_TEXTURE_DECODE_LIMIT"
            && diagnostic.status == aurora_migration::MigrationStatus::Unsupported
    }));
    assert!(preview.diagnostics.iter().all(|diagnostic| {
        !diagnostic.code.contains("GLB") && !diagnostic.code.contains("PNG_WRITE")
    }));
}

#[test]
fn export_rejects_a_resource_changed_after_audit_without_publishing() {
    let fixture = synthetic_fixture(true);
    audit_area_migration(&fixture.source, "area_a", &AtomicBool::new(false)).expect("audit");
    let texture_path = fixture._root.path().join("stone.tga");
    let mut changed = fs::read(&texture_path).expect("texture");
    changed[18] ^= 0x7f;
    fs::write(&texture_path, changed).expect("mutate selected texture");
    let destination = safe_destination(&fixture, "changed-source.area-migration-v1");

    let error = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect_err("changed resource is rejected");

    assert_eq!(error.code, "MIGRATION_SOURCE_CHANGED");
    assert!(!destination.exists());
}

#[test]
fn texture_fallback_rejects_invalid_dds_then_uses_the_deterministic_tga_candidate() {
    let mut fixture = synthetic_fixture(true);
    let dds_path = fixture._root.path().join("stone.dds");
    fs::write(&dds_path, b"DDS-invalid-candidate").expect("invalid DDS");
    fixture
        .source
        .resource_catalog
        .entries
        .push(resolved(&dds_path, "stone", 2033, Vec::new()));
    fixture
        .source
        .resource_catalog
        .entries
        .sort_by(|left, right| left.key.cmp(&right.key));
    let preview = audit_area_migration(&fixture.source, "area_a", &AtomicBool::new(false))
        .expect("fallback audit");
    assert!(
        preview.ready,
        "valid lower-priority fallback remains exportable"
    );
    assert!(
        preview.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "MIGRATION_TEXTURE_CANDIDATE_REJECTED"
                && diagnostic.resource.as_deref() == Some("stone.dds")
        }),
        "diagnostics: {:?}",
        preview.diagnostics
    );
    let destination = safe_destination(&fixture, "texture-fallback.area-migration-v1");
    export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect("fallback export");
    let manifest: BundleManifest =
        serde_json::from_slice(&fs::read(destination.join("manifest.json")).expect("manifest"))
            .expect("manifest JSON");
    assert!(manifest.resources.iter().any(|resource| {
        resource.resource_key == "stone.tga"
            && resource.purpose == "base-color-texture"
            && resource.selected.source_file_name == "stone.tga"
    }));
}

#[test]
fn cancellation_during_sequential_materialization_leaves_no_staging_or_destination() {
    let fixture = synthetic_fixture(true);
    let destination = safe_destination(&fixture, "cancelled-during-export.area-migration-v1");
    let cancelled = AtomicBool::new(false);
    let error = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &cancelled,
        |progress| {
            if matches!(progress.phase, aurora_migration::MigrationPhase::Navigation) {
                cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        },
    )
    .expect_err("cancellation during materialization");

    assert_eq!(error.code, "JOB_CANCELLED", "error: {error:?}");
    assert!(!destination.exists());
    assert!(
        fs::read_dir(fixture._safe_root.path())
            .expect("fixture root")
            .filter_map(Result::ok)
            .all(|entry| {
                !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".opennever-area-migration-")
            })
    );
}

#[test]
fn audit_reports_missing_resource_closure_instead_of_dropping_it() {
    let fixture = synthetic_fixture(false);
    let preview =
        audit_area_migration(&fixture.source, "area_a", &AtomicBool::new(false)).expect("audit");
    assert!(!preview.ready, "missing resources block publication");
    assert!(
        !preview.complete,
        "missing resources make the audit incomplete"
    );
    assert!(preview.counts.missing_items > 0);
    assert!(preview.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code.as_str(),
            "RESOURCE_CLOSURE_ITEM_MISSING" | "MDL_RESOURCE_NOT_FOUND"
        )
    }));
    let destination = safe_destination(&fixture, "missing-closure.area-migration-v1");
    let error = export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect_err("blocking audit prevents publication");
    assert_eq!(error.code, "MIGRATION_EXPORT_BLOCKED");
    assert!(!destination.exists());
}

#[test]
fn destination_validation_rejects_relative_and_existing_paths() {
    let root = tempfile::tempdir().expect("tempdir");
    assert!(validate_bundle_destination(Path::new("relative/output")).is_err());
    let existing = root.path().join("existing");
    fs::create_dir(&existing).expect("existing");
    let error = validate_bundle_destination(&existing).expect_err("existing rejected");
    assert_eq!(error.code, "MIGRATION_DESTINATION_EXISTS");
    let protected_root = root.path().join("nwn-install");
    let protected_parent = protected_root.join("modules");
    fs::create_dir_all(&protected_parent).expect("protected root");
    let unsafe_destination = protected_parent.join("export.area-migration-v1");
    let error = aurora_migration::validate_bundle_destination_with_sources(
        &unsafe_destination,
        &[protected_root],
    )
    .expect_err("protected descendant rejected");
    assert_eq!(error.code, "MIGRATION_DESTINATION_UNSAFE");
}

#[test]
fn audit_reports_an_unchecked_declared_dependency_as_incomplete() {
    let mut fixture = synthetic_fixture(true);
    fixture.source.dependency_report = ModuleDependencyReport {
        dependencies: vec![ModuleDependency {
            kind: ModuleDependencyKind::Hak,
            logical_name: "synthetic.hak".to_owned(),
            state: ModuleDependencyState::Unchecked,
            selected_path: None,
            shadowed_paths: Vec::new(),
            searched_paths: Vec::new(),
            fingerprint: None,
            change: ModuleDependencyChange::FirstSeen,
        }],
        resolved_count: 0,
        missing_count: 0,
        unchecked_count: 1,
        invalid_count: 0,
        changed_count: 0,
    };

    let preview =
        audit_area_migration(&fixture.source, "area_a", &AtomicBool::new(false)).expect("audit");
    assert!(!preview.ready, "unchecked dependency blocks publication");
    assert!(!preview.complete);
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "MIGRATION_DEPENDENCY_UNCHECKED")
    );
}

#[test]
fn export_normalizes_resolved_dependency_fingerprints() {
    let mut fixture = synthetic_fixture(true);
    let dependency_path = fixture._root.path().join("synthetic.hak");
    let dependency_bytes = b"synthetic dependency payload";
    fs::write(&dependency_path, dependency_bytes).expect("dependency");
    let uppercase_sha256 = hex::encode_upper(Sha256::digest(dependency_bytes));
    fixture.source.dependency_report = ModuleDependencyReport {
        dependencies: vec![ModuleDependency {
            kind: ModuleDependencyKind::Hak,
            logical_name: "synthetic.hak".to_owned(),
            state: ModuleDependencyState::Resolved,
            selected_path: Some(dependency_path.display().to_string()),
            shadowed_paths: Vec::new(),
            searched_paths: vec![dependency_path.display().to_string()],
            fingerprint: Some(ModuleFingerprint {
                sha256: uppercase_sha256.clone(),
                size_bytes: dependency_bytes.len() as u64,
            }),
            change: ModuleDependencyChange::FirstSeen,
        }],
        resolved_count: 1,
        missing_count: 0,
        unchecked_count: 0,
        invalid_count: 0,
        changed_count: 0,
    };
    let destination = safe_destination(&fixture, "dependency.area-migration-v1");

    export_area_migration(
        &fixture.source,
        &AreaMigrationExportRequest {
            area_resref: "area_a".to_owned(),
            destination: destination.clone(),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect("export with resolved dependency");

    let manifest: BundleManifest =
        serde_json::from_slice(&fs::read(destination.join("manifest.json")).expect("manifest"))
            .expect("manifest JSON");
    assert_eq!(
        manifest.dependencies[0].selected_content_sha256.as_deref(),
        Some(uppercase_sha256.to_ascii_lowercase().as_str())
    );
}

#[test]
fn documented_manifest_schema_tracks_the_v1_contract_and_file_budget() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/area-migration-bundle-v1.schema.json"
    ))
    .expect("documented JSON Schema");
    assert_eq!(
        schema["properties"]["schemaVersion"]["const"],
        aurora_migration::BUNDLE_SCHEMA_VERSION
    );
    assert_eq!(
        schema["properties"]["files"]["maxItems"],
        aurora_migration::MAX_BUNDLE_FILES
    );
    assert_eq!(
        schema["properties"]["coordinateSystem"]["properties"]["mapping"]["const"],
        "[x,y,z] -> [x,z,-y]"
    );
}

fn synthetic_fixture(include_assets: bool) -> Fixture {
    let root = tempfile::tempdir().expect("tempdir");
    let module_path = root.path().join("synthetic.mod");
    let are_path = root.path().join("area_a.are");
    let git_path = root.path().join("area_a.git");
    let model_path = root.path().join("tile_a.mdl");
    let shadowed_model_path = root.path().join("shadowed_tile_a.mdl");
    let referenced_model_path = root.path().join("part_a.mdl");
    let supermodel_path = root.path().join("super_a.mdl");
    let texture_path = root.path().join("stone.tga");
    let wok_path = root.path().join("tile_a.wok");
    fs::write(&are_path, b"synthetic ARE provenance only").expect("ARE");
    fs::write(&git_path, b"synthetic GIT provenance only").expect("GIT");
    fs::write(&module_path, b"immutable synthetic module source").expect("module");
    let model = b"newmodel tile_a\nsetsupermodel tile_a super_a\nnode reference attachment\nrefmodel part_a\nendnode\nnode trimesh body\nbitmap stone\nverts 3\n0 0 0\n1 0 0\n0 1 0\ntverts 3\n0 0\n1 0\n0 1\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n";
    fs::write(&model_path, model).expect("MDL");
    fs::write(&shadowed_model_path, model).expect("shadowed MDL");
    fs::write(
        &referenced_model_path,
        b"newmodel part_a\nnode trimesh part\nverts 3\n0 0 0\n0 1 0\n0 0 1\nfaces 1\n0 1 2 0 0 0 0 0\nendnode\n",
    )
    .expect("referenced MDL");
    fs::write(
        &supermodel_path,
        b"newmodel super_a\nnewanim idle\ndoneanim idle\n",
    )
    .expect("supermodel MDL");
    let mut tga = vec![0_u8; 18];
    tga[2] = 2;
    tga[12..14].copy_from_slice(&1_u16.to_le_bytes());
    tga[14..16].copy_from_slice(&1_u16.to_le_bytes());
    tga[16] = 24;
    tga[17] = 0x20;
    tga.extend([16, 32, 64]);
    fs::write(&texture_path, tga).expect("TGA");
    fs::write(
        &wok_path,
        b"# Exported from NWmax\n#NWmax WALKMESH  ASCII\nbeginwalkmeshgeom tile\nnode aabb walk\nparent tile\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 3 0 1 2 4\naabb 0 0 0 1 1 0 0\nendnode\nendwalkmeshgeom tile\n",
    )
    .expect("WOK");

    let mut entries = vec![resolved(&are_path, "area_a", 2012, Vec::new())];
    if include_assets {
        entries.push(resolved(
            &model_path,
            "tile_a",
            2002,
            vec![version(&shadowed_model_path, "tile_a", 2002, 20)],
        ));
        entries.push(resolved(&referenced_model_path, "part_a", 2002, Vec::new()));
        entries.push(resolved(&supermodel_path, "super_a", 2002, Vec::new()));
        entries.push(resolved(&texture_path, "stone", 3, Vec::new()));
        entries.push(resolved(&wok_path, "tile_a", 2016, Vec::new()));
    }
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    let catalog = ResourceCatalog {
        version_count: entries.iter().map(|entry| 1 + entry.shadowed.len()).sum(),
        shadowed_count: entries.iter().map(|entry| entry.shadowed.len()).sum(),
        entries,
        diagnostics: Vec::new(),
    };
    let area = AreaMap {
        resref: "area_a".to_owned(),
        name: LocalizedText {
            string_ref: None,
            text: Some("Synthetic area".to_owned()),
        },
        width: 1,
        height: 1,
        tileset: None,
        tiles: vec![AreaTile {
            x: 0,
            y: 0,
            tile_id: 7,
            orientation: 1,
            height: 0,
        }],
        instances: vec![AreaInstance {
            id: "area_a:WaypointList:0".to_owned(),
            category: "waypoint".to_owned(),
            tag: Some("WP_SYNTHETIC".to_owned()),
            template_resref: None,
            x: 2.0,
            y: 3.0,
            z: 0.5,
            bearing: Some(0.0),
            x_orientation: None,
            y_orientation: None,
            appearance: None,
            transition_destination: None,
            transition_flags: None,
            load_screen_id: None,
            geometry: Vec::new(),
            spawn_points: Vec::new(),
            inventory: Vec::new(),
            source_path: format!("{}::WaypointList[0]", git_path.display()),
        }],
        diagnostics: Vec::new(),
        are_source: are_path.display().to_string(),
        git_source: Some(git_path.display().to_string()),
        gic_source: None,
    };
    let scene = SceneManifest {
        area: "area_a".to_owned(),
        width: 1,
        height: 1,
        tileset: None,
        objects: vec![SceneObject {
            id: "tile:0:0".to_owned(),
            kind: "tile".to_owned(),
            label: "Tile 7".to_owned(),
            x: 5.0,
            y: 0.0,
            z: 5.0,
            rotation: std::f32::consts::FRAC_PI_2,
            marker: !include_assets,
            model_resref: Some("tile_a".to_owned()),
            model_resrefs: vec!["tile_a".to_owned()],
            walkmesh_available: include_assets,
            source_path: are_path.display().to_string(),
        }],
        overlays: vec![SceneObject {
            id: "area_a:WaypointList:0".to_owned(),
            kind: "waypoint".to_owned(),
            label: "WP_SYNTHETIC".to_owned(),
            x: 2.0,
            y: 0.5,
            z: 3.0,
            rotation: 0.0,
            marker: true,
            model_resref: None,
            model_resrefs: Vec::new(),
            walkmesh_available: false,
            source_path: format!("{}::WaypointList[0]", git_path.display()),
        }],
        resolved_assets: usize::from(include_assets),
        unique_models: 1,
        walkmesh_assets: usize::from(include_assets),
        missing_assets: usize::from(!include_assets),
        memory_budget_bytes: 256 * 1024 * 1024,
        diagnostics: Vec::new(),
    };
    let mut world = WorldIndex::default();
    world.areas.push(area);
    world.scenes.push(scene);
    Fixture {
        source: AreaMigrationSource {
            module_path: module_path.clone(),
            module_sha256: hex::encode_upper(Sha256::digest(b"immutable synthetic module source")),
            module_size_bytes: b"immutable synthetic module source".len() as u64,
            resource_catalog: catalog,
            world_index: world,
            dependency_report: ModuleDependencyReport {
                dependencies: Vec::new(),
                resolved_count: 0,
                missing_count: 0,
                unchecked_count: 0,
                invalid_count: 0,
                changed_count: 0,
            },
            protected_roots: vec![root.path().to_path_buf()],
            source_snapshot: Arc::new(Mutex::new(None)),
        },
        source_files: vec![
            module_path,
            are_path,
            git_path,
            model_path,
            shadowed_model_path,
            referenced_model_path,
            supermodel_path,
            texture_path,
            wok_path,
        ],
        _safe_root: tempfile::tempdir().expect("safe export root"),
        _root: root,
    }
}

fn safe_destination(fixture: &Fixture, name: &str) -> PathBuf {
    fixture._safe_root.path().join(name)
}

fn resolved(
    path: &Path,
    resref: &str,
    resource_type: u16,
    shadowed: Vec<ResourceVersion>,
) -> ResolvedResource {
    ResolvedResource {
        key: ResourceKey::new(resref, resource_type),
        selected: version(path, resref, resource_type, 10),
        shadowed,
    }
}

fn version(path: &Path, resref: &str, resource_type: u16, priority: u32) -> ResourceVersion {
    let bytes = fs::read(path).expect("fixture source");
    ResourceVersion {
        key: ResourceKey::new(resref, resource_type),
        source_kind: ResourceSourceKind::Development,
        source_name: path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned(),
        source_path: path.display().to_string(),
        priority,
        offset: 0,
        size: bytes.len() as u64,
        sha256: Some(hex::encode(Sha256::digest(&bytes))),
        location: ResourceLocation::File {
            path: path.display().to_string(),
        },
    }
}

fn snapshot(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    paths
        .iter()
        .map(|path| fs::read(path).expect("source snapshot"))
        .collect()
}

fn relative_files(root: &Path) -> Vec<String> {
    fn visit(root: &Path, current: &Path, target: &mut Vec<String>) {
        for entry in fs::read_dir(current).expect("read bundle") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                visit(root, &path, target);
            } else {
                target.push(
                    path.strip_prefix(root)
                        .expect("relative")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    files
}
