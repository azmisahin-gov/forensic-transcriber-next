//! `ftn-export` — transcript rendering to TXT, SRT, JSON and HTML.
//!
//! Every renderer takes the same input so the outputs stay consistent. Turkish
//! and other Unicode text is preserved as UTF-8; HTML is escaped; file names are
//! never derived from user text without sanitisation.

use ftn_domain::export_record::ExportFormat;
use ftn_domain::transcript::{SegmentStatus, TranscriptSegment};
use thiserror::Error;

/// The transcript JSON schema version emitted by this build.
pub const TRANSCRIPT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("unsafe output file name: {0}")]
    UnsafeName(String),
}

/// Context that accompanies a transcript in every export.
#[derive(Debug, Clone)]
pub struct ExportMeta {
    pub case_id: String,
    pub case_title: String,
    pub evidence_name: String,
    pub transcript_id: String,
    pub language: String,
    pub engine: Option<String>,
    pub model_id: Option<String>,
    pub generated_at: String,
    pub app_version: String,
}

/// Render plain text: one line per segment, prefixed with the time range and
/// speaker. `[ANLAŞILAMADI]`-style markers are preserved verbatim.
pub fn render_txt(meta: &ExportMeta, segments: &[TranscriptSegment]) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", meta.case_title));
    out.push_str(&format!("Kanıt: {}\n", meta.evidence_name));
    out.push_str(&format!("Oluşturulma: {}\n", meta.generated_at));
    out.push_str(&"-".repeat(40));
    out.push('\n');
    for s in segments {
        out.push_str(&format!(
            "[{}] {}: {}\n",
            format_srt_time(s.start_ms),
            s.speaker.as_str(),
            s.text
        ));
    }
    out
}

/// Render SRT subtitles. Times format as `HH:MM:SS,mmm`; the sequence number is
/// 1-based and continuous.
pub fn render_srt(
    _meta: &ExportMeta,
    segments: &[TranscriptSegment],
) -> Result<String, ExportError> {
    let mut out = String::new();
    for (i, s) in segments.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_srt_time(s.start_ms),
            format_srt_time(s.end_ms)
        ));
        out.push_str(&s.text);
        out.push_str("\n\n");
    }
    Ok(out)
}

/// Render the structured JSON export with an explicit schema version.
pub fn render_json(
    meta: &ExportMeta,
    segments: &[TranscriptSegment],
) -> Result<String, ExportError> {
    let automatic = segments
        .iter()
        .filter(|s| s.status == SegmentStatus::Automatic)
        .count();
    let json = serde_json::json!({
        "schema_version": TRANSCRIPT_SCHEMA_VERSION,
        "generated_at": meta.generated_at,
        "app_version": meta.app_version,
        "case": { "id": meta.case_id, "title": meta.case_title },
        "evidence": { "original_name": meta.evidence_name },
        "transcript": {
            "id": meta.transcript_id,
            "language": meta.language,
            "engine": meta.engine,
            "model_id": meta.model_id,
        },
        "summary": {
            "total": segments.len(),
            "automatic": automatic,
            "human": segments.len() - automatic,
        },
        "segments": segments.iter().map(|s| serde_json::json!({
            "id": s.id.as_str(),
            "ordinal": s.ordinal,
            "start_ms": s.start_ms,
            "end_ms": s.end_ms,
            "start": format_srt_time(s.start_ms),
            "end": format_srt_time(s.end_ms),
            "speaker": s.speaker.as_str(),
            "text": s.text,
            "original_text": s.original_text,
            "status": s.status.as_str(),
            "confidence": s.confidence,
            "flags": s.flags,
        })).collect::<Vec<_>>(),
    });
    Ok(serde_json::to_string_pretty(&json)?)
}

/// Render a self-contained HTML report. All text is escaped; no remote
/// resources are referenced.
pub fn render_html(meta: &ExportMeta, segments: &[TranscriptSegment]) -> String {
    let automatic = segments
        .iter()
        .filter(|s| s.status == SegmentStatus::Automatic)
        .count();
    let mut rows = String::new();
    for s in segments {
        rows.push_str(&format!(
            "<tr data-status=\"{status}\"><td class=\"time\">{start}</td><td class=\"speaker\">{speaker}</td><td class=\"text\">{text}</td><td class=\"status\">{status}</td></tr>\n",
            status = html_escape(s.status.as_str()),
            start = html_escape(&format_srt_time(s.start_ms)),
            speaker = html_escape(s.speaker.as_str()),
            text = html_escape(&s.text),
        ));
    }
    format!(
        "<!doctype html>\n<html lang=\"tr\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{title}</title>\
<style>body{{font-family:system-ui,Segoe UI,Arial,sans-serif;margin:2rem;color:#1a1a1a}}\
table{{border-collapse:collapse;width:100%}}td{{border-bottom:1px solid #ddd;padding:.4rem;vertical-align:top}}\
.time{{white-space:nowrap;color:#555;font-variant-numeric:tabular-nums}}.speaker{{white-space:nowrap;font-weight:600}}\
.status{{white-space:nowrap;color:#777;font-size:.85em}}.meta{{color:#555;margin:.3rem 0}}\
tr[data-status=\"AUTOMATIC\"] .status{{color:#a15c00}}</style></head><body>\
<h1>{title}</h1>\
<p class=\"meta\">Kanıt: {evidence}</p>\
<p class=\"meta\">Oluşturulma: {generated} — Uygulama {app}</p>\
<p class=\"meta\">Toplam {total} bölüm — {automatic} otomatik, {human} insan incelemesi</p>\
<table><thead><tr><th>Zaman</th><th>Konuşmacı</th><th>Metin</th><th>Durum</th></tr></thead>\
<tbody>\n{rows}</tbody></table></body></html>\n",
        title = html_escape(&meta.case_title),
        evidence = html_escape(&meta.evidence_name),
        generated = html_escape(&meta.generated_at),
        app = html_escape(&meta.app_version),
        total = segments.len(),
        automatic = automatic,
        human = segments.len() - automatic,
        rows = rows,
    )
}

