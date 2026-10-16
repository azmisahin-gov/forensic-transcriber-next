//! `ftn-domain` — the pure core of Forensic Transcriber Next.
//!
//! This crate contains entities and business rules only. It has no dependency
//! on SQLite, FFmpeg, whisper.cpp, the UI or the operating system. That keeps
//! the rules testable in isolation and prevents a second copy of them appearing
//! in the frontend.

pub mod audit;
pub mod case;
pub mod error;
pub mod evidence;
pub mod export_record;
pub mod ids;
pub mod job;
pub mod revision;
pub mod settings;
pub mod time;
pub mod transcript;

pub use error::DomainError;
pub use time::Timestamp;

/// The application version reported in audit and export records. Kept in one
/// place so every layer reports the same value.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
