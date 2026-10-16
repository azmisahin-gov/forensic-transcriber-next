//! `ftn-transcription` — pluggable ASR engines.
//!
//! whisper.cpp is the default engine, wrapped behind [`EngineAdapter`] so a
//! future engine can be added without touching the pipeline.

pub mod engine;
pub mod whisper;

pub use engine::{
    CancelToken, EngineAdapter, EngineError, EngineProbe, ProgressFn, RawSegment,
    TranscriptionOutput, TranscriptionRequest,
};
pub use whisper::{parse_whisper_json, WhisperCppAdapter};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Create a fresh cancellation token.
pub fn new_cancel_token() -> CancelToken {
    Arc::new(AtomicBool::new(false))
}
