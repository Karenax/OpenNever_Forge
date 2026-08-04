use crate::ModuleFingerprint;
use crate::hashing::hash_existing_file;
use aurora_core::AppResult;
use aurora_gff::ModuleInfo;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyRoots {
    pub game_install_path: Option<PathBuf>,
    pub user_data_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleDependencyKind {
    Hak,
    CustomTlk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleDependencyState {
    Resolved,
    Missing,
    Unchecked,
    Invalid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleDependencyChange {
    FirstSeen,
    Unchanged,
    ContentChanged,
    LocationChanged,
    BecameAvailable,
    BecameMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDependency {
    pub kind: ModuleDependencyKind,
    pub logical_name: String,
    pub state: ModuleDependencyState,
    pub selected_path: Option<String>,
    pub shadowed_paths: Vec<String>,
    pub searched_paths: Vec<String>,
    pub fingerprint: Option<ModuleFingerprint>,
    pub change: ModuleDependencyChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDependencyReport {
    pub dependencies: Vec<ModuleDependency>,
    pub resolved_count: usize,
    pub missing_count: usize,
    pub unchecked_count: usize,
    pub invalid_count: usize,
    pub changed_count: usize,
}

pub fn inspect_module_dependencies(
    module_info: &ModuleInfo,
    roots: &DependencyRoots,
) -> ModuleDependencyReport {
    let mut dependencies = module_info
        .hak_files
        .iter()
        .map(|name| inspect_dependency(ModuleDependencyKind::Hak, name, roots))
        .collect::<Vec<_>>();

    if let Some(custom_tlk) = &module_info.custom_tlk {
        dependencies.push(inspect_dependency(
            ModuleDependencyKind::CustomTlk,
            custom_tlk,
            roots,
        ));
    }

    ModuleDependencyReport {
        resolved_count: count_state(&dependencies, ModuleDependencyState::Resolved),
        missing_count: count_state(&dependencies, ModuleDependencyState::Missing),
        unchecked_count: count_state(&dependencies, ModuleDependencyState::Unchecked),
        invalid_count: count_state(&dependencies, ModuleDependencyState::Invalid),
        changed_count: 0,
        dependencies,
    }
}

fn inspect_dependency(
    kind: ModuleDependencyKind,
    logical_name: &str,
    roots: &DependencyRoots,
) -> ModuleDependency {
    let logical_name = logical_name.trim().to_owned();
    let extension = match kind {
        ModuleDependencyKind::Hak => "hak",
        ModuleDependencyKind::CustomTlk => "tlk",
    };
    let Some(file_name) = safe_file_name(&logical_name, extension) else {
        return ModuleDependency {
            kind,
            logical_name,
            state: ModuleDependencyState::Invalid,
            selected_path: None,
            shadowed_paths: Vec::new(),
            searched_paths: Vec::new(),
            fingerprint: None,
            change: ModuleDependencyChange::FirstSeen,
        };
    };

    let searched = candidate_paths(kind, &file_name, roots);
    if searched.is_empty() {
        return ModuleDependency {
            kind,
            logical_name,
            state: ModuleDependencyState::Unchecked,
            selected_path: None,
            shadowed_paths: Vec::new(),
            searched_paths: Vec::new(),
            fingerprint: None,
            change: ModuleDependencyChange::FirstSeen,
        };
    }

    let matches = searched
        .iter()
        .filter(|path| path.is_file())
        .map(|path| display(path))
        .collect::<Vec<_>>();
    ModuleDependency {
        kind,
        logical_name,
        state: if matches.is_empty() {
            ModuleDependencyState::Missing
        } else {
            ModuleDependencyState::Resolved
        },
        selected_path: matches.first().cloned(),
        shadowed_paths: matches.into_iter().skip(1).collect(),
        searched_paths: searched.iter().map(|path| display(path)).collect(),
        fingerprint: None,
        change: ModuleDependencyChange::FirstSeen,
    }
}

pub fn fingerprint_module_dependencies(
    report: &mut ModuleDependencyReport,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    for dependency in &mut report.dependencies {
        let Some(path) = dependency.selected_path.as_deref() else {
            continue;
        };
        dependency.fingerprint = Some(hash_existing_file(Path::new(path), cancelled, |_| {})?);
    }
    Ok(())
}

pub fn compare_dependency_reports(
    current: &mut ModuleDependencyReport,
    previous: Option<&ModuleDependencyReport>,
) {
    for dependency in &mut current.dependencies {
        dependency.change =
            match previous.and_then(|report| matching_dependency(report, dependency)) {
                None => ModuleDependencyChange::FirstSeen,
                Some(previous) => compare_dependency(dependency, previous),
            };
    }
    current.changed_count = current
        .dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.change,
                ModuleDependencyChange::ContentChanged
                    | ModuleDependencyChange::LocationChanged
                    | ModuleDependencyChange::BecameAvailable
                    | ModuleDependencyChange::BecameMissing
            )
        })
        .count();
}

fn matching_dependency<'a>(
    report: &'a ModuleDependencyReport,
    dependency: &ModuleDependency,
) -> Option<&'a ModuleDependency> {
    report.dependencies.iter().find(|candidate| {
        candidate.kind == dependency.kind
            && candidate
                .logical_name
                .eq_ignore_ascii_case(&dependency.logical_name)
    })
}

