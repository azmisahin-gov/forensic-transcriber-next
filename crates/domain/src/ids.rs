//! Typed identifiers.
//!
//! Every aggregate gets its own newtype so a `CaseId` can never be passed where
//! an `EvidenceId` is expected. IDs are opaque UUID v4 strings; they are stable
//! for the lifetime of the record and never reused.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Create a new random identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            /// Wrap an existing identifier (reading from storage).
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the raw string.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume into the raw string.
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(
    /// Identifies a [`crate::case::Case`].
    CaseId
);
define_id!(
    /// Identifies an [`crate::evidence::EvidenceFile`].
    EvidenceId
);
define_id!(
    /// Identifies a [`crate::evidence::DerivedMediaAsset`].
    DerivedAssetId
);
define_id!(
    /// Identifies a [`crate::job::TranscriptionJob`].
    JobId
);
define_id!(
    /// Identifies a [`crate::transcript::Transcript`].
    TranscriptId
);
define_id!(
    /// Identifies a [`crate::transcript::TranscriptSegment`].
    SegmentId
);
define_id!(
    /// Identifies a [`crate::revision::TranscriptRevision`].
    RevisionId
);
define_id!(
    /// Identifies an [`crate::audit::AuditEvent`].
    AuditEventId
);
define_id!(
    /// Identifies an [`crate::export_record::ExportRecord`].
    ExportId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_unique_and_nonempty() {
        let a = CaseId::new();
        let b = CaseId::new();
        assert_ne!(a, b);
        assert!(!a.as_str().is_empty());
    }

    #[test]
    fn round_trips_through_string() {
        let id = EvidenceId::new();
        let raw = id.clone().into_string();
        assert_eq!(EvidenceId::from_string(raw), id);
    }
}
