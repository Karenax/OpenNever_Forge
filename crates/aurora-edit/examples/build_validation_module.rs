use std::env;
use std::path::PathBuf;

use aurora_edit::{NewModuleDefinition, create_empty_module};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: build_validation_module <output.mod>")?;
    let report = create_empty_module(
        &output,
        &NewModuleDefinition {
            name: "OpenNever Forge Validation".to_owned(),
            tag: "OPENNEVER_VALIDATION".to_owned(),
            entry_area: "onf_start".to_owned(),
            tileset: "tno01".to_owned(),
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
