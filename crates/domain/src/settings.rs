//! Local application settings (key/value). Never leaves the machine.

use serde::{Deserialize, Serialize};

use crate::time::Timestamp;

/// A single application setting row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSetting {
    pub key: String,
    pub value: String,
    pub updated_at: Timestamp,
}

impl ApplicationSetting {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            updated_at: Timestamp::now(),
        }
    }
}

/// Well-known setting keys.
pub mod keys {
    pub const THEME: &str = "ui.theme";
    pub const LANGUAGE: &str = "ui.language";
    pub const MODEL_PATH: &str = "engine.model_path";
    pub const ENGINE_PATH: &str = "engine.whisper_path";
    pub const FFMPEG_PATH: &str = "engine.ffmpeg_path";
    pub const DATA_DIR: &str = "app.data_dir";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_carry_timestamp() {
        let s = ApplicationSetting::new(keys::THEME, "dark");
        assert_eq!(s.key, "ui.theme");
        assert!(s.updated_at.as_millis() > 0);
    }
}
