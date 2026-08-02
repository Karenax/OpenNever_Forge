use aurora_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PROJECT_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadonlyProjectDraft {
    pub name: String,
    pub module_path: PathBuf,
    pub game_install_path: PathBuf,
    pub user_data_path: PathBuf,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatedProjectPaths {
    pub project_version: u32,
    pub name: String,
    pub module_path: PathBuf,
    pub game_install_path: PathBuf,
    pub user_data_path: PathBuf,
    pub read_only: bool,
}

impl ReadonlyProjectDraft {
    pub fn validate(self) -> AppResult<ValidatedProjectPaths> {
        if !self.read_only {
            return Err(AppError::invalid_path(
                self.module_path.display().to_string(),
                "Phase 1 projects must set read_only=true",
            )
            .into());
        }

        validate_module(&self.module_path)?;
        validate_directory(&self.game_install_path, "game_install_path")?;
        validate_directory(&self.user_data_path, "user_data_path")?;

        let name = self.name.trim().to_owned();
        if name.is_empty() {
            return Err(AppError::invalid_path(
                self.module_path.display().to_string(),
                "Project name cannot be empty",
            )
            .into());
        }

        Ok(ValidatedProjectPaths {
            project_version: PROJECT_FILE_VERSION,
            name,
            module_path: self.module_path,
            game_install_path: self.game_install_path,
            user_data_path: self.user_data_path,
            read_only: true,
        })
    }
}

fn validate_module(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::module_not_found(path.display().to_string()).into());
    }

    let extension = path.extension().and_then(|value| value.to_str());
    if !extension.is_some_and(|value| value.eq_ignore_ascii_case("mod")) {
        return Err(AppError::invalid_path(
            path.display().to_string(),
            "Selected module does not have the .mod extension",
        )
        .into());
    }

    Ok(())
}

fn validate_directory(path: &Path, field: &str) -> AppResult<()> {
    if path.is_dir() {
        return Ok(());
    }

    Err(AppError::invalid_path(
        path.display().to_string(),
        format!("{field} is not an existing directory"),
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn validates_a_readonly_project() {
        let root = tempdir().expect("temp directory");
        let module = root.path().join("sample.mod");
        let install = root.path().join("install");
        let user = root.path().join("user");
        fs::write(&module, b"fixture").expect("module fixture");
        fs::create_dir(&install).expect("install directory");
        fs::create_dir(&user).expect("user directory");

        let project = ReadonlyProjectDraft {
            name: " Sample ".to_owned(),
            module_path: module.clone(),
            game_install_path: install,
            user_data_path: user,
            read_only: true,
        }
        .validate()
        .expect("valid project");

        assert_eq!(project.name, "Sample");
        assert_eq!(project.module_path, module);
        assert!(project.read_only);
    }

    #[test]
    fn rejects_a_writable_phase_one_project() {
        let root = tempdir().expect("temp directory");
        let draft = ReadonlyProjectDraft {
            name: "Writable".to_owned(),
            module_path: root.path().join("sample.mod"),
            game_install_path: root.path().to_path_buf(),
            user_data_path: root.path().to_path_buf(),
            read_only: false,
        };

        assert_eq!(
            draft.validate().expect_err("must reject writable").code,
            "PROJECT_PATH_INVALID"
        );
    }
}
