use crate::model::{
    MigrationDiagnostic, MigrationDiagnosticSeverity, MigrationPhase, MigrationStatus,
};
use aurora_world::{DiagnosticSeverity, WorldDiagnostic};

#[derive(Debug, Default)]
pub(crate) struct DiagnosticCollector {
    items: Vec<MigrationDiagnostic>,
}

impl DiagnosticCollector {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push(
        &mut self,
        severity: MigrationDiagnosticSeverity,
        status: MigrationStatus,
        phase: MigrationPhase,
        code: impl Into<String>,
        message: impl Into<String>,
        resource: Option<String>,
        identity: Option<String>,
    ) {
        self.items.push(MigrationDiagnostic {
            sequence: 0,
            severity,
            status,
            phase,
            code: code.into(),
            message: message.into(),
            resource: resource.map(|value| portable_resource_label(&value)),
            identity,
        });
    }

    pub(crate) fn extend_world(&mut self, diagnostics: &[WorldDiagnostic]) {
        for diagnostic in diagnostics {
            let severity = match diagnostic.severity {
                DiagnosticSeverity::Info => MigrationDiagnosticSeverity::Info,
                DiagnosticSeverity::Warning => MigrationDiagnosticSeverity::Warning,
                DiagnosticSeverity::Error => MigrationDiagnosticSeverity::Error,
            };
            self.push(
                severity,
                status_for_diagnostic(&diagnostic.code, severity),
                MigrationPhase::Audit,
                &diagnostic.code,
                &diagnostic.message,
                Some(diagnostic.resource.clone()),
                None,
            );
        }
    }

    pub(crate) fn into_sorted(mut self) -> Vec<MigrationDiagnostic> {
        self.items.sort_by(|left, right| {
            (
                left.phase,
                left.severity,
                &left.code,
                &left.resource,
                &left.identity,
                &left.message,
            )
                .cmp(&(
                    right.phase,
                    right.severity,
                    &right.code,
                    &right.resource,
                    &right.identity,
                    &right.message,
                ))
        });
        self.items.dedup_by(|left, right| {
            left.phase == right.phase
                && left.severity == right.severity
                && left.status == right.status
                && left.code == right.code
                && left.resource == right.resource
                && left.identity == right.identity
                && left.message == right.message
        });
        for (sequence, diagnostic) in self.items.iter_mut().enumerate() {
            diagnostic.sequence = sequence + 1;
        }
        self.items
    }
}

/// Single semantic mapping from world/resource diagnostics to the v1 disposition vocabulary.
/// Severity alone is not enough: a missing GIT is commonly reported as a warning by the reader,
/// while it must still make the migration incomplete and block publication.
pub(crate) fn status_for_diagnostic(
    code: &str,
    severity: MigrationDiagnosticSeverity,
) -> MigrationStatus {
    let normalized = code.to_ascii_uppercase();
    if normalized.contains("LICENSE") || normalized.contains("LICENCE") {
        return MigrationStatus::LicenseBlocked;
    }
    if normalized.contains("UNSUPPORTED")
        || normalized.contains("FORMAT")
        || normalized.contains("CONVERSION")
        || normalized.contains("NOT_CONVERTIBLE")
    {
        return MigrationStatus::Unsupported;
    }
    if normalized.contains("MISSING")
        || normalized.ends_with("_NOT_FOUND")
        || normalized.contains("UNRESOLVED")
        || normalized.contains("BROKEN")
        || normalized.contains("ABSENT")
    {
        return MigrationStatus::Missing;
    }
    if severity == MigrationDiagnosticSeverity::Error {
        return MigrationStatus::Missing;
    }
    MigrationStatus::Manual
}

fn portable_resource_label(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    if let Some((container, locator)) = normalized.rsplit_once("::") {
        let logical_container = container.rsplit("::").next().unwrap_or(container);
        let file_name = logical_container
            .rsplit('/')
            .next()
            .unwrap_or(logical_container);
        return format!("{file_name}::{locator}");
    }
    if normalized.contains('/') {
        return normalized.rsplit('/').next().unwrap_or_default().to_owned();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_resources_never_expose_local_source_paths() {
        assert_eq!(
            portable_resource_label(r"C:\private\modules\sample.mod::area_a.gic::Door List[2]"),
            "area_a.gic::Door List[2]"
        );
        assert_eq!(
            portable_resource_label(r"C:\private\modules\area_a.are"),
            "area_a.are"
        );
        assert_eq!(portable_resource_label("tile_a.mdl"), "tile_a.mdl");
    }

    #[test]
    fn warning_level_missing_world_diagnostics_are_still_missing() {
        assert_eq!(
            status_for_diagnostic("AREA_GIT_NOT_FOUND", MigrationDiagnosticSeverity::Warning),
            MigrationStatus::Missing
        );
        assert_eq!(
            status_for_diagnostic(
                "INSTANCE_MODEL_UNRESOLVED",
                MigrationDiagnosticSeverity::Warning
            ),
            MigrationStatus::Missing
        );
        assert_eq!(
            status_for_diagnostic(
                "MODEL_FORMAT_UNSUPPORTED",
                MigrationDiagnosticSeverity::Warning
            ),
            MigrationStatus::Unsupported
        );
    }
}
