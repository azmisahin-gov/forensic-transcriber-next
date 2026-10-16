//! The SQLite-backed store. All SQL lives here; every statement is parameterised.

use std::path::{Path, PathBuf};

use ftn_domain::audit::AuditEvent;
use ftn_domain::case::Case;
use ftn_domain::evidence::{
    DerivedKind, DerivedMediaAsset, EvidenceFile, IntegrityStatus, MediaMetadata, Sha256,
};
use ftn_domain::export_record::{ExportFormat, ExportRecord};
use ftn_domain::ids::{
    AuditEventId, CaseId, DerivedAssetId, EvidenceId, ExportId, JobId, RevisionId, SegmentId,
    TranscriptId,
};
use ftn_domain::job::{EngineInfo, JobStatus, TranscriptionJob};
use ftn_domain::revision::{RevisionState, TranscriptRevision};
use ftn_domain::settings::ApplicationSetting;
use ftn_domain::time::Timestamp;
use ftn_domain::transcript::{SegmentStatus, SpeakerLabel, Transcript, TranscriptSegment};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::StorageError;
use crate::schema;

/// A handle to the case database. Not `Sync`; callers serialise access.
pub struct SqliteStore {
    conn: Connection,
    path: Option<PathBuf>,
}

impl SqliteStore {
    /// Open (creating if needed) a database file and run migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut conn = Connection::open(&path)?;
        configure(&conn)?;
        schema::migrate(&mut conn)?;
        Ok(Self {
            conn,
            path: Some(path),
        })
    }

    /// Open an in-memory database (used by tests).
    pub fn in_memory() -> Result<Self, StorageError> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn, path: None })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        schema::current_version(&self.conn)
    }

    /// Run SQLite's integrity check. Returns `Ok(())` when the database is
    /// structurally sound.
    pub fn integrity_check(&self) -> Result<(), StorageError> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(StorageError::NotFound {
                entity: "integrity_check",
                id: result,
            })
        }
    }

    // ---- cases -----------------------------------------------------------

    pub fn create_case(&self, case: &Case) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO cases (case_id, title, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                case.id.as_str(),
                case.title,
                case.notes,
                case.created_at.as_millis(),
                case.updated_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn get_case(&self, id: &CaseId) -> Result<Case, StorageError> {
        self.conn
            .query_row(
                "SELECT case_id, title, notes, created_at, updated_at FROM cases WHERE case_id = ?1",
                params![id.as_str()],
                map_case,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound {
                entity: "case",
                id: id.to_string(),
            })
    }

    pub fn list_cases(&self) -> Result<Vec<Case>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT case_id, title, notes, created_at, updated_at FROM cases ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_case)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn update_case(&self, case: &Case) -> Result<(), StorageError> {
        let n = self.conn.execute(
            "UPDATE cases SET title = ?2, notes = ?3, updated_at = ?4 WHERE case_id = ?1",
            params![
                case.id.as_str(),
                case.title,
                case.notes,
                case.updated_at.as_millis(),
            ],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound {
                entity: "case",
                id: case.id.to_string(),
            });
        }
        Ok(())
    }

    pub fn delete_case(&self, id: &CaseId) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM cases WHERE case_id = ?1", params![id.as_str()])?;
        Ok(())
    }

    pub fn count_cases(&self) -> Result<i64, StorageError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM cases", [], |row| row.get(0))?)
    }

    // ---- evidence --------------------------------------------------------

    /// Insert evidence bound to its owning case.
    pub fn insert_evidence_for_case(
        &self,
        case_id: &CaseId,
        e: &EvidenceFile,
    ) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO evidence (
                evidence_id, case_id, original_name, stored_name, original_path, stored_path,
                size_bytes, format, codec, duration_seconds, sample_rate, channels, bit_depth,
                audio_stream_count, source_sha256, stored_sha256, integrity, imported_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                e.id.as_str(),
                case_id.as_str(),
                e.original_name,
                e.stored_name,
                e.original_path,
                e.stored_path,
                e.size_bytes as i64,
                e.metadata.format,
                e.metadata.codec,
                e.metadata.duration_seconds,
                e.metadata.sample_rate.map(|v| v as i64),
                e.metadata.channels.map(|v| v as i64),
                e.metadata.bit_depth.map(|v| v as i64),
                e.metadata.audio_stream_count.map(|v| v as i64),
                e.source_sha256.as_str(),
                e.stored_sha256.as_str(),
                integrity_str(e.integrity),
                e.imported_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn get_evidence(&self, id: &EvidenceId) -> Result<EvidenceFile, StorageError> {
        self.conn
            .query_row(
                "SELECT evidence_id, original_name, stored_name, original_path, stored_path,
                        size_bytes, format, codec, duration_seconds, sample_rate, channels,
                        bit_depth, audio_stream_count, source_sha256, stored_sha256, integrity,
                        imported_at, case_id
                 FROM evidence WHERE evidence_id = ?1",
                params![id.as_str()],
                map_evidence,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound {
                entity: "evidence",
                id: id.to_string(),
            })
    }

    pub fn list_evidence(&self, case_id: &CaseId) -> Result<Vec<EvidenceFile>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT evidence_id, original_name, stored_name, original_path, stored_path,
                    size_bytes, format, codec, duration_seconds, sample_rate, channels,
                    bit_depth, audio_stream_count, source_sha256, stored_sha256, integrity,
                    imported_at, case_id
             FROM evidence WHERE case_id = ?1 ORDER BY imported_at",
        )?;
        let rows = stmt.query_map(params![case_id.as_str()], map_evidence)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Record the outcome of an integrity re-check.
    pub fn set_evidence_integrity(
        &self,
        id: &EvidenceId,
        status: IntegrityStatus,
        stored_sha256: &Sha256,
    ) -> Result<(), StorageError> {
        let n = self.conn.execute(
            "UPDATE evidence SET integrity = ?2, stored_sha256 = ?3 WHERE evidence_id = ?1",
            params![id.as_str(), integrity_str(status), stored_sha256.as_str()],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound {
                entity: "evidence",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    // ---- derived assets --------------------------------------------------

    pub fn insert_derived_asset(&self, a: &DerivedMediaAsset) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO derived_assets (
                derived_asset_id, evidence_id, path, kind, format, sample_rate, channels,
                sha256, generator, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                a.id.as_str(),
                a.evidence_id.as_str(),
                a.path,
                derived_kind_str(a.kind),
                a.format,
                a.sample_rate as i64,
                a.channels as i64,
                a.sha256.as_str(),
                a.generator,
                a.created_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn list_derived_assets(
        &self,
        evidence_id: &EvidenceId,
    ) -> Result<Vec<DerivedMediaAsset>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT derived_asset_id, evidence_id, path, kind, format, sample_rate, channels,
                    sha256, generator, created_at
             FROM derived_assets WHERE evidence_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![evidence_id.as_str()], map_derived)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- transcripts & segments -----------------------------------------

    pub fn create_transcript(&self, case_id: &CaseId, t: &Transcript) -> Result<(), StorageError> {
        let exists: Option<String> = self
            .conn
            .query_row(
                "SELECT transcript_id FROM transcripts WHERE case_id = ?1 AND evidence_id = ?2",
                params![case_id.as_str(), t.evidence_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Err(StorageError::TranscriptExists(t.evidence_id.to_string()));
        }
        self.conn.execute(
            "INSERT INTO transcripts (transcript_id, case_id, evidence_id, language, engine, model_id, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                t.id.as_str(),
                case_id.as_str(),
                t.evidence_id.as_str(),
                t.language,
                t.engine,
                t.model_id,
                t.created_at.as_millis(),
                t.updated_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn get_transcript(&self, id: &TranscriptId) -> Result<Transcript, StorageError> {
        self.conn
            .query_row(
                "SELECT transcript_id, evidence_id, language, engine, model_id, created_at, updated_at
                 FROM transcripts WHERE transcript_id = ?1",
                params![id.as_str()],
                map_transcript,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound {
                entity: "transcript",
                id: id.to_string(),
            })
    }

    pub fn get_transcript_by_evidence(
        &self,
        evidence_id: &EvidenceId,
    ) -> Result<Option<Transcript>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT transcript_id, evidence_id, language, engine, model_id, created_at, updated_at
                 FROM transcripts WHERE evidence_id = ?1",
                params![evidence_id.as_str()],
                map_transcript,
            )
            .optional()?)
    }

    pub fn list_transcripts_for_case(
        &self,
        case_id: &CaseId,
    ) -> Result<Vec<Transcript>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT transcript_id, evidence_id, language, engine, model_id, created_at, updated_at
             FROM transcripts WHERE case_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![case_id.as_str()], map_transcript)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Replace all segments of a transcript inside a transaction.
    pub fn replace_segments(
        &mut self,
        transcript_id: &TranscriptId,
        segments: &[TranscriptSegment],
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM segments WHERE transcript_id = ?1",
            params![transcript_id.as_str()],
        )?;
        for s in segments {
            insert_segment(&tx, s)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Update a single segment's editable fields and bump the transcript's
    /// `updated_at`. Returns the previous segment so the caller can record an
    /// audit diff.
    pub fn update_segment(
        &mut self,
        segment: &TranscriptSegment,
    ) -> Result<TranscriptSegment, StorageError> {
        let tx = self.conn.transaction()?;
        let previous = tx
            .query_row(
                "SELECT segment_id, transcript_id, ordinal, start_ms, end_ms, speaker, text,
                        original_text, status, confidence, flags_json, updated_at
                 FROM segments WHERE segment_id = ?1",
                params![segment.id.as_str()],
                map_segment,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound {
                entity: "segment",
                id: segment.id.to_string(),
            })?;

        tx.execute(
            "UPDATE segments SET ordinal=?2, start_ms=?3, end_ms=?4, speaker=?5, text=?6,
                    original_text=?7, status=?8, confidence=?9, flags_json=?10, updated_at=?11
             WHERE segment_id=?1",
            params![
                segment.id.as_str(),
                segment.ordinal as i64,
                segment.start_ms,
                segment.end_ms,
                segment.speaker.as_str(),
                segment.text,
                segment.original_text,
                segment.status.as_str(),
                segment.confidence.map(|c| c as f64),
                serde_json::to_string(&segment.flags)?,
                segment.updated_at.as_millis(),
            ],
        )?;
        tx.execute(
            "UPDATE transcripts SET updated_at = ?2 WHERE transcript_id = ?1",
            params![
                segment.transcript_id.as_str(),
                segment.updated_at.as_millis()
            ],
        )?;
        tx.commit()?;
        Ok(previous)
    }

    pub fn list_segments(
        &self,
        transcript_id: &TranscriptId,
    ) -> Result<Vec<TranscriptSegment>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT segment_id, transcript_id, ordinal, start_ms, end_ms, speaker, text,
                    original_text, status, confidence, flags_json, updated_at
             FROM segments WHERE transcript_id = ?1 ORDER BY ordinal, start_ms",
        )?;
        let rows = stmt.query_map(params![transcript_id.as_str()], map_segment)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn count_segments(&self, transcript_id: &TranscriptId) -> Result<i64, StorageError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM segments WHERE transcript_id = ?1",
            params![transcript_id.as_str()],
            |row| row.get(0),
        )?)
    }

    pub fn get_segment(&self, id: &SegmentId) -> Result<TranscriptSegment, StorageError> {
        self.conn
            .query_row(
                "SELECT segment_id, transcript_id, ordinal, start_ms, end_ms, speaker, text,
                        original_text, status, confidence, flags_json, updated_at
                 FROM segments WHERE segment_id = ?1",
                params![id.as_str()],
                map_segment,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound {
                entity: "segment",
                id: id.to_string(),
            })
    }

    // ---- revisions -------------------------------------------------------

    /// Append a revision and make it the current one, demoting the previous
    /// current revision. Runs in a transaction so exactly one revision is ever
    /// current.
    pub fn append_revision(&mut self, revision: &TranscriptRevision) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE transcript_revisions SET is_current = 0 WHERE transcript_id = ?1",
            params![revision.transcript_id.as_str()],
        )?;
        tx.execute(
            "INSERT INTO transcript_revisions (revision_id, transcript_id, job_id, state, is_current, segments_json, created_at)
             VALUES (?1,?2,?3,?4,1,?5,?6)",
            params![
                revision.id.as_str(),
                revision.transcript_id.as_str(),
                revision.job_id.as_ref().map(|j| j.to_string()),
                revision.state.as_str(),
                serde_json::to_string(&revision.segments)?,
                revision.created_at.as_millis(),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_revisions(
        &self,
        transcript_id: &TranscriptId,
    ) -> Result<Vec<TranscriptRevision>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT revision_id, transcript_id, job_id, state, is_current, segments_json, created_at
             FROM transcript_revisions WHERE transcript_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![transcript_id.as_str()], map_revision)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- jobs ------------------------------------------------------------

    pub fn insert_job(&self, job: &TranscriptionJob) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO transcription_jobs (
                job_id, evidence_id, transcript_id, status, engine, engine_version, model_id,
                model_sha256, settings_json, error_code, error_message, started_at, finished_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                job.id.as_str(),
                job.evidence_id.as_str(),
                job.transcript_id.as_ref().map(|t| t.to_string()),
                job.status.as_str(),
                job.engine.as_ref().map(|e| e.engine.clone()),
                job.engine.as_ref().and_then(|e| e.engine_version.clone()),
                job.engine.as_ref().and_then(|e| e.model_id.clone()),
                job.engine.as_ref().and_then(|e| e.model_sha256.clone()),
                job.engine
                    .as_ref()
                    .map(|e| serde_json::to_string(&e.settings))
                    .transpose()?,
                job.error_code,
                job.error_message,
                job.started_at.as_millis(),
                job.finished_at.map(|t| t.as_millis()),
            ],
        )?;
        Ok(())
    }

    pub fn update_job(&self, job: &TranscriptionJob) -> Result<(), StorageError> {
        let n = self.conn.execute(
            "UPDATE transcription_jobs SET transcript_id=?2, status=?3, error_code=?4,
                    error_message=?5, finished_at=?6 WHERE job_id=?1",
            params![
                job.id.as_str(),
                job.transcript_id.as_ref().map(|t| t.to_string()),
                job.status.as_str(),
                job.error_code,
                job.error_message,
                job.finished_at.map(|t| t.as_millis()),
            ],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound {
                entity: "job",
                id: job.id.to_string(),
            });
        }
        Ok(())
    }

    pub fn list_jobs(
        &self,
        evidence_id: &EvidenceId,
    ) -> Result<Vec<TranscriptionJob>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT job_id, evidence_id, transcript_id, status, engine, engine_version, model_id,
                    model_sha256, settings_json, error_code, error_message, started_at, finished_at
             FROM transcription_jobs WHERE evidence_id = ?1 ORDER BY started_at",
        )?;
        let rows = stmt.query_map(params![evidence_id.as_str()], map_job)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- audit -----------------------------------------------------------

    pub fn append_audit(&self, event: &AuditEvent) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO audit_events (
                audit_event_id, case_id, evidence_id, transcript_id, segment_id, action,
                before_json, after_json, actor_label, app_version, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                event.id.as_str(),
                event.case_id.as_str(),
                event.evidence_id.as_ref().map(|e| e.to_string()),
                event.transcript_id.as_ref().map(|t| t.to_string()),
                event.segment_id.as_ref().map(|s| s.to_string()),
                event.action.as_str(),
                event.before.as_ref().map(|v| v.to_string()),
                event.after.as_ref().map(|v| v.to_string()),
                event.actor_label,
                event.app_version,
                event.created_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn list_audit(&self, case_id: &CaseId) -> Result<Vec<AuditEvent>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT audit_event_id, case_id, evidence_id, transcript_id, segment_id, action,
                    before_json, after_json, actor_label, app_version, created_at
             FROM audit_events WHERE case_id = ?1 ORDER BY created_at, audit_event_id",
        )?;
        let rows = stmt.query_map(params![case_id.as_str()], map_audit)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- exports ---------------------------------------------------------

    pub fn insert_export(&self, record: &ExportRecord) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO export_records (export_id, case_id, transcript_id, evidence_id, format, path, sha256, schema_version, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                record.id.as_str(),
                record.case_id.as_str(),
                record.transcript_id.as_ref().map(|t| t.to_string()),
                record.evidence_id.as_ref().map(|e| e.to_string()),
                record.format.as_str(),
                record.path,
                record.sha256,
                record.schema_version,
                record.created_at.as_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn list_exports(&self, case_id: &CaseId) -> Result<Vec<ExportRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT export_id, case_id, transcript_id, evidence_id, format, path, sha256, schema_version, created_at
             FROM export_records WHERE case_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![case_id.as_str()], map_export)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- settings --------------------------------------------------------

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO application_settings (key, value, updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, Timestamp::now().as_millis()],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM application_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn all_settings(&self) -> Result<Vec<ApplicationSetting>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value, updated_at FROM application_settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok(ApplicationSetting {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: Timestamp::from_millis(row.get(2)?),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

fn configure(conn: &Connection) -> Result<(), StorageError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    Ok(())
}

fn insert_segment(conn: &Connection, s: &TranscriptSegment) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO segments (segment_id, transcript_id, ordinal, start_ms, end_ms, speaker,
                text, original_text, status, confidence, flags_json, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            s.id.as_str(),
            s.transcript_id.as_str(),
            s.ordinal as i64,
            s.start_ms,
            s.end_ms,
            s.speaker.as_str(),
            s.text,
            s.original_text,
            s.status.as_str(),
            s.confidence.map(|c| c as f64),
            serde_json::to_string(&s.flags)?,
            s.updated_at.as_millis(),
        ],
    )?;
    Ok(())
}

