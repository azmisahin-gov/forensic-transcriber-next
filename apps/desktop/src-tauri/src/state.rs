//! Shared application state for the Tauri commands.
//!
//! Startup is resilient: if the workspace cannot be opened (for example the data
//! directory is not writable), the window still opens and the failure is
//! reported through the `startup_status` command and a UI banner, rather than
//! crashing before the UI can render.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ftn_application::Workspace;
use ftn_media::FfmpegPaths;
use ftn_transcription::{CancelToken, WhisperCppAdapter};
use tauri::Manager;

use crate::error::CommandError;

/// State managed by Tauri.
pub struct AppState {
    workspace: Option<Arc<Mutex<Workspace>>>,
    startup_error: Option<String>,
    data_dir: PathBuf,
    engine_path: Arc<Mutex<Option<PathBuf>>>,
    jobs: Arc<Mutex<HashMap<String, CancelToken>>>,
}

impl AppState {
    /// Build the state. Never fails: a startup problem is captured instead.
    pub fn init(app: &tauri::App) -> Self {
        let data_dir = std::env::var_os("FTN_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| app.path().app_data_dir().ok())
            .unwrap_or_else(|| PathBuf::from("ftn-data"));
        let engine_path = std::env::var_os("FTN_ENGINE_PATH").map(PathBuf::from);

        // Expose only the managed data directory to the asset protocol so the
        // audio player can stream managed copies. Nothing else is exposed.
        if let Err(error) = app.asset_protocol_scope().allow_directory(&data_dir, true) {
            // Non-fatal: only the audio player is affected.
            eprintln!("asset scope could not be extended to the data dir: {error}");
        }

        let engine = Arc::new(WhisperCppAdapter::new(engine_path.clone()));
        let media = FfmpegPaths::resolve(None, None).ok();
        let opened = Workspace::open(&data_dir, engine).map(|ws| ws.with_media(media));
        let (workspace, startup_error) = match opened {
            Ok(ws) => (Some(Arc::new(Mutex::new(ws))), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
            workspace,
            startup_error,
            data_dir,
            engine_path: Arc::new(Mutex::new(engine_path)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Obtain the workspace, or a clear STARTUP_ERROR if it could not be opened.
    pub fn workspace(&self) -> Result<Arc<Mutex<Workspace>>, CommandError> {
        self.workspace.clone().ok_or_else(|| {
            CommandError::new(
                "STARTUP_ERROR",
                self.startup_error
                    .clone()
                    .unwrap_or_else(|| "Çalışma alanı başlatılamadı.".to_string()),
            )
        })
    }

    pub fn engine_adapter(&self) -> WhisperCppAdapter {
        let path = self.engine_path.lock().ok().and_then(|p| p.clone());
        WhisperCppAdapter::new(path)
    }

    /// The map of running jobs to their cancellation tokens.
    pub fn jobs(&self) -> Arc<Mutex<HashMap<String, CancelToken>>> {
        Arc::clone(&self.jobs)
    }

    pub fn data_dir(&self) -> String {
        self.data_dir.display().to_string()
    }

    pub fn startup_error(&self) -> Option<String> {
        self.startup_error.clone()
    }

    pub fn is_ready(&self) -> bool {
        self.workspace.is_some()
    }
}
