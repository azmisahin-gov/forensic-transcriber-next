//! Versioned schema migrations.
//!
//! Migrations are append-only: a released migration is never edited, only new
//! ones are added. The current version is tracked in SQLite's `user_version`
//! pragma and each migration runs inside a transaction so a failure leaves the
//! database at its previous, consistent version.

use rusqlite::Connection;

use crate::error::StorageError;

/// The schema version this build understands.
pub const SCHEMA_VERSION: i64 = 1;

/// An ordered migration step.
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// All migrations, in application order.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    sql: INITIAL_SCHEMA,
}];

/// The initial schema (version 1).
const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cases (
    case_id     TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    notes       TEXT NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evidence (
    evidence_id    TEXT PRIMARY KEY,
    case_id        TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    original_name  TEXT NOT NULL,
    stored_name    TEXT NOT NULL,
    original_path  TEXT NOT NULL,
    stored_path    TEXT NOT NULL,
    size_bytes     INTEGER NOT NULL,
    format         TEXT,
    codec          TEXT,
    duration_seconds REAL,
    sample_rate    INTEGER,
    channels       INTEGER,
    bit_depth      INTEGER,
    audio_stream_count INTEGER,
    source_sha256  TEXT NOT NULL,
    stored_sha256  TEXT NOT NULL,
    integrity      TEXT NOT NULL DEFAULT 'UNCHECKED',
    imported_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_evidence_case ON evidence(case_id);

CREATE TABLE IF NOT EXISTS derived_assets (
    derived_asset_id TEXT PRIMARY KEY,
    evidence_id      TEXT NOT NULL REFERENCES evidence(evidence_id) ON DELETE CASCADE,
    path             TEXT NOT NULL,
    kind             TEXT NOT NULL,
    format           TEXT NOT NULL,
    sample_rate      INTEGER NOT NULL,
    channels         INTEGER NOT NULL,
    sha256           TEXT NOT NULL,
    generator        TEXT NOT NULL,
    created_at       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_derived_evidence ON derived_assets(evidence_id);

CREATE TABLE IF NOT EXISTS transcripts (
    transcript_id TEXT PRIMARY KEY,
    case_id       TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    evidence_id   TEXT NOT NULL REFERENCES evidence(evidence_id) ON DELETE CASCADE,
    language      TEXT NOT NULL DEFAULT 'tr',
    engine        TEXT,
    model_id      TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    UNIQUE(case_id, evidence_id)
);

CREATE TABLE IF NOT EXISTS segments (
    segment_id    TEXT PRIMARY KEY,
    transcript_id TEXT NOT NULL REFERENCES transcripts(transcript_id) ON DELETE CASCADE,
    ordinal       INTEGER NOT NULL,
    start_ms      INTEGER NOT NULL,
    end_ms        INTEGER NOT NULL,
    speaker       TEXT NOT NULL DEFAULT 'SPEAKER_01',
    text          TEXT NOT NULL DEFAULT '',
    original_text TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'AUTOMATIC',
    confidence    REAL,
    flags_json    TEXT NOT NULL DEFAULT '[]',
    updated_at    INTEGER NOT NULL,
    CHECK (start_ms >= 0),
    CHECK (end_ms > start_ms)
);
CREATE INDEX IF NOT EXISTS idx_segments_transcript ON segments(transcript_id, ordinal);

CREATE TABLE IF NOT EXISTS transcript_revisions (
    revision_id   TEXT PRIMARY KEY,
    transcript_id TEXT NOT NULL REFERENCES transcripts(transcript_id) ON DELETE CASCADE,
    job_id        TEXT,
    state         TEXT NOT NULL,
    is_current    INTEGER NOT NULL DEFAULT 0,
    segments_json TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_revisions_transcript ON transcript_revisions(transcript_id, created_at);

CREATE TABLE IF NOT EXISTS transcription_jobs (
    job_id        TEXT PRIMARY KEY,
    evidence_id   TEXT NOT NULL REFERENCES evidence(evidence_id) ON DELETE CASCADE,
    transcript_id TEXT,
    status        TEXT NOT NULL,
    engine        TEXT,
    engine_version TEXT,
    model_id      TEXT,
    model_sha256  TEXT,
    settings_json TEXT,
    error_code    TEXT,
    error_message TEXT,
    started_at    INTEGER NOT NULL,
    finished_at   INTEGER
);
CREATE INDEX IF NOT EXISTS idx_jobs_evidence ON transcription_jobs(evidence_id, started_at);

CREATE TABLE IF NOT EXISTS audit_events (
    audit_event_id TEXT PRIMARY KEY,
    case_id        TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    evidence_id    TEXT,
    transcript_id  TEXT,
    segment_id     TEXT,
    action         TEXT NOT NULL,
    before_json    TEXT,
    after_json     TEXT,
    actor_label    TEXT,
    app_version    TEXT NOT NULL,
    created_at     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_case ON audit_events(case_id, created_at);

CREATE TABLE IF NOT EXISTS export_records (
    export_id      TEXT PRIMARY KEY,
    case_id        TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    transcript_id  TEXT,
    evidence_id    TEXT,
    format         TEXT NOT NULL,
    path           TEXT NOT NULL,
    sha256         TEXT,
    schema_version TEXT NOT NULL,
    created_at     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_exports_case ON export_records(case_id, created_at);

CREATE TABLE IF NOT EXISTS application_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;

/// The result of running migrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub from_version: i64,
    pub to_version: i64,
    pub applied: Vec<i64>,
}

/// Apply every pending migration. Foreign keys are enabled first so a migration
/// that violates them fails loudly rather than silently corrupting the data.
pub fn migrate(conn: &mut Connection) -> Result<MigrationReport, StorageError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let from_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if from_version > SCHEMA_VERSION {
        return Err(StorageError::SchemaTooNew {
            found: from_version,
            supported: SCHEMA_VERSION,
        });
    }

    let mut applied = Vec::new();
    for migration in MIGRATIONS {
        if migration.version <= from_version {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
        applied.push(migration.version);
    }

    Ok(MigrationReport {
        from_version,
        to_version: SCHEMA_VERSION,
        applied,
    })
}

/// Read the current schema version without migrating.
pub fn current_version(conn: &Connection) -> Result<i64, StorageError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let first = migrate(&mut conn).unwrap();
        assert_eq!(first.from_version, 0);
        assert_eq!(first.to_version, SCHEMA_VERSION);
        assert_eq!(first.applied, vec![1]);

        let second = migrate(&mut conn).unwrap();
        assert_eq!(second.from_version, SCHEMA_VERSION);
        assert!(second.applied.is_empty());
    }

    #[test]
    fn detects_future_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 5)
            .unwrap();
        let err = migrate(&mut conn).unwrap_err();
        assert!(matches!(err, StorageError::SchemaTooNew { .. }));
    }
}
