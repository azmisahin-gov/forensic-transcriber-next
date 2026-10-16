//! `ftn-storage` — SQLite persistence.
//!
//! The React layer never touches SQLite directly: it calls Tauri commands which
//! go through the application layer into this crate.

pub mod error;
pub mod schema;
pub mod store;

pub use error::StorageError;
pub use schema::{migrate, MigrationReport, SCHEMA_VERSION};
pub use store::SqliteStore;
