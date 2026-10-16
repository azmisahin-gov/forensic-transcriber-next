//! The whisper.cpp adapter.
//!
//! NOTE: the command-line contract below follows the documented whisper.cpp
//! CLI (`-m`, `-f`, `-l`, `-t`, `-oj`, `--output-file`). It has **not** been
//! executed against a real whisper-cli binary in this environment because none
//! is installed. The argument construction and the JSON parser are unit-tested;
//! end-to-end transcription remains unverified here and is marked as such in
//! the documentation.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

use serde_json::Value;

use crate::engine::{
    CancelToken, EngineAdapter, EngineError, EngineProbe, ProgressFn, RawSegment,
    TranscriptionOutput, TranscriptionRequest,
};

/// The whisper.cpp adapter.
#[derive(Debug, Clone)]
pub struct WhisperCppAdapter {
    /// Path to the `whisper-cli` executable, when configured.
    pub binary: Option<PathBuf>,
}

impl WhisperCppAdapter {
    pub fn new(binary: Option<PathBuf>) -> Self {
        Self { binary }
    }

    fn binary_or_err(&self) -> Result<&Path, EngineError> {
        match self.binary.as_deref() {
            Some(p) if p.is_file() => Ok(p),
            Some(p) => Err(EngineError::EngineNotFound(p.display().to_string())),
            None => Err(EngineError::EngineNotFound("whisper-cli".to_string())),
        }
    }

    /// Build the argument vector for a run. Kept separate so it is testable
    /// without spawning anything.
    pub fn build_args(request: &TranscriptionRequest, output_base: &Path) -> Vec<String> {
        let mut args = vec![
            "-m".to_string(),
            request.model.display().to_string(),
            "-f".to_string(),
            request.input.display().to_string(),
            "-l".to_string(),
            request.language.clone(),
            "-oj".to_string(),
            "-of".to_string(),
            output_base.display().to_string(),
            "-nt".to_string(),
        ];
        if let Some(t) = request.threads {
            args.push("-t".to_string());
            args.push(t.to_string());
        }
        if !request.use_gpu {
            args.push("-ng".to_string());
        }
        args
    }
}

impl EngineAdapter for WhisperCppAdapter {
    fn id(&self) -> &'static str {
        "whisper.cpp"
    }

    fn probe(&self) -> EngineProbe {
        let path = match self.binary.as_deref() {
            Some(p) if p.is_file() => p,
            _ => {
                return EngineProbe {
                    available: false,
                    version: None,
                    devices: Vec::new(),
                    note: Some("whisper-cli not configured or not found".to_string()),
                }
            }
        };
        // `whisper-cli --help` prints usage and the build line to stderr.
        match Command::new(path).arg("--help").output() {
            Ok(out) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                EngineProbe {
                    available: true,
                    version: first_version_line(&text),
                    devices: Vec::new(),
                    note: None,
                }
            }
            Err(_) => EngineProbe {
                available: false,
                version: None,
                devices: Vec::new(),
                note: Some("engine executable could not be launched".to_string()),
            },
        }
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
        cancel: &CancelToken,
        progress: ProgressFn<'_>,
    ) -> Result<TranscriptionOutput, EngineError> {
        let binary = self.binary_or_err()?;
        request.validate(Some(binary))?;

        let output_base = request.input.with_file_name(format!(
            "{}.ftn-output",
            request
                .input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("audio")
        ));
        let json_path = output_base.with_extension("json");

        let args = Self::build_args(request, &output_base);
        let mut child = Command::new(binary)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream stderr for progress and cancellation.
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if cancel.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&json_path);
                    return Err(EngineError::Cancelled);
                }
                if let Some(p) = parse_progress_line(&line) {
                    progress(Some(p));
                }
            }
        }

        let status = child.wait()?;
        if !status.success() {
            let _ = std::fs::remove_file(&json_path);
            return Err(EngineError::ProcessFailed {
                code: status.code().unwrap_or(-1),
                stderr: "see engine diagnostics".to_string(),
            });
        }

        let json = std::fs::read_to_string(&json_path).map_err(|e| {
            EngineError::Parse(format!(
                "engine output not found ({}): {e}",
                json_path.display()
            ))
        })?;
        let _ = std::fs::remove_file(&json_path);

        let segments = parse_whisper_json(&json)?;
        progress(Some(1.0));

        Ok(TranscriptionOutput {
            segments,
            engine_version: None,
            model_id: request
                .model
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            settings: serde_json::json!({
                "language": request.language,
                "threads": request.threads,
                "use_gpu": request.use_gpu,
                "engine": "whisper.cpp",
            }),
        })
    }
}

