//! Shared application state for the Tauri commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ftn_application::Workspace;
use ftn_media::FfmpegPaths;
use ftn_transcription::WhisperCppAdapter;
use tauri::Manager;

/// State managed by Tauri. The workspace is behind a mutex because SQLite
/// access is serialised and some use cases need `&mut`.
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub engine_path: Mutex<Option<PathBuf>>,
}

impl AppState {
    /// Initialise the workspace. Data directory and engine path come from
    /// environment variables when present, otherwise from Tauri's app data dir
    /// and "no engine configured".
    pub fn init(app: &tauri::App) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = std::env::var_os("FTN_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| app.path().app_data_dir().ok())
            .unwrap_or_else(|| PathBuf::from("ftn-data"));
        let engine_path = std::env::var_os("FTN_ENGINE_PATH").map(PathBuf::from);

        let engine = Arc::new(WhisperCppAdapter::new(engine_path.clone()));
        let media = FfmpegPaths::resolve(None, None).ok();
        let workspace = Workspace::open(data_dir, engine)?.with_media(media);

        Ok(Self {
            workspace: Mutex::new(workspace),
            engine_path: Mutex::new(engine_path),
        })
    }

    pub fn engine_adapter(&self) -> WhisperCppAdapter {
        let path = self.engine_path.lock().ok().and_then(|p| p.clone());
        WhisperCppAdapter::new(path)
    }
}
