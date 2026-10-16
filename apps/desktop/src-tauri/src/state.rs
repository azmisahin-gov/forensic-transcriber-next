//! Shared application state for the Tauri commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ftn_application::Workspace;
use ftn_media::FfmpegPaths;
use ftn_transcription::{CancelToken, WhisperCppAdapter};
use tauri::Manager;

/// State managed by Tauri.
///
/// The workspace is wrapped in an `Arc<Mutex<..>>` so a long-running
/// transcription can run on a background thread while other commands (listing
/// cases, reading evidence) still acquire the lock briefly. `jobs` maps a job id
/// to its cancellation token so the UI can cancel a run in progress.
pub struct AppState {
    pub workspace: Arc<Mutex<Workspace>>,
    pub engine_path: Arc<Mutex<Option<PathBuf>>>,
    pub jobs: Arc<Mutex<HashMap<String, CancelToken>>>,
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
            workspace: Arc::new(Mutex::new(workspace)),
            engine_path: Arc::new(Mutex::new(engine_path)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Build a fresh engine adapter from the current configured path.
    pub fn engine_adapter(&self) -> WhisperCppAdapter {
        let path = self.engine_path.lock().ok().and_then(|p| p.clone());
        WhisperCppAdapter::new(path)
    }
}
