//! Case aggregate.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::CaseId;
use crate::time::Timestamp;

/// A case groups evidence files, transcripts and audit history. It is the
/// top-level unit a user opens and closes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    pub id: CaseId,
    pub title: String,
    pub notes: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Case {
    /// Create a new case. The title is trimmed; an empty title is rejected.
    pub fn new(title: impl Into<String>) -> Result<Self, DomainError> {
        let title = title.into().trim().to_string();
        if title.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        let now = Timestamp::now();
        Ok(Self {
            id: CaseId::new(),
            title,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Rename the case and bump `updated_at`.
    pub fn rename(&mut self, title: impl Into<String>) -> Result<(), DomainError> {
        let title = title.into().trim().to_string();
        if title.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        self.title = title;
        self.updated_at = Timestamp::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_title() {
        assert_eq!(Case::new("   "), Err(DomainError::EmptyTitle));
    }

    #[test]
    fn trims_title() {
        let c = Case::new("  Vaka 1  ").unwrap();
        assert_eq!(c.title, "Vaka 1");
    }

    #[test]
    fn rename_rejects_empty_and_keeps_old_title() {
        let mut c = Case::new("A").unwrap();
        let before = c.updated_at;
        assert!(c.rename("  ").is_err());
        assert_eq!(c.title, "A");
        assert_eq!(c.updated_at, before);
    }
}
