use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use aurora_erf::{
    ContainerReader, ErfReader, ErfResourceSource, ErfResourceStreamInput,
    write_erf_streaming_with_metadata,
};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: repack_module <input.mod> <output.mod>")?;
    let output = env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .ok_or("usage: repack_module <input.mod> <output.mod>")?;
    let reader = ErfReader::default();
    let inventory = reader.read_inventory(&input, &AtomicBool::new(false))?;
    let metadata = reader.read_archive_metadata(&input)?;
    let resources = inventory
        .resources
        .into_iter()
        .map(|resource| ErfResourceStreamInput {
            key: resource.key,
            source: ErfResourceSource::Range {
                path: input.clone(),
                offset: resource.offset,
                size: resource.size,
            },
        })
        .collect::<Vec<_>>();
    write_erf_streaming_with_metadata(&output, "MOD ", &resources, &metadata)?;
    Ok(())
}
