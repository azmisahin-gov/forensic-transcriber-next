//! Forensic Transcriber Next — Tauri 2 desktop shell.
//!
//! The shell is intentionally thin: it owns the window and the command surface
//! and delegates all logic to `ftn-application`.

mod commands;
mod dto;
mod error;
mod state;

pub use error::CommandError;

use state::AppState;
use tauri::Manager;

/// Build and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::init(app)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_cases,
            commands::create_case,
            commands::get_case,
            commands::list_evidence,
            commands::import_evidence,
            commands::verify_evidence,
            commands::transcribe,
            commands::transcript_for_evidence,
            commands::list_segments,
            commands::edit_segment,
            commands::export_transcript,
            commands::engine_probe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
