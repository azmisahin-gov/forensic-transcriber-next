//! Reading media metadata with FFprobe.

use std::path::Path;

use ftn_domain::evidence::MediaMetadata;
use serde_json::Value;

use crate::ffmpeg::{run_capture, MediaError};

/// The raw plus normalised result of probing a file.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaProbe {
    pub metadata: MediaMetadata,
    /// Seconds of duration as reported (also mirrored in `metadata`).
    pub duration_seconds: Option<f64>,
}

/// Probe a media file with `ffprobe -print_format json`. Only the first audio
/// stream's parameters are used for the summary fields; the number of audio
/// streams is recorded so the UI can warn about multi-track containers.
pub fn probe_media(ffprobe: &Path, input: &Path) -> Result<MediaProbe, MediaError> {
    if !input.is_file() {
        return Err(MediaError::InputMissing(input.to_path_buf()));
    }
    let args = vec![
        "-v".to_string(),
        "error".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        input.display().to_string(),
    ];
    let stdout = run_capture(ffprobe, &args)?;
    parse_probe_json(&stdout)
}

/// Parse the JSON emitted by ffprobe. Exposed separately so it can be tested
/// without a real ffprobe binary.
pub fn parse_probe_json(json: &str) -> Result<MediaProbe, MediaError> {
    let value: Value = serde_json::from_str(json).map_err(|e| MediaError::Parse(e.to_string()))?;

    let format = value.get("format");
    let format_name = format
        .and_then(|f| f.get("format_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = format
        .and_then(|f| f.get("duration"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d >= 0.0);

    let streams = value
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let audio_streams: Vec<&Value> = streams
        .iter()
        .filter(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"))
        .collect();
    let audio_stream_count = audio_streams.len() as u32;
    let first = audio_streams.first();

    let metadata = MediaMetadata {
        format: format_name,
        codec: first
            .and_then(|s| s.get("codec_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        duration_seconds: duration,
        sample_rate: first
            .and_then(|s| s.get("sample_rate"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u32>().ok()),
        channels: first
            .and_then(|s| s.get("channels"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        bit_depth: first
            .and_then(|s| s.get("bits_per_raw_sample"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u32>().ok())
            .or_else(|| {
                first
                    .and_then(|s| s.get("bits_per_sample"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
            }),
        audio_stream_count: Some(audio_stream_count),
    };

    Ok(MediaProbe {
        duration_seconds: duration,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_ffprobe_output() {
        let json = r#"{
            "streams": [
                {"index":0,"codec_type":"audio","codec_name":"mp3","sample_rate":"44100","channels":2,"bits_per_raw_sample":"0"},
                {"index":1,"codec_type":"video","codec_name":"mjpeg"}
            ],
            "format": {"format_name":"mp3","duration":"12.345000"}
        }"#;
        let probe = parse_probe_json(json).unwrap();
        assert_eq!(probe.metadata.codec.as_deref(), Some("mp3"));
        assert_eq!(probe.metadata.sample_rate, Some(44100));
        assert_eq!(probe.metadata.channels, Some(2));
        assert_eq!(probe.metadata.audio_stream_count, Some(1));
        assert!((probe.duration_seconds.unwrap() - 12.345).abs() < 1e-9);
    }

    #[test]
    fn counts_multiple_audio_streams() {
        let json = r#"{
            "streams": [
                {"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":1},
                {"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":1}
            ],
            "format": {"format_name":"mov","duration":"3.0"}
        }"#;
        let probe = parse_probe_json(json).unwrap();
        assert_eq!(probe.metadata.audio_stream_count, Some(2));
    }

    #[test]
    fn handles_missing_duration_without_panicking() {
        let json = r#"{"streams":[],"format":{"format_name":"wav"}}"#;
        let probe = parse_probe_json(json).unwrap();
        assert_eq!(probe.duration_seconds, None);
        assert_eq!(probe.metadata.codec, None);
        assert_eq!(probe.metadata.audio_stream_count, Some(0));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_probe_json("not json").is_err());
    }
}
