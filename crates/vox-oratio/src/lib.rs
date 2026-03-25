//! **Oratio** — Vox speech-to-text and transcript refinement.
//!
//! Default STT uses **Candle Whisper** (pure Rust + Hugging Face weights). There is no
//! whisper.cpp / clang dependency. Set `VOX_ORATIO_MODEL` / `VOX_ORATIO_REVISION` to pick
//! checkpoints. Without the `stt-candle` feature, only `.txt` / `.md` passthrough is available.

#![warn(missing_docs)]

mod language;
mod runtime_config;

#[cfg(feature = "stt-candle")]
mod backends;

pub mod eval;
pub mod refine;
pub mod routing;
pub mod session;
pub mod traits;

#[cfg(feature = "stt-candle")]
pub use backends::candle_whisper::{
    ENV_CUDA, ENV_MODEL, ENV_REVISION, candle_backend_status_json, transcribe_audio_file,
    transcribe_audio_file_with_language, LanguageEnvOverride,
};

pub use runtime_config::{
    resolved_runtime_config, runtime_config_diagnostic_json, tool_route_min_confidence,
    HfTunables, LlmPolicyTunables, OratioRuntimeConfig, RefineTunables, RoutingTunables,
    SessionTimingDefaults,
};
pub use traits::{transcribe_path, transcribe_path_detailed, transcript_status, TranscribeDetail, Transcript};
pub use session::{
    session_config_with_runtime,
    transcribe_path_session_with_runtime,
    CaptureState,
    DeadlineDiagnostics,
    OratioDeadlineTaxonomy,
    OratioSessionConfig,
    OratioSessionResult,
    OratioTimings,
    transcribe_path_session,
};
pub use routing::{route_transcript, route_transcript_with_options, RouteMode, RouteResponse};