fn compare_dependency(
    current: &ModuleDependency,
    previous: &ModuleDependency,
) -> ModuleDependencyChange {
    match (current.state, previous.state) {
        (ModuleDependencyState::Resolved, ModuleDependencyState::Resolved) => {
            if current.fingerprint != previous.fingerprint {
                ModuleDependencyChange::ContentChanged
            } else if current.selected_path != previous.selected_path {
                ModuleDependencyChange::LocationChanged
            } else {
                ModuleDependencyChange::Unchanged
            }
        }
        (ModuleDependencyState::Resolved, _) => ModuleDependencyChange::BecameAvailable,
        (ModuleDependencyState::Missing, ModuleDependencyState::Resolved) => {
            ModuleDependencyChange::BecameMissing
        }
        _ => ModuleDependencyChange::Unchanged,
    }
}

fn candidate_paths(
    kind: ModuleDependencyKind,
    file_name: &str,
    roots: &DependencyRoots,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(user_data) = &roots.user_data_path {
        let folder = match kind {
            ModuleDependencyKind::Hak => "hak",
            ModuleDependencyKind::CustomTlk => "tlk",
        };
        push_unique(&mut candidates, user_data.join(folder).join(file_name));
    }
    if let Some(game_install) = &roots.game_install_path {
        match kind {
            ModuleDependencyKind::Hak => {
                push_unique(
                    &mut candidates,
                    game_install.join("data").join("hk").join(file_name),
                );
                push_unique(&mut candidates, game_install.join("hak").join(file_name));
            }
            ModuleDependencyKind::CustomTlk => {
                push_unique(
                    &mut candidates,
                    game_install.join("data").join("tlk").join(file_name),
                );
                push_unique(&mut candidates, game_install.join("tlk").join(file_name));
            }
        }
    }
    candidates
}

fn safe_file_name(logical_name: &str, extension: &str) -> Option<String> {
    if logical_name.is_empty() || logical_name.contains(['/', '\\', ':']) {
        return None;
    }
    let path = Path::new(logical_name);
    if !matches!(
        path.components().collect::<Vec<_>>().as_slice(),
        [Component::Normal(_)]
    ) {
        return None;
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        Some(logical_name.to_owned())
    } else {
        Some(format!("{logical_name}.{extension}"))
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|path| path == &candidate) {
        paths.push(candidate);
    }
}

