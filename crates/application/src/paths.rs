//! On-disk workspace layout.
//!
//! ```text
//! <data_dir>/
//!   case.sqlite
//!   cases/<case_id>/
//!     evidence/original/<stored_name>
//!     derived/<evidence_id>.16k.wav
//!     exports/<file>
//!     revisions/<revision_id>.json   (audit snapshots, human readable)
//! ```

use std::path::{Path, PathBuf};

use ftn_domain::ids::{CaseId, EvidenceId};

/// Resolved directories for a workspace.
#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub data_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("case.sqlite")
    }

    pub fn case_dir(&self, case_id: &CaseId) -> PathBuf {
        self.data_dir.join("cases").join(case_id.as_str())
    }

    pub fn evidence_original_dir(&self, case_id: &CaseId) -> PathBuf {
        self.case_dir(case_id).join("evidence").join("original")
    }

    pub fn derived_dir(&self, case_id: &CaseId) -> PathBuf {
        self.case_dir(case_id).join("derived")
    }

    pub fn exports_dir(&self, case_id: &CaseId) -> PathBuf {
        self.case_dir(case_id).join("exports")
    }

    pub fn derived_wav_path(&self, case_id: &CaseId, evidence_id: &EvidenceId) -> PathBuf {
        self.derived_dir(case_id)
            .join(format!("{evidence_id}.16k.wav"))
    }

    /// Create every directory for a case.
    pub fn ensure_case_dirs(&self, case_id: &CaseId) -> std::io::Result<()> {
        std::fs::create_dir_all(self.evidence_original_dir(case_id))?;
        std::fs::create_dir_all(self.derived_dir(case_id))?;
        std::fs::create_dir_all(self.exports_dir(case_id))?;
        Ok(())
    }

    /// Ensure the top-level data directory exists.
    pub fn ensure_root(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }

    pub fn is_within_data_dir(&self, path: &Path) -> bool {
        let canonical_root =
            std::fs::canonicalize(&self.data_dir).unwrap_or_else(|_| self.data_dir.clone());
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        canonical.starts_with(&canonical_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_path_ends_with_16k_wav() {
        let p = WorkspacePaths::new("C:/data");
        let ev = EvidenceId::new();
        let path = p.derived_wav_path(&CaseId::new(), &ev);
        assert!(path.to_string_lossy().ends_with(".16k.wav"));
        assert!(path.to_string_lossy().contains(ev.as_str()));
    }
}
