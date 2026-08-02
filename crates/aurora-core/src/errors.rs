use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub user_message: String,
    pub technical_message: String,
    pub source: Option<String>,
    pub resource: Option<String>,
    pub import_stage: Option<String>,
    pub cause: Option<String>,
    pub severity: ErrorSeverity,
    pub suggestion: Option<String>,
}

impl AppError {
    pub fn new(
        code: impl Into<String>,
        user_message: impl Into<String>,
        technical_message: impl Into<String>,
        severity: ErrorSeverity,
    ) -> Self {
        Self {
            code: code.into(),
            user_message: user_message.into(),
            technical_message: technical_message.into(),
            source: None,
            resource: None,
            import_stage: None,
            cause: None,
            severity,
            suggestion: None,
        }
    }

    pub fn invalid_path(path: impl Into<String>, detail: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(
            "PROJECT_PATH_INVALID",
            "Le chemin sélectionné n'est pas valide.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(path)
        .with_suggestion("Sélectionnez un fichier ou un dossier accessible en lecture.")
    }

    pub fn module_not_found(path: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(
            "MODULE_NOT_FOUND",
            "Le module sélectionné est introuvable.",
            format!("No regular file exists at {path}"),
            ErrorSeverity::Error,
        )
        .with_source(path)
        .with_suggestion("Vérifiez le chemin du fichier .mod puis réessayez.")
    }

    pub fn job_cancelled(resource: impl Into<String>) -> Self {
        Self::new(
            "JOB_CANCELLED",
            "L'opération a été annulée.",
            "The background job observed its cancellation flag.",
            ErrorSeverity::Info,
        )
        .with_resource(resource)
    }

    pub fn io(operation: &str, path: impl Into<String>, error: &std::io::Error) -> Self {
        let path = path.into();
        Self::new(
            "FILE_IO_ERROR",
            "Le fichier n'a pas pu être lu.",
            format!("{operation} failed for {path}: {error}"),
            ErrorSeverity::Error,
        )
        .with_source(path)
        .with_cause(error.to_string())
    }

    pub fn database(path: impl Into<String>, detail: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(
            "DATABASE_INITIALIZATION_FAILED",
            "L'index local n'a pas pu être initialisé.",
            detail,
            ErrorSeverity::Fatal,
        )
        .with_source(path)
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    pub fn with_import_stage(mut self, stage: impl Into<String>) -> Self {
        self.import_stage = Some(stage.into());
        self
    }

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.technical_message)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, Box<AppError>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_error_keeps_stable_code_and_context() {
        let error = AppError::invalid_path("C:/missing.mod", "not a file")
            .with_resource("module")
            .with_import_stage("project_validation");

        assert_eq!(error.code, "PROJECT_PATH_INVALID");
        assert_eq!(error.source.as_deref(), Some("C:/missing.mod"));
        assert_eq!(error.resource.as_deref(), Some("module"));
        assert_eq!(error.import_stage.as_deref(), Some("project_validation"));
        assert_eq!(error.severity, ErrorSeverity::Error);
    }

    #[test]
    fn serializes_for_the_typescript_contract() {
        let error = AppError::invalid_path("C:/missing.mod", "not a file")
            .with_import_stage("project_validation");
        let json = serde_json::to_value(error).expect("serialize structured error");

        assert_eq!(
            json["userMessage"],
            "Le chemin sélectionné n'est pas valide."
        );
        assert_eq!(json["importStage"], "project_validation");
        assert!(json.get("user_message").is_none());
    }
}