// ---- row mappers --------------------------------------------------------

fn map_case(row: &Row) -> rusqlite::Result<Case> {
    Ok(Case {
        id: CaseId::from_string(row.get::<_, String>(0)?),
        title: row.get(1)?,
        notes: row.get(2)?,
        created_at: Timestamp::from_millis(row.get(3)?),
        updated_at: Timestamp::from_millis(row.get(4)?),
    })
}

fn sha_from_column(value: String) -> rusqlite::Result<Sha256> {
    Sha256::parse(value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn map_evidence(row: &Row) -> rusqlite::Result<EvidenceFile> {
    Ok(EvidenceFile {
        id: EvidenceId::from_string(row.get::<_, String>(0)?),
        case_id: CaseId::from_string(row.get::<_, String>(17)?),
        original_name: row.get(1)?,
        stored_name: row.get(2)?,
        original_path: row.get(3)?,
        stored_path: row.get(4)?,
        size_bytes: row.get::<_, i64>(5)? as u64,
        metadata: MediaMetadata {
            format: row.get(6)?,
            codec: row.get(7)?,
            duration_seconds: row.get(8)?,
            sample_rate: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
            channels: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
            bit_depth: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
            audio_stream_count: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
        },
        source_sha256: sha_from_column(row.get(13)?)?,
        stored_sha256: sha_from_column(row.get(14)?)?,
        integrity: parse_integrity(&row.get::<_, String>(15)?),
        imported_at: Timestamp::from_millis(row.get(16)?),
    })
}

fn map_derived(row: &Row) -> rusqlite::Result<DerivedMediaAsset> {
    Ok(DerivedMediaAsset {
        id: DerivedAssetId::from_string(row.get::<_, String>(0)?),
        evidence_id: EvidenceId::from_string(row.get::<_, String>(1)?),
        path: row.get(2)?,
        kind: if row.get::<_, String>(3)? == "WAVEFORM" {
            DerivedKind::Waveform
        } else {
            DerivedKind::TranscriptionInput
        },
        format: row.get(4)?,
        sample_rate: row.get::<_, i64>(5)? as u32,
        channels: row.get::<_, i64>(6)? as u16,
        sha256: sha_from_column(row.get(7)?)?,
        generator: row.get(8)?,
        created_at: Timestamp::from_millis(row.get(9)?),
    })
}

fn map_transcript(row: &Row) -> rusqlite::Result<Transcript> {
    Ok(Transcript {
        id: TranscriptId::from_string(row.get::<_, String>(0)?),
        evidence_id: EvidenceId::from_string(row.get::<_, String>(1)?),
        language: row.get(2)?,
        engine: row.get(3)?,
        model_id: row.get(4)?,
        created_at: Timestamp::from_millis(row.get(5)?),
        updated_at: Timestamp::from_millis(row.get(6)?),
    })
}

fn map_segment(row: &Row) -> rusqlite::Result<TranscriptSegment> {
    let flags_json: String = row.get(10)?;
    let flags: Vec<String> = serde_json::from_str(&flags_json).unwrap_or_default();
    let status =
        SegmentStatus::parse(&row.get::<_, String>(8)?).unwrap_or(SegmentStatus::Automatic);
    Ok(TranscriptSegment {
        id: SegmentId::from_string(row.get::<_, String>(0)?),
        transcript_id: TranscriptId::from_string(row.get::<_, String>(1)?),
        ordinal: row.get::<_, i64>(2)? as u32,
        start_ms: row.get(3)?,
        end_ms: row.get(4)?,
        speaker: SpeakerLabel::parse(row.get::<_, String>(5)?)
            .unwrap_or_else(|_| SpeakerLabel::default_speaker()),
        text: row.get(6)?,
        original_text: row.get(7)?,
        status,
        confidence: row.get::<_, Option<f64>>(9)?.map(|v| v as f32),
        flags,
        updated_at: Timestamp::from_millis(row.get(11)?),
    })
}

fn map_revision(row: &Row) -> rusqlite::Result<TranscriptRevision> {
    let segments_json: String = row.get(5)?;
    let segments: Vec<TranscriptSegment> = serde_json::from_str(&segments_json).unwrap_or_default();
    let state = RevisionState::parse(&row.get::<_, String>(3)?).unwrap_or(RevisionState::Machine);
    Ok(TranscriptRevision {
        id: RevisionId::from_string(row.get::<_, String>(0)?),
        transcript_id: TranscriptId::from_string(row.get::<_, String>(1)?),
        job_id: row.get::<_, Option<String>>(2)?.map(JobId::from_string),
        state,
        is_current: row.get::<_, i64>(4)? != 0,
        segments,
        created_at: Timestamp::from_millis(row.get(6)?),
    })
}

fn map_job(row: &Row) -> rusqlite::Result<TranscriptionJob> {
    let engine: Option<String> = row.get(4)?;
    let engine_info = engine.map(|engine| {
        let settings_json: Option<String> = row.get(8).ok().flatten();
        EngineInfo {
            engine,
            engine_version: row.get(5).ok().flatten(),
            model_id: row.get(6).ok().flatten(),
            model_sha256: row.get(7).ok().flatten(),
            settings: settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null),
        }
    });
    Ok(TranscriptionJob {
        id: JobId::from_string(row.get::<_, String>(0)?),
        evidence_id: EvidenceId::from_string(row.get::<_, String>(1)?),
        transcript_id: row
            .get::<_, Option<String>>(2)?
            .map(TranscriptId::from_string),
        status: JobStatus::parse(&row.get::<_, String>(3)?).unwrap_or(JobStatus::Failed),
        engine: engine_info,
        error_code: row.get(9)?,
        error_message: row.get(10)?,
        started_at: Timestamp::from_millis(row.get(11)?),
        finished_at: row.get::<_, Option<i64>>(12)?.map(Timestamp::from_millis),
    })
}

fn map_audit(row: &Row) -> rusqlite::Result<AuditEvent> {
    let action_str: String = row.get(5)?;
    let action = parse_audit_action(&action_str);
    Ok(AuditEvent {
        id: AuditEventId::from_string(row.get::<_, String>(0)?),
        case_id: CaseId::from_string(row.get::<_, String>(1)?),
        evidence_id: row
            .get::<_, Option<String>>(2)?
            .map(EvidenceId::from_string),
        transcript_id: row
            .get::<_, Option<String>>(3)?
            .map(TranscriptId::from_string),
        segment_id: row.get::<_, Option<String>>(4)?.map(SegmentId::from_string),
        action,
        before: row
            .get::<_, Option<String>>(6)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        after: row
            .get::<_, Option<String>>(7)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        actor_label: row.get(8)?,
        app_version: row.get(9)?,
        created_at: Timestamp::from_millis(row.get(10)?),
    })
}

fn map_export(row: &Row) -> rusqlite::Result<ExportRecord> {
    let fmt = ExportFormat::parse(&row.get::<_, String>(4)?).unwrap_or(ExportFormat::Json);
    Ok(ExportRecord {
        id: ExportId::from_string(row.get::<_, String>(0)?),
        case_id: CaseId::from_string(row.get::<_, String>(1)?),
        transcript_id: row
            .get::<_, Option<String>>(2)?
            .map(TranscriptId::from_string),
        evidence_id: row
            .get::<_, Option<String>>(3)?
            .map(EvidenceId::from_string),
        format: fmt,
        path: row.get(5)?,
        sha256: row.get(6)?,
        schema_version: row.get(7)?,
        created_at: Timestamp::from_millis(row.get(8)?),
    })
}

fn integrity_str(status: IntegrityStatus) -> &'static str {
    match status {
        IntegrityStatus::Verified => "VERIFIED",
        IntegrityStatus::Mismatch => "MISMATCH",
        IntegrityStatus::Missing => "MISSING",
        IntegrityStatus::Unchecked => "UNCHECKED",
    }
}

