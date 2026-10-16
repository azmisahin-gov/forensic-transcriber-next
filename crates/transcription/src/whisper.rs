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

        // whisper.cpp writes `<output_file>.<ext>` for the base given to `-of`,
        // so all engine output goes to a private temporary directory and the JSON
        // path is the base with `.json` appended (not `with_extension`, which
        // would clobber the last dot-component of the base).
        let run_dir = unique_run_dir()?;
        let output_base = run_dir.join("transcript");
        let json_path = json_output_path(&output_base);
        let args = Self::build_args(request, &output_base);

        let mut child = Command::new(binary)
            .args(&args)
            .stdin(Stdio::null())
            // JSON goes to a file; stdout is not used. Nulling it removes any
            // chance of the child blocking on a full stdout pipe.
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        // Drain stderr on a separate thread so the child can never block on a
        // full stderr pipe and so cancellation is observed promptly.
        let (tx, rx) = std::sync::mpsc::channel::<f32>();
        let reader = child.stderr.take().map(|stderr| {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    if let Some(p) = parse_progress_line(&line) {
                        let _ = tx.send(p);
                    }
                }
            })
        });

        let exit_status = loop {
            while let Ok(p) = rx.try_recv() {
                progress(Some(p));
            }
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            match child.try_wait()? {
                Some(status) => break Some(status),
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        };
        while let Ok(p) = rx.try_recv() {
            progress(Some(p));
        }
        if let Some(handle) = reader {
            let _ = handle.join();
        }

        let result = match exit_status {
            // `None` is only produced by cancellation.
            None => Err(EngineError::Cancelled),
            Some(status) if !status.success() => Err(EngineError::ProcessFailed {
                code: status.code().unwrap_or(-1),
                stderr: "engine diagnostics were not captured to avoid leaking audio content"
                    .to_string(),
            }),
            Some(_) => {
                let json = std::fs::read_to_string(&json_path).map_err(|e| {
                    EngineError::Parse(format!(
                        "engine output not found ({}): {e}",
                        json_path.display()
                    ))
                })?;
                parse_whisper_document(&json).map(|doc| doc.into_output(request))
            }
        };

        // Always remove the temporary run directory, on success, failure or cancel.
        let _ = std::fs::remove_dir_all(&run_dir);

        let output = result?;
        progress(Some(1.0));
        Ok(output)
    }
}

/// The `<base>.json` path that `whisper-cli -of <base>` writes.
fn json_output_path(base: &Path) -> PathBuf {
    let mut os = base.as_os_str().to_os_string();
    os.push(".json");
    PathBuf::from(os)
}

/// Create a unique private directory for one engine run.
fn unique_run_dir() -> Result<PathBuf, EngineError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("ftn-asr-{}-{}", std::process::id(), nanos));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
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

/// A parsed whisper.cpp JSON document.
#[derive(Debug, Clone, PartialEq)]
pub struct WhisperDocument {
    pub segments: Vec<RawSegment>,
    /// Model identifier reported by the engine when present.
    pub model_id: Option<String>,
    /// The engine's `systeminfo` string (version/backends), when present.
    pub system_info: Option<String>,
}

impl WhisperDocument {
    /// Turn the document into the engine-agnostic output, filling in the model
    /// id from the model file name when the engine did not report one.
    pub fn into_output(self, request: &TranscriptionRequest) -> TranscriptionOutput {
        let model_id = self.model_id.or_else(|| {
            request
                .model
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        });
        TranscriptionOutput {
            segments: self.segments,
            engine_version: None,
            model_id,
            settings: serde_json::json!({
                "language": request.language,
                "threads": request.threads,
                "use_gpu": request.use_gpu,
                "engine": "whisper.cpp",
                "system_info": self.system_info,
            }),
        }
    }
}

/// Parse the JSON produced by `whisper-cli -oj`. The documented shape is:
/// `{ "transcription": [ { "offsets": { "from": ms, "to": ms }, "text": "..." } ],
///    "model": { ... }, "systeminfo": "..." }`.
pub fn parse_whisper_document(json: &str) -> Result<WhisperDocument, EngineError> {
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

    let model_id = value.get("model").and_then(extract_model_id);
    let system_info = value
        .get("systeminfo")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(WhisperDocument {
        segments,
        model_id,
        system_info,
    })
}

/// Parse just the segments (kept for callers that only need them).
pub fn parse_whisper_json(json: &str) -> Result<Vec<RawSegment>, EngineError> {
    Ok(parse_whisper_document(json)?.segments)
}

/// Extract a model id from the `model` object, tolerating the two shapes the
/// engine has used (`type` as a string, or `type.name`).
fn extract_model_id(model: &Value) -> Option<String> {
    let type_field = model.get("type")?;
    if let Some(s) = type_field.as_str() {
        return Some(s.to_string());
    }
    type_field
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
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
    fn json_output_path_appends_extension_without_clobbering_dots() {
        // The regression this guards: `with_extension("json")` would turn
        // `a.16k.ftn-output` into `a.16k.json`, but whisper writes
        // `a.16k.ftn-output.json`.
        let base = Path::new("C:/tmp/run/transcript");
        assert_eq!(
            json_output_path(base).to_string_lossy(),
            "C:/tmp/run/transcript.json"
        );
        let dotted = Path::new("C:/tmp/a.16k.ftn-output");
        assert_eq!(
            json_output_path(dotted).to_string_lossy(),
            "C:/tmp/a.16k.ftn-output.json"
        );
    }

    #[test]
    fn parses_model_and_systeminfo_when_present() {
        let json = r#"{
            "systeminfo": "whisper.cpp 1.9.4, CUDA : 0",
            "model": { "type": { "name": "large-v3-turbo" } },
            "transcription": [ {"offsets":{"from":0,"to":1000},"text":"a"} ]
        }"#;
        let doc = parse_whisper_document(json).unwrap();
        assert_eq!(doc.model_id.as_deref(), Some("large-v3-turbo"));
        assert_eq!(
            doc.system_info.as_deref(),
            Some("whisper.cpp 1.9.4, CUDA : 0")
        );
    }

    #[test]
    fn parses_string_model_type() {
        let json = r#"{"model":{"type":"small"},"transcription":[{"offsets":{"from":0,"to":1},"text":"x"}]}"#;
        assert_eq!(
            parse_whisper_document(json).unwrap().model_id.as_deref(),
            Some("small")
        );
    }

    #[test]
    fn document_into_output_falls_back_to_model_file_name() {
        let req = TranscriptionRequest::new(EvidenceId::new(), "in.wav", "C:/m/ggml-medium.bin");
        let doc = WhisperDocument {
            segments: vec![RawSegment {
                start_ms: 0,
                end_ms: 1,
                text: "x".to_string(),
                confidence: None,
            }],
            model_id: None,
            system_info: None,
        };
        let out = doc.into_output(&req);
        assert_eq!(out.model_id.as_deref(), Some("ggml-medium"));
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
