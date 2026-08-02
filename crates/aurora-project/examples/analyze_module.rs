use aurora_project::analyze_module_file;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: analyze_module <module.mod>");
        return ExitCode::from(2);
    };

    match analyze_module_file(Path::new(&path), &AtomicBool::new(false), |_| {}) {
        Ok(analysis) => {
            let name = analysis
                .module_info
                .name
                .primary_text()
                .unwrap_or("<no embedded name>");
            println!("Name: {name}");
            println!("Tag: {}", analysis.module_info.tag);
            println!("Minimum NWN: {}", analysis.module_info.minimum_game_version);
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
            println!("SHA-256: {}", analysis.fingerprint.sha256);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
