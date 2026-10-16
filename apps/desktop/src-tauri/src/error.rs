//! The error type crossing the IPC boundary. It always carries a stable code
//! and a human-readable message; it never carries a stack trace or file content.

use ftn_application::AppError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
        }
    }
}

impl From<ftn_domain::DomainError> for CommandError {
    fn from(error: ftn_domain::DomainError) -> Self {
        Self {
            code: "DOMAIN_ERROR".to_string(),
            message: error.to_string(),
        }
    }
}

impl From<std::sync::PoisonError<std::sync::MutexGuard<'_, ftn_application::Workspace>>>
    for CommandError
{
    fn from(
        _: std::sync::PoisonError<std::sync::MutexGuard<'_, ftn_application::Workspace>>,
    ) -> Self {
        Self::new(
            "LOCK_POISONED",
            "İç durum kilidi bozuldu; uygulamayı yeniden başlatın.",
        )
    }
}
