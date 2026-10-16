//! `ftn-media` — FFmpeg / FFprobe integration.
//!
//! Subprocesses are always launched with an argument array and `shell: false`
//! semantics; no command string is ever assembled from user input. The tool
//! paths and the media file are validated before a process is started.

pub mod ffmpeg;
pub mod probe;

pub use ffmpeg::{FfmpegPaths, MediaError};
pub use probe::{probe_media, MediaProbe};
