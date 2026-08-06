mod commands;
mod jobs;
mod logging;
mod state;

use aurora_index::initialize_database;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_local_data_dir()?;
            let log_guard = logging::initialize(&app_data_dir.join("logs"))?;
            let database = initialize_database(&app_data_dir.join("index.sqlite3"))
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;

            tracing::info!(
                schema_version = database.schema_version,
                "OpenNever Forge application state initialized"
            );
            app.manage(AppState::new(
                database,
                app_data_dir.join("asset-cache"),
                log_guard,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::start_module_analysis,
            commands::get_job,
            commands::cancel_job,
            commands::query_resources,
            commands::inspect_resource,
            commands::query_scripts,
            commands::inspect_script,
            commands::query_dialogues,
            commands::inspect_dialogue,
            commands::edit_dialogue_field,
            commands::edit_dialogue_structure_command,
            commands::inspect_world,
            commands::inspect_narrative,
            commands::inspect_narrative_documents,
            commands::edit_journal_structure_command,
            commands::edit_faction_structure_command,
            commands::edit_blueprint_structure_command,
            commands::inspect_scene,
            commands::model_preview_glb,
            commands::resolve_texture,
            commands::asset_preview_bytes,
            commands::diagnostic_report,
            commands::create_edit_workspace,
            commands::get_edit_workspace,
            commands::undo_edit_command,
            commands::redo_edit_command,
            commands::apply_gff_edit,
            commands::edit_script_source,
            commands::compile_workspace_script,
            commands::move_area_instance,
            commands::set_area_tile,
            commands::edit_area_structure_command,
            commands::inspect_workspace_area,
            commands::build_workspace_module,
            commands::deploy_workspace_development,
            commands::clean_workspace_development,
            commands::build_workspace_hak,
            commands::export_workspace_sources,
            commands::edit_workspace_2da,
            commands::edit_workspace_tlk,
            commands::edit_workspace_module_dependencies,
            commands::list_workspace_build_profiles,
            commands::save_workspace_build_profile,
            commands::verify_workspace_reproducible_build,
            commands::run_workspace_build_profile,
            commands::inspect_git_workspace,
            commands::list_workspace_launch_profiles,
            commands::save_workspace_launch_profile,
            commands::launch_workspace_test_profile,
            commands::inspect_aurora_workspace,
            commands::plan_aurora_workspace_sync,
            commands::apply_aurora_workspace_sync,
            commands::validate_walkmesh_draft,
            commands::transform_walkmesh_draft,
            commands::inspect_workspace_walkmesh,
            commands::save_workspace_walkmesh,
            commands::preview_ai_change_set,
            commands::request_ai_change_set,
            commands::apply_ai_change_set,
            commands::get_agent_studio_state,
            commands::save_agent_policy,
            commands::create_agent_run,
            commands::validate_agent_blueprint,
            commands::advance_agent_run,
            commands::test_agent_provider,
            commands::cancel_agent_run,
            commands::resolve_agent_approval,
            commands::create_new_module,
            commands::get_standard_palette,
            commands::create_workspace_area,
            commands::list_workspace_created_areas,
            commands::delete_workspace_area,
            commands::add_workspace_area_instance,
            commands::remove_workspace_area_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenNever Forge");
}