fn count_state(dependencies: &[ModuleDependency], state: ModuleDependencyState) -> usize {
    dependencies
        .iter()
        .filter(|dependency| dependency.state == state)
        .count()
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_gff::{LocalizedString, LocalizedValue};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_user_dependencies_before_game_dependencies() {
        let root = tempdir().expect("temporary directory");
        let user = root.path().join("user");
        let game = root.path().join("game");
        fs::create_dir_all(user.join("hak")).expect("user HAK directory");
        fs::create_dir_all(game.join("data").join("hk")).expect("game HAK directory");
        fs::create_dir_all(game.join("data").join("tlk")).expect("game TLK directory");
        fs::write(user.join("hak").join("shared.hak"), b"user").expect("user HAK");
        fs::write(game.join("data").join("hk").join("shared.hak"), b"game").expect("game HAK");
        fs::write(game.join("data").join("tlk").join("custom.tlk"), b"tlk").expect("custom TLK");

        let mut report = inspect_module_dependencies(
            &module_info(vec!["shared", "missing"], Some("custom")),
            &DependencyRoots {
                game_install_path: Some(game.clone()),
                user_data_path: Some(user.clone()),
            },
        );
        fingerprint_module_dependencies(&mut report, &AtomicBool::new(false))
            .expect("dependency fingerprints");

        assert_eq!(report.resolved_count, 2);
        assert_eq!(report.missing_count, 1);
        assert_eq!(
            report.dependencies[0].state,
            ModuleDependencyState::Resolved
        );
        assert_eq!(
            report.dependencies[0].selected_path.as_deref(),
            Some(display(&user.join("hak").join("shared.hak")).as_str())
        );
        assert_eq!(
            report.dependencies[0].shadowed_paths,
            vec![display(&game.join("data").join("hk").join("shared.hak"))]
        );
        assert_eq!(report.dependencies[1].state, ModuleDependencyState::Missing);
        assert_eq!(report.dependencies[2].kind, ModuleDependencyKind::CustomTlk);
        assert_eq!(
            report.dependencies[0]
                .fingerprint
                .as_ref()
                .map(|value| value.size_bytes),
            Some(4)
        );
        assert!(report.dependencies[1].fingerprint.is_none());
    }

    #[test]
    fn reports_unchecked_and_invalid_dependency_names_without_traversal() {
        let unchecked = inspect_module_dependencies(
            &module_info(vec!["safe"], None),
            &DependencyRoots::default(),
        );
        assert_eq!(unchecked.unchecked_count, 1);
        assert!(unchecked.dependencies[0].searched_paths.is_empty());

        let invalid = inspect_module_dependencies(
            &module_info(vec!["../escape"], None),
            &DependencyRoots {
                game_install_path: Some(PathBuf::from("game")),
                user_data_path: None,
            },
        );
        assert_eq!(invalid.invalid_count, 1);
        assert!(invalid.dependencies[0].searched_paths.is_empty());
    }

    #[test]
    fn detects_content_changes_and_disappearing_dependencies() {
        let root = tempdir().expect("temporary directory");
        let game = root.path().join("game");
        let hak_path = game.join("data").join("hk").join("changing.hak");
        fs::create_dir_all(hak_path.parent().expect("HAK parent")).expect("game HAK directory");
        fs::write(&hak_path, b"first").expect("initial HAK");
        let roots = DependencyRoots {
            game_install_path: Some(game),
            user_data_path: None,
        };

        let mut previous =
            inspect_module_dependencies(&module_info(vec!["changing"], None), &roots);
        fingerprint_module_dependencies(&mut previous, &AtomicBool::new(false))
            .expect("initial fingerprint");

        fs::write(&hak_path, b"second").expect("changed HAK");
        let mut changed = inspect_module_dependencies(&module_info(vec!["changing"], None), &roots);
        fingerprint_module_dependencies(&mut changed, &AtomicBool::new(false))
            .expect("changed fingerprint");
        compare_dependency_reports(&mut changed, Some(&previous));

        assert_eq!(changed.changed_count, 1);
        assert_eq!(
            changed.dependencies[0].change,
            ModuleDependencyChange::ContentChanged
        );

        fs::remove_file(&hak_path).expect("remove HAK");
        let mut missing = inspect_module_dependencies(&module_info(vec!["changing"], None), &roots);
        compare_dependency_reports(&mut missing, Some(&changed));

        assert_eq!(missing.changed_count, 1);
        assert_eq!(
            missing.dependencies[0].change,
            ModuleDependencyChange::BecameMissing
        );
    }

    fn module_info(hak_files: Vec<&str>, custom_tlk: Option<&str>) -> ModuleInfo {
        ModuleInfo {
            name: LocalizedString {
                string_ref: None,
                values: vec![LocalizedValue {
                    language_id: 0,
                    text: "Synthetic".to_owned(),
                }],
            },
            description: LocalizedString {
                string_ref: None,
                values: Vec::new(),
            },
            tag: "MODULE".to_owned(),
            minimum_game_version: Some("1.69".to_owned()),
            custom_tlk: custom_tlk.map(str::to_owned),
            entry_area: "start".to_owned(),
            hak_files: hak_files.into_iter().map(str::to_owned).collect(),
        }
    }
}
