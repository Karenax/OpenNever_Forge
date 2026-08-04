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
            commands::inspect_world,
            commands::inspect_narrative,
            commands::inspect_scene,
            commands::model_preview_glb,
            commands::resolve_texture,
            commands::asset_preview_bytes,
            commands::diagnostic_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenNever Forge");
}
