use aurora_project::{
    DependencyRoots, ModuleDependencyKind, ModuleDependencyState, analyze_module_file_with_roots,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[test]
fn analyzes_the_redistributable_custom_tlk_fixture() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/lot1_custom_tlk");
    let module = root.join("module/forge_lot1.mod");
    let analysis = analyze_module_file_with_roots(
        &module,
        &DependencyRoots {
            game_install_path: None,
            user_data_path: Some(root.join("user")),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .expect("synthetic Lot 1 fixture must be valid");

    assert_eq!(
        analysis.module_info.name.primary_text(),
        Some("OpenNever Forge Lot 1")
    );
    assert_eq!(analysis.module_info.tag, "OPENNEVER_LOT1");
    assert_eq!(analysis.module_info.entry_area, "startarea");
    assert_eq!(analysis.module_info.hak_files, vec!["forge_assets"]);
    assert_eq!(
        analysis.module_info.custom_tlk.as_deref(),
        Some("forge_dialog")
    );
    assert_eq!(analysis.dependency_report.resolved_count, 2);
    assert_eq!(analysis.dependency_report.missing_count, 0);
    assert_eq!(analysis.dependency_report.dependencies.len(), 2);
    assert_eq!(
        analysis.dependency_report.dependencies[0].kind,
        ModuleDependencyKind::Hak
    );
    assert_eq!(
        analysis.dependency_report.dependencies[1].kind,
        ModuleDependencyKind::CustomTlk
    );
    assert!(
        analysis
            .dependency_report
            .dependencies
            .iter()
            .all(|dependency| dependency.state == ModuleDependencyState::Resolved)
    );
    assert!(
        analysis
            .dependency_report
            .dependencies
            .iter()
            .all(|dependency| dependency.fingerprint.is_some())
    );
    assert!(
        analysis.resource_catalog_summary.resource_count
            >= analysis.inventory.resource_count as usize
    );
    assert_eq!(analysis.resource_catalog_summary.diagnostic_count, 0);
    assert_eq!(analysis.structured_summary.gff.failed, 0);
    assert!(analysis.structured_summary.gff.parsed >= 1);
    assert_eq!(analysis.structured_summary.talk_tables.len(), 1);
    assert_eq!(analysis.structured_summary.talk_tables[0].kind, "custom");
    assert!(analysis.structured_summary.diagnostics.is_empty());
    assert_eq!(analysis.script_index_summary.scripts, 2);
    assert_eq!(analysis.script_index_summary.nss, 2);
    assert_eq!(analysis.script_index_summary.ncs, 1);
    assert_eq!(analysis.script_index_summary.missing_source, 0);
    assert!(
        analysis
            .script_index
            .get("forge_start")
            .is_some_and(|script| script
                .nss
                .as_ref()
                .is_some_and(|nss| nss.includes[0].resolved))
    );
}
