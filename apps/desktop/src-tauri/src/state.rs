//! Shared application state for the Tauri commands.
//!
//! Startup is resilient: if the workspace cannot be opened the window still
//! opens and the failure is reported through `startup_status`. The engine and
//! model are discovered automatically; a first launch with neither present
//! shows a guided preparation flow instead of asking for file paths.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ftn_application::Workspace;
use ftn_media::FfmpegPaths;
use ftn_transcription::{CancelToken, WhisperCppAdapter};
use tauri::Manager;

use crate::error::CommandError;
use crate::runtime;

pub struct AppState {
    workspace: Option<Arc<Mutex<Workspace>>>,
    startup_error: Option<String>,
    data_dir: PathBuf,
    engine_path: Arc<Mutex<Option<PathBuf>>>,
    model_path: Arc<Mutex<Option<PathBuf>>>,
    jobs: Arc<Mutex<HashMap<String, CancelToken>>>,
    prepare_cancel: Arc<Mutex<Option<CancelToken>>>,
}

impl AppState {
    pub fn init(app: &tauri::App) -> Self {
        let data_dir = std::env::var_os("FTN_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| app.path().app_data_dir().ok())
            .unwrap_or_else(|| PathBuf::from("ftn-data"));
        let engine_env = std::env::var_os("FTN_ENGINE_PATH").map(PathBuf::from);
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));

        if let Err(error) = app.asset_protocol_scope().allow_directory(&data_dir, true) {
            eprintln!("asset scope could not be extended to the data dir: {error}");
        }

        let engine_path = runtime::find_engine(&exe_dir, &data_dir, engine_env.as_deref());
        let model_path = runtime::find_model(&exe_dir, &data_dir, None);

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
            model_path: Arc::new(Mutex::new(model_path)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            prepare_cancel: Arc::new(Mutex::new(None)),
        }
    }

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

    pub fn engine_path(&self) -> Option<PathBuf> {
        self.engine_path.lock().ok().and_then(|p| p.clone())
    }

    pub fn set_engine_path(&self, path: PathBuf) {
        if let Ok(mut guard) = self.engine_path.lock() {
            *guard = Some(path);
        }
        if let Ok(workspace) = self.workspace() {
            if let Ok(mut ws) = workspace.lock() {
                ws.set_engine(Arc::new(self.engine_adapter()));
            }
        }
    }

    pub fn model_path(&self) -> Option<PathBuf> {
        self.model_path.lock().ok().and_then(|p| p.clone())
    }

    pub fn set_model_path(&self, path: PathBuf) {
        if let Ok(mut guard) = self.model_path.lock() {
            *guard = Some(path);
        }
    }

    pub fn jobs(&self) -> Arc<Mutex<HashMap<String, CancelToken>>> {
        Arc::clone(&self.jobs)
    }

    pub fn prepare_cancel(&self) -> Arc<Mutex<Option<CancelToken>>> {
        Arc::clone(&self.prepare_cancel)
    }

    pub fn data_dir(&self) -> String {
        self.data_dir.display().to_string()
    }

    pub fn data_dir_path(&self) -> PathBuf {
        self.data_dir.clone()
    }

    pub fn startup_error(&self) -> Option<String> {
        self.startup_error.clone()
    }

    pub fn is_ready(&self) -> bool {
        self.workspace.is_some()
    }
}
