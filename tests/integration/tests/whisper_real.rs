//! Real whisper.cpp integration test.
//!
//! This test is **gated**: it only runs when `FT_TEST_WHISPER_CLI` and
//! `FT_TEST_MODEL` point at a real `whisper-cli` executable and a GGML model. If
//! either is missing it prints a SKIP notice and returns, so a machine without
//! whisper.cpp does not fail the build — and a machine with it gets a genuine
//! end-to-end check of the CLI contract and output-file handling.
//!
//! On this development machine neither the binary nor a model is installed, so
//! this test **skips** and the transcription path remains unverified here.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ftn_domain::ids::EvidenceId;
use ftn_transcription::{EngineAdapter, TranscriptionRequest, WhisperCppAdapter};

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

#[test]
fn real_whisper_cpp_transcribes_a_synthetic_wav() {
    let Some(cli) = env_path("FT_TEST_WHISPER_CLI") else {
        eprintln!("SKIP real_whisper_cpp: FT_TEST_WHISPER_CLI not set");
        return;
    };
    let Some(model) = env_path("FT_TEST_MODEL") else {
        eprintln!("SKIP real_whisper_cpp: FT_TEST_MODEL not set");
        return;
    };
    if !cli.is_file() {
        eprintln!("SKIP real_whisper_cpp: cli not found at {}", cli.display());
        return;
    }
    if !model.is_file() {
        eprintln!(
            "SKIP real_whisper_cpp: model not found at {}",
            model.display()
        );
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("tone.wav");
    let ffmpeg = env_path("FT_TEST_FFMPEG").unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let generated = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status();
    match generated {
        Ok(status) if status.success() => {}
        _ => {
            eprintln!("SKIP real_whisper_cpp: ffmpeg could not generate the input");
            return;
        }
    }

    let adapter = WhisperCppAdapter::new(Some(cli));
    let request = TranscriptionRequest::new(EvidenceId::new(), &wav, &model);
    let cancel = Arc::new(AtomicBool::new(false));
    let mut last_progress: Option<Option<f32>> = None;

    let output = adapter
        .transcribe(&request, &cancel, &mut |p| last_progress = Some(p))
        .expect("a real whisper.cpp run against a valid WAV should succeed");

    // A pure tone contains no speech, so the segment count is not asserted; the
    // point is that the CLI contract, output-file path and JSON parsing work.
    for segment in &output.segments {
        assert!(
            segment.start_ms < segment.end_ms,
            "engine produced an invalid segment: {segment:?}"
        );
    }
    assert!(
        last_progress.is_some(),
        "progress callback was never invoked"
    );
    assert!(output.model_id.is_some() || output.engine_version.is_some());
}
