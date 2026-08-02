use aurora_project::hash_module_file;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: hash_module <module.mod>");
        return ExitCode::from(2);
    };

    match hash_module_file(Path::new(&path), &AtomicBool::new(false), |_| {}) {
        Ok(fingerprint) => {
            println!(
                "{}  {}  {} bytes",
                fingerprint.sha256, path, fingerprint.size_bytes
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
