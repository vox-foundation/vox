//! **Oratio** — Vox speech-to-text and transcript refinement.
//!
//! Default STT uses **Candle Whisper** (pure Rust + Hugging Face weights). There is no
//! whisper.cpp / clang dependency. Set `VOX_ORATIO_MODEL` / `VOX_ORATIO_REVISION` to pick
//! checkpoints. Without the `stt-candle` feature, only `.txt` / `.md` passthrough is available.

#![warn(missing_docs)]

#[cfg(feature = "stt-candle")]
mod backends;

pub mod eval;
pub mod refine;
pub mod traits;

#[cfg(feature = "stt-candle")]
pub use backends::candle_whisper::{
    ENV_CUDA, ENV_MODEL, ENV_REVISION, candle_backend_status_json, transcribe_audio_file,
};

pub use traits::{Transcript, transcribe_path, transcript_status};