/// Render the requested format.
pub fn render(
    format: ExportFormat,
    meta: &ExportMeta,
    segments: &[TranscriptSegment],
) -> Result<String, ExportError> {
    match format {
        ExportFormat::Txt => Ok(render_txt(meta, segments)),
        ExportFormat::Srt => render_srt(meta, segments),
        ExportFormat::Json => render_json(meta, segments),
        ExportFormat::Html => Ok(render_html(meta, segments)),
    }
}

/// Format milliseconds as `HH:MM:SS,mmm` (SRT convention).
pub fn format_srt_time(ms: i64) -> String {
    let clamped = ms.max(0);
    let hours = clamped / 3_600_000;
    let minutes = (clamped / 60_000) % 60;
    let seconds = (clamped / 1000) % 60;
    let millis = clamped % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

/// Escape text for HTML body content.
pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Derive a safe export file name from a case title and format. The result is a
/// single path component with no separators.
pub fn safe_export_file_name(
    case_title: &str,
    format: ExportFormat,
) -> Result<String, ExportError> {
    let mut stem: String = case_title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    stem = stem.trim_matches('_').to_string();
    if stem.is_empty() {
        stem = "transcript".to_string();
    }
    if stem.len() > 80 {
        stem.truncate(80);
    }
    if stem.contains('/') || stem.contains('\\') || stem.contains(':') {
        return Err(ExportError::UnsafeName(case_title.to_string()));
    }
    Ok(format!("{stem}.{}", format.extension()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftn_domain::ids::TranscriptId;
    use ftn_domain::transcript::SpeakerLabel;

    fn segment(ord: u32, start: i64, end: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment::automatic(
            TranscriptId::new(),
            ord,
            start,
            end,
            SpeakerLabel::default_speaker(),
            text,
        )
        .unwrap()
    }

    fn meta() -> ExportMeta {
        ExportMeta {
            case_id: "c1".into(),
            case_title: "Deneme Vakası".into(),
            evidence_name: "kayit.mp3".into(),
            transcript_id: "t1".into(),
            language: "tr".into(),
            engine: Some("whisper.cpp".into()),
            model_id: Some("large-v3-turbo".into()),
            generated_at: "2026-10-16T09:30:00.000Z".into(),
            app_version: "0.1.0".into(),
        }
    }

    #[test]
    fn srt_time_formatting() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
        assert_eq!(format_srt_time(1_500), "00:00:01,500");
        assert_eq!(format_srt_time(3_661_001), "01:01:01,001");
        assert_eq!(format_srt_time(-5), "00:00:00,000");
    }

    #[test]
    fn srt_is_one_based_and_continuous() {
        let segs = vec![segment(0, 0, 1000, "ilk"), segment(1, 1000, 2000, "ikinci")];
        let srt = render_srt(&meta(), &segs).unwrap();
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,000\nilk\n\n2\n"));
        assert!(srt.trim_end().ends_with("ikinci"));
    }

    #[test]
    fn json_has_schema_version_and_summary() {
        let mut segs = vec![segment(0, 0, 1000, "a"), segment(1, 1000, 2000, "b")];
        segs[0].edit_text("edited").unwrap();
        let json = render_json(&meta(), &segs).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["schema_version"], TRANSCRIPT_SCHEMA_VERSION);
        assert_eq!(v["summary"]["total"], 2);
        assert_eq!(v["summary"]["automatic"], 1);
        assert_eq!(v["summary"]["human"], 1);
        assert_eq!(v["segments"][0]["original_text"], "a");
        assert_eq!(v["segments"][0]["text"], "edited");
    }

    #[test]
    fn html_escapes_user_text() {
        let segs = vec![segment(0, 0, 1000, "<script>alert('x')</script> & \"q\"")];
        let html = render_html(&meta(), &segs);
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&amp;"));
    }

    #[test]
    fn txt_preserves_unicode() {
        let segs = vec![segment(0, 0, 1000, "Şğüöçİı — ğüşiöç")];
        let txt = render_txt(&meta(), &segs);
        assert!(txt.contains("Şğüöçİı — ğüşiöç"));
    }

    #[test]
    fn safe_export_name_strips_separators() {
        let name = safe_export_file_name("../../etc/passwd", ExportFormat::Json).unwrap();
        assert!(!name.contains('/'));
        assert!(!name.contains(".."));
        assert!(name.ends_with(".json"));
    }

    #[test]
    fn safe_export_name_handles_empty_title() {
        assert_eq!(
            safe_export_file_name("///", ExportFormat::Txt).unwrap(),
            "transcript.txt"
        );
    }

    #[test]
    fn render_dispatches_all_formats() {
        let segs = vec![segment(0, 0, 1000, "a")];
        for f in [
            ExportFormat::Txt,
            ExportFormat::Srt,
            ExportFormat::Json,
            ExportFormat::Html,
        ] {
            assert!(render(f, &meta(), &segs).is_ok());
        }
    }
}
