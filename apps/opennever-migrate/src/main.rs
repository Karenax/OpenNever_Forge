use aurora_migration::{
    AreaMigrationExportRequest, AreaMigrationSource, audit_area_migration, export_area_migration,
};
use aurora_project::{DependencyRoots, analyze_module_file_with_roots};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    AreaAudit,
    AreaExport,
}

#[derive(Debug)]
struct Options {
    command: Command,
    module: PathBuf,
    area: String,
    output: Option<PathBuf>,
    game_install: Option<PathBuf>,
    user_data: Option<PathBuf>,
}

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.is_empty()
        || matches!(arguments.first().map(String::as_str), Some("--help" | "-h"))
    {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }
    match run(arguments) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: impl IntoIterator<Item = String>) -> Result<u8, String> {
    let options = parse_options(arguments)?;
    let cancelled = AtomicBool::new(false);
    let roots = DependencyRoots {
        game_install_path: options.game_install,
        user_data_path: options.user_data,
    };
    let analysis = analyze_module_file_with_roots(&options.module, &roots, &cancelled, |_| {})
        .map_err(|error| format_error(&error))?;
    let protected_roots = [
        roots.game_install_path.clone(),
        roots.user_data_path.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let source =
        AreaMigrationSource::from_analysis_with_roots(&analysis, &options.module, protected_roots);
    let preview = audit_area_migration(&source, &options.area, &cancelled)
        .map_err(|error| format_error(&error))?;

    match options.command {
        Command::AreaAudit => {
            println!(
                "{}",
                serde_json::to_string_pretty(&preview)
                    .map_err(|error| format!("CLI_JSON_FAILED: {error}"))?
            );
            Ok(preview_exit_code(&preview))
        }
        Command::AreaExport => {
            let destination = options
                .output
                .ok_or_else(|| "area-export requires --output <directory>".to_owned())?;
            let result = export_area_migration(
                &source,
                &AreaMigrationExportRequest {
                    area_resref: options.area,
                    destination,
                },
                &cancelled,
                |_| {},
            )
            .map_err(|error| format_error(&error))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("CLI_JSON_FAILED: {error}"))?
            );
            Ok(if result.report.complete { 0 } else { 2 })
        }
    }
}

fn preview_exit_code(preview: &aurora_migration::AreaMigrationPreview) -> u8 {
    if !preview.ready {
        1
    } else if preview.complete {
        0
    } else {
        2
    }
}

fn parse_options(arguments: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut arguments = arguments.into_iter();
    let command = match arguments.next().as_deref() {
        Some("area-audit") => Command::AreaAudit,
        Some("area-export") => Command::AreaExport,
        Some("--help" | "-h") | None => return Err(usage().to_owned()),
        Some(value) => return Err(format!("unknown command {value}\n\n{}", usage())),
    };
    let mut module = None;
    let mut area = None;
    let mut output = None;
    let mut game_install = None;
    let mut user_data = None;
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        let target = match flag.as_str() {
            "--module" | "--source-module" => &mut module,
            "--area" => &mut area,
            "--output" => &mut output,
            "--game-install" => &mut game_install,
            "--user-data" => &mut user_data,
            _ => return Err(format!("unknown option {flag}\n\n{}", usage())),
        };
        if target.replace(value).is_some() {
            return Err(format!("{flag} was provided more than once"));
        }
    }
    let module = module.ok_or_else(|| "--module <path> is required".to_owned())?;
    let area = area.ok_or_else(|| "--area <resref> is required".to_owned())?;
    if command == Command::AreaExport && output.is_none() {
        return Err("area-export requires --output <directory>".to_owned());
    }
    Ok(Options {
        command,
        module: PathBuf::from(module),
        area,
        output: output.map(PathBuf::from),
        game_install: game_install.map(PathBuf::from),
        user_data: user_data.map(PathBuf::from),
    })
}

fn format_error(error: &aurora_core::AppError) -> String {
    serde_json::to_string_pretty(error)
        .unwrap_or_else(|_| format!("{}: {}", error.code, error.technical_message))
}

fn usage() -> &'static str {
    "OpenNever area migration (headless)\n\n\
Usage:\n\
  opennever-migrate area-audit --module <module.mod> --area <resref> [dependencies]\n\
  opennever-migrate area-export --module <module.mod> --area <resref> --output <new-directory> [dependencies]\n\n\
Dependency roots (explicit; no machine discovery):\n\
  --game-install <directory>  NWN installation root used by Resource Manager\n\
  --user-data <directory>     NWN user-data root used for HAK/TLK/override resolution\n\n\
The CLI performs its own read-only analysis. It cannot reuse an in-memory desktop analysis across process boundaries."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_audit_and_dependency_roots() {
        let options = parse_options([
            "area-audit".to_owned(),
            "--module".to_owned(),
            "module.mod".to_owned(),
            "--area".to_owned(),
            "area_a".to_owned(),
            "--game-install".to_owned(),
            "C:/NWN".to_owned(),
        ])
        .expect("options");
        assert_eq!(options.command, Command::AreaAudit);
        assert_eq!(options.area, "area_a");
        assert_eq!(options.game_install, Some(PathBuf::from("C:/NWN")));
    }

    #[test]
    fn export_requires_output() {
        let error = parse_options([
            "area-export".to_owned(),
            "--module".to_owned(),
            "module.mod".to_owned(),
            "--area".to_owned(),
            "area_a".to_owned(),
        ])
        .expect_err("missing output");
        assert!(error.contains("--output"));
    }

    #[test]
    fn audit_exit_code_distinguishes_complete_allowed_incomplete_and_blocked() {
        let mut preview = aurora_migration::AreaMigrationPreview {
            schema_version: "area-migration-bundle@1.0.0".to_owned(),
            area_resref: "area_a".to_owned(),
            area_name: "Area".to_owned(),
            suggested_directory_name: "area_a.area-migration-v1".to_owned(),
            ready: true,
            complete: true,
            counts: Default::default(),
            diagnostics: Vec::new(),
            classification: "local_only_proprietary".to_owned(),
            redistribution: "not_redistributable_without_separate_rights".to_owned(),
            navigation_status: "preserved-not-converted".to_owned(),
        };
        assert_eq!(preview_exit_code(&preview), 0);
        preview.complete = false;
        assert_eq!(preview_exit_code(&preview), 2);
        preview.ready = false;
        assert_eq!(preview_exit_code(&preview), 1);
    }
}