/// Parse `whisper-cli` progress lines such as `progress = 42%`.
fn parse_progress_line(line: &str) -> Option<f32> {
    let idx = line.find("progress =")?;
    let rest = &line[idx + "progress =".len()..];
    let pct: String = rest
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    pct.parse::<f32>().ok().map(|v| (v / 100.0).clamp(0.0, 1.0))
}

/// Parse the JSON produced by `whisper-cli -oj`. The documented shape is:
/// `{ "transcription": [ { "offsets": { "from": ms, "to": ms }, "text": "..." } ] }`.
pub fn parse_whisper_json(json: &str) -> Result<Vec<RawSegment>, EngineError> {
    let value: Value = serde_json::from_str(json).map_err(|e| EngineError::Parse(e.to_string()))?;
    let transcription = value
        .get("transcription")
        .and_then(|v| v.as_array())
        .ok_or_else(|| EngineError::Parse("missing 'transcription' array".to_string()))?;

    let mut segments = Vec::with_capacity(transcription.len());
    for item in transcription {
        let offsets = item.get("offsets");
        let from = offsets
            .and_then(|o| o.get("from"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let to = offsets
            .and_then(|o| o.get("to"))
            .and_then(|v| v.as_i64())
            .unwrap_or(from);
        let text = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        // Skip empty or degenerate spans rather than emit an invalid segment.
        if to <= from || text.is_empty() {
            continue;
        }
        segments.push(RawSegment {
            start_ms: from,
            end_ms: to,
            text,
            confidence: None,
        });
    }
    Ok(segments)
}

/// Find a version-looking line in `--help` output.
fn first_version_line(text: &str) -> Option<String> {
    text.lines()
        .find(|l| l.to_ascii_lowercase().contains("whisper") || l.contains("version"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftn_domain::ids::EvidenceId;

    #[test]
    fn build_args_includes_model_input_language() {
        let req =
            TranscriptionRequest::new(EvidenceId::new(), "C:/a/working.wav", "C:/m/model.bin");
        let args = WhisperCppAdapter::build_args(&req, Path::new("C:/a/working.ftn-output"));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"C:/m/model.bin".to_string()));
        assert!(args.contains(&"-f".to_string()));
        assert!(args.contains(&"-l".to_string()));
        assert!(args.contains(&"tr".to_string()));
        // CPU-only default must appear when GPU is not requested.
        assert!(args.contains(&"-ng".to_string()));
    }

    #[test]
    fn build_args_omits_ng_when_gpu_requested() {
        let mut req =
            TranscriptionRequest::new(EvidenceId::new(), "C:/a/working.wav", "C:/m/model.bin");
        req.use_gpu = true;
        let args = WhisperCppAdapter::build_args(&req, Path::new("out"));
        assert!(!args.contains(&"-ng".to_string()));
    }

    #[test]
    fn parses_progress_lines() {
        assert_eq!(
            parse_progress_line("whisper_print: progress = 25%"),
            Some(0.25)
        );
        assert_eq!(parse_progress_line("no progress here"), None);
        assert_eq!(parse_progress_line("progress = 100%"), Some(1.0));
    }

    #[test]
    fn parses_documented_json_shape() {
        let json = r#"{
            "transcription": [
                {"offsets":{"from":0,"to":1200},"text":" Merhaba dünya."},
                {"offsets":{"from":1200,"to":2400},"text":"Nasılsınız?"}
            ]
        }"#;
        let segs = parse_whisper_json(json).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].start_ms, 0);
        assert_eq!(segs[0].end_ms, 1200);
        assert_eq!(segs[0].text, "Merhaba dünya.");
        assert_eq!(segs[1].text, "Nasılsınız?");
    }

    #[test]
    fn skips_degenerate_spans() {
        let json = r#"{"transcription":[
            {"offsets":{"from":100,"to":100},"text":"zero-length"},
            {"offsets":{"from":200,"to":300},"text":"   "},
            {"offsets":{"from":400,"to":500},"text":"ok"}
        ]}"#;
        let segs = parse_whisper_json(json).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "ok");
    }

    #[test]
    fn missing_transcription_array_is_an_error() {
        assert!(parse_whisper_json("{}").is_err());
    }

    #[test]
    fn probe_without_binary_is_unavailable() {
        let adapter = WhisperCppAdapter::new(None);
        let probe = adapter.probe();
        assert!(!probe.available);
        assert!(probe.note.is_some());
    }

    #[test]
    fn transcribe_without_binary_reports_engine_not_found() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let adapter = WhisperCppAdapter::new(None);
        let req = TranscriptionRequest::new(EvidenceId::new(), "x.wav", "m.bin");
        let cancel = Arc::new(AtomicBool::new(false));
        let mut noop = |_: Option<f32>| {};
        let err = adapter.transcribe(&req, &cancel, &mut noop).unwrap_err();
        assert_eq!(err.code(), "ENGINE_NOT_FOUND");
    }
}
