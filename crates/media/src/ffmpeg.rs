//! Locating and running the FFmpeg tools.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// Errors from media operations. Safe to log; never includes file contents.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("required tool not found: {0}")]
    ToolNotFound(String),

    #[error("media file does not exist: {0}")]
    InputMissing(PathBuf),

    #[error("ffprobe failed (exit {code}): {stderr}")]
    ProbeFailed { code: i32, stderr: String },

    #[error("ffmpeg failed (exit {code}): {stderr}")]
    TranscodeFailed { code: i32, stderr: String },

    #[error("could not parse ffprobe output: {0}")]
    Parse(String),

    #[error("io error launching {tool}: {source}")]
    Io {
        tool: String,
        #[source]
        source: std::io::Error,
    },
}

/// Resolved paths to the FFmpeg tools.
#[derive(Debug, Clone)]
pub struct FfmpegPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

impl FfmpegPaths {
    /// Resolve the tools. A caller-provided path (from Settings) wins; otherwise
    /// the tools are looked up on `PATH`. Missing tools are reported as an
    /// error, never silently assumed.
    pub fn resolve(
        ffmpeg_override: Option<&Path>,
        ffprobe_override: Option<&Path>,
    ) -> Result<Self, MediaError> {
        let ffmpeg = resolve_tool(ffmpeg_override, "ffmpeg")?;
        let ffprobe = resolve_tool(ffprobe_override, "ffprobe")?;
        Ok(Self { ffmpeg, ffprobe })
    }
}

fn resolve_tool(override_path: Option<&Path>, name: &str) -> Result<PathBuf, MediaError> {
    if let Some(p) = override_path {
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
        return Err(MediaError::ToolNotFound(format!(
            "configured {name} path does not exist: {}",
            p.display()
        )));
    }
    which(name).ok_or_else(|| MediaError::ToolNotFound(name.to_string()))
}

/// Minimal `which`: search the `PATH` for an executable, honouring `PATHEXT`
/// on Windows.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path_var) {
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
    }
    None
}

/// Run a tool with an argument array (never a shell string) and capture stdout.
pub fn run_capture(tool: &Path, args: &[String]) -> Result<String, MediaError> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|source| MediaError::Io {
            tool: tool.display().to_string(),
            source,
        })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(2000)
            .collect();
        return Err(MediaError::ProbeFailed { code, stderr });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Convert any decodable media into a 16 kHz mono PCM WAV suitable for
/// whisper.cpp. The output lands in a temporary sibling and is only published on
/// success.
pub fn transcode_to_16k_mono_wav(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
) -> Result<(), MediaError> {
    if !input.is_file() {
        return Err(MediaError::InputMissing(input.to_path_buf()));
    }
    let tmp = output.with_extension("wav.part");
    let args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input.display().to_string(),
        "-vn".to_string(),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        "16000".to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
        // The temp file uses a `.part` suffix, so the muxer cannot be inferred
        // from the extension; state it explicitly.
        "-f".to_string(),
        "wav".to_string(),
        tmp.display().to_string(),
    ];
    let output_result = Command::new(ffmpeg)
        .args(&args)
        .output()
        .map_err(|source| MediaError::Io {
            tool: ffmpeg.display().to_string(),
            source,
        })?;
    if !output_result.status.success() {
        let _ = std::fs::remove_file(&tmp);
        let code = output_result.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output_result.stderr)
            .chars()
            .take(2000)
            .collect();
        return Err(MediaError::TranscodeFailed { code, stderr });
    }
    std::fs::rename(&tmp, output).map_err(|source| MediaError::Io {
        tool: "rename".to_string(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_reports_missing_override() {
        let missing = Path::new("C:/definitely/not/here/ffmpeg.exe");
        let err = FfmpegPaths::resolve(Some(missing), Some(missing));
        assert!(matches!(err, Err(MediaError::ToolNotFound(_))));
    }

    #[test]
    fn transcode_reports_missing_input() {
        // Use an obviously missing input; the tool path is never executed.
        let err = transcode_to_16k_mono_wav(
            Path::new("ffmpeg-does-not-run"),
            Path::new("C:/no/such/input.wav"),
            Path::new("C:/tmp/out.wav"),
        );
        assert!(matches!(err, Err(MediaError::InputMissing(_))));
    }
}
