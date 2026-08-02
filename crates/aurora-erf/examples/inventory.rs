use aurora_erf::{ContainerReader, ErfReader};
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: inventory <container.mod>");
        return ExitCode::from(2);
    };

    match ErfReader::default().read_inventory(Path::new(&path), &AtomicBool::new(false)) {
        Ok(inventory) => {
            println!(
                "{} {}: {} resources, {} resource types",
                inventory.file_type.trim(),
                inventory.file_version,
                inventory.resource_count,
                inventory.type_summaries.len()
            );
            for summary in inventory.type_summaries {
                println!(
                    "{:>5} {:>6} {:>10} bytes",
                    summary.extension.as_deref().unwrap_or("?"),
                    summary.count,
                    summary.total_size
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
