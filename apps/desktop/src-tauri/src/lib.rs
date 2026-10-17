//! Forensic Transcriber Next — Tauri 2 desktop shell.
//!
//! The shell is intentionally thin: it owns the window and the command surface
//! and delegates all logic to `ftn-application`.

mod commands;
mod dto;
mod error;
mod runtime;
mod state;

pub use error::CommandError;

use state::AppState;
use tauri::Manager;

/// Build and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Startup never aborts the window: a failure is reported through the
            // `startup_status` command so the UI can show a clear message.
            let state = AppState::init(app);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_status,
            commands::prepare_engine,
            commands::download_model,
            commands::cancel_preparation,
            commands::list_cases,
            commands::create_case,
            commands::get_case,
            commands::list_evidence,
            commands::import_evidence,
            commands::verify_evidence,
            commands::transcribe,
            commands::cancel_transcription,
            commands::job_status,
            commands::transcript_for_evidence,
            commands::list_segments,
            commands::edit_segment,
            commands::export_transcript,
            commands::engine_probe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
