//! Subtitle generation subsystem (.srt).

/// Subtitle generator formatting utilities (SRT).
pub mod srt;
/// Fallback ffmpeg audio extraction.
pub mod ffmpeg_extract;

pub use srt::{format_srt, generate_srt_file};
