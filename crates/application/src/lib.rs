//! `ftn-application` — use cases orchestrating the domain, storage, media and
//! transcription layers.

pub mod error;
pub mod paths;
pub mod workspace;

pub use error::AppError;
pub use paths::WorkspacePaths;
pub use workspace::{
    cancel_token, execute_transcription, TranscriptionOutcome, TranscriptionPlan, Workspace,
};
