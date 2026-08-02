use crate::{HashProgress, ModuleFingerprint, hash_module_file};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_erf::{ContainerInventory, ContainerReader, ErfReader};
use aurora_gff::{ModuleInfo, read_module_info};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAnalysis {
    pub fingerprint: ModuleFingerprint,
    pub inventory: ContainerInventory,
    pub module_info: ModuleInfo,
}

pub fn analyze_module_file<F>(
    path: &Path,
    cancelled: &AtomicBool,
    on_progress: F,
) -> AppResult<ModuleAnalysis>
where
    F: FnMut(HashProgress),
{
    let fingerprint = hash_module_file(path, cancelled, on_progress)?;
    let reader = ErfReader::default();
    let inventory = reader.read_inventory(path, cancelled)?;
    let module_resources = inventory
        .resources
        .iter()
        .filter(|resource| {
            resource.key.resource_type == 2014 && resource.key.resref.eq_ignore_ascii_case("module")
        })
        .collect::<Vec<_>>();
    let module_resource = match module_resources.as_slice() {
        [resource] => *resource,
        [] => {
            return Err(module_info_error(
                path,
                "MODULE_IFO_NOT_FOUND",
                "No module.ifo resource exists in the selected container".to_owned(),
            ));
        }
        resources => {
            return Err(module_info_error(
                path,
                "MODULE_IFO_AMBIGUOUS",
                format!(
                    "Container has {} resources matching module.ifo",
                    resources.len()
                ),
            ));
        }
    };
    let module_bytes = reader.read_resource(path, module_resource, cancelled)?;
    let module_info = read_module_info(&module_bytes, &format!("{}::module.ifo", path.display()))?;

    Ok(ModuleAnalysis {
        fingerprint,
        inventory,
        module_info,
    })
}

fn module_info_error(path: &Path, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Le fichier module.ifo est absent ou ambigu.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(path.display().to_string())
        .with_resource("module.ifo")
        .with_import_stage("module_info")
        .with_suggestion("Vérifiez que la copie sélectionnée est un module NWN complet."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[test]
    fn rejects_a_container_without_module_info() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("empty.mod");
        let mut bytes = vec![0_u8; 160];
        bytes[0..4].copy_from_slice(b"MOD ");
        bytes[4..8].copy_from_slice(b"V1.0");
        bytes[20..24].copy_from_slice(&160_u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&160_u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&160_u32.to_le_bytes());
        fs::write(&module, bytes).expect("write synthetic module");

        let error = analyze_module_file(&module, &AtomicBool::new(false), |_| {})
            .expect_err("missing module.ifo");

        assert_eq!(error.code, "MODULE_IFO_NOT_FOUND");
    }
}
