//! Storage-layer integration tests: foreign keys, cascade and transactions.

use ftn_domain::case::Case;
use ftn_domain::evidence::{EvidenceFile, IntegrityStatus, MediaMetadata, Sha256};
use ftn_domain::ids::EvidenceId;
use ftn_domain::time::Timestamp;
use ftn_storage::SqliteStore;

fn sample_evidence(case_id: &ftn_domain::ids::CaseId, digest: &str) -> EvidenceFile {
    let sha = Sha256::parse(digest.repeat(64 / digest.len())).unwrap();
    EvidenceFile {
        id: EvidenceId::new(),
        case_id: case_id.clone(),
        original_name: "a.wav".into(),
        stored_name: "a.wav".into(),
        original_path: "C:/a.wav".into(),
        stored_path: "store/a.wav".into(),
        size_bytes: 10,
        metadata: MediaMetadata::default(),
        source_sha256: sha.clone(),
        stored_sha256: sha,
        integrity: IntegrityStatus::Verified,
        imported_at: Timestamp::from_millis(0),
    }
}

#[test]
fn foreign_keys_are_enabled() {
    let store = SqliteStore::in_memory().unwrap();
    // Inserting evidence under a non-existent case must fail.
    let ghost = ftn_domain::ids::CaseId::new();
    let ev = sample_evidence(&ghost, "ab");
    let err = store.insert_evidence_for_case(&ghost, &ev);
    assert!(err.is_err(), "foreign key violation must be rejected");
}

#[test]
fn deleting_a_case_cascades_to_evidence() {
    let store = SqliteStore::in_memory().unwrap();
    let case = Case::new("Cascade").unwrap();
    store.create_case(&case).unwrap();
    let ev = sample_evidence(&case.id, "cd");
    store.insert_evidence_for_case(&case.id, &ev).unwrap();
    assert_eq!(store.list_evidence(&case.id).unwrap().len(), 1);

    store.delete_case(&case.id).unwrap();
    // Evidence rows belong to the deleted case; the list is empty and a direct
    // fetch reports not found.
    assert!(store.list_evidence(&case.id).unwrap().is_empty());
    assert!(store.get_evidence(&ev.id).is_err());
}

#[test]
fn integrity_check_passes_on_a_fresh_database() {
    let store = SqliteStore::in_memory().unwrap();
    store.integrity_check().unwrap();
}

#[test]
fn schema_version_is_recorded() {
    let store = SqliteStore::in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), ftn_storage::SCHEMA_VERSION);
}

#[test]
fn settings_round_trip() {
    let store = SqliteStore::in_memory().unwrap();
    store.set_setting("ui.theme", "dark").unwrap();
    assert_eq!(
        store.get_setting("ui.theme").unwrap().as_deref(),
        Some("dark")
    );
    store.set_setting("ui.theme", "light").unwrap();
    assert_eq!(
        store.get_setting("ui.theme").unwrap().as_deref(),
        Some("light")
    );
    assert_eq!(store.all_settings().unwrap().len(), 1);
}