fn parse_integrity(value: &str) -> IntegrityStatus {
    match value {
        "VERIFIED" => IntegrityStatus::Verified,
        "MISMATCH" => IntegrityStatus::Mismatch,
        "MISSING" => IntegrityStatus::Missing,
        _ => IntegrityStatus::Unchecked,
    }
}

fn derived_kind_str(kind: DerivedKind) -> &'static str {
    match kind {
        DerivedKind::TranscriptionInput => "TRANSCRIPTION_INPUT",
        DerivedKind::Waveform => "WAVEFORM",
    }
}

fn parse_audit_action(value: &str) -> ftn_domain::audit::AuditAction {
    use ftn_domain::audit::AuditAction::*;
    match value {
        "CASE_CREATED" => CaseCreated,
        "CASE_RENAMED" => CaseRenamed,
        "EVIDENCE_IMPORTED" => EvidenceImported,
        "EVIDENCE_INTEGRITY_CHECKED" => EvidenceIntegrityChecked,
        "TRANSCRIBE_STARTED" => TranscribeStarted,
        "TRANSCRIBE_SUCCEEDED" => TranscribeSucceeded,
        "TRANSCRIBE_FAILED" => TranscribeFailed,
        "TRANSCRIBE_CANCELLED" => TranscribeCancelled,
        "SEGMENT_TEXT_EDITED" => SegmentTextEdited,
        "SEGMENT_SPEAKER_CHANGED" => SegmentSpeakerChanged,
        "SEGMENT_SPLIT" => SegmentSplit,
        "SEGMENT_MERGED" => SegmentMerged,
        "SEGMENT_STATUS_CHANGED" => SegmentStatusChanged,
        "TRANSCRIPT_SAVED" => TranscriptSaved,
        "REVISION_CREATED" => RevisionCreated,
        "EXPORT_CREATED" => ExportCreated,
        _ => CaseCreated,
    }
}
