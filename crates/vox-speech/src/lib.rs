//! **Oratio** — Vox speech-to-text and transcript refinement.
//!
//! STT runs through **Sherpa-ONNX** in-process (`stt-sherpa`) or **Candle Whisper** in the
//! `oratio` plugin (vox-plugin-speech), which hosts register through
//! [`backend_dispatch::register_whisper_transcriber`]. Audio file decoding needs the
//! `audio-decode` feature; without it only `.txt` / `.md` passthrough is available.

#![warn(missing_docs)]

mod language;
mod runtime_config;

/// Serializes tests across the crate that mutate shared global env vars
/// (e.g. `VOX_ORATIO_SHERPA_MODEL_DIR`). Currently taken by
/// `backends::sherpa_model_config`'s test; a planned `backend_dispatch` test
/// (STT-accuracy plan Task 6) will mutate the same var and must take this
/// same lock too. `cargo test` runs test functions concurrently by default;
/// a comment-only "don't run this in parallel" convention is not enough once
/// more than one file touches the same var.
/// Both consumers are gated on `stt-sherpa` (`backends::sherpa_model_config`,
/// and `backend_dispatch`'s Sherpa-fallback test), so the lock carries the same
/// predicate — otherwise it is an unused static on the default feature set,
/// which `clippy --all-targets -- -D warnings` rejects.
#[cfg(all(test, feature = "stt-sherpa"))]
pub(crate) mod env_test_lock {
    pub static SHERPA_MODEL_DIR_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

pub mod acoustic_preprocess;

pub use acoustic_preprocess::{AcousticPreprocessDiagnostics, preprocess_audio_pcm_f32_reported};
pub mod ast_mapper;
pub mod contextual_bias;

pub mod backend_dispatch;
pub mod backends;

pub mod eval;
pub mod eval_srt;
pub mod failure_taxonomy;
pub mod refine;
pub mod routing;
pub mod session;
pub mod speaker_profile;
pub mod speech_intent;
pub mod speech_lexicon;
pub mod speech_normalize;
pub mod speech_policy;
/// Env-tunable stabilization thresholds for **future** streaming decoders; offline `transcribe_path` ignores this.
pub mod streaming_partial;
// Gated: `subtitle::srt`'s audio path decodes files through `backends::audio_io`,
// which only exists with `audio-decode`.
#[cfg(feature = "audio-decode")]
pub mod subtitle;
/// Policy helpers (escalation hint, cache keys) for hosts — not required for file-based STT.
pub mod tiering;
pub mod trace;
pub mod traits;
pub mod transcript_rerank;
pub mod vad;

#[cfg(feature = "serve")]
pub mod serve;

pub use backend_dispatch::create_backend;
pub use backends::asr_backend::{AsrBackend, AsrOutput};

/// JSON status for the in-process Candle backend — always `{"stt_candle": false}`:
/// Candle Whisper runs in the `oratio` plugin (vox-plugin-speech).
#[must_use]
pub fn candle_backend_status_json() -> serde_json::Value {
    serde_json::json!({ "stt_candle": false })
}

pub use routing::{RouteMode, RouteResponse, route_transcript, route_transcript_with_options};
pub use runtime_config::{
    HfTunables, LlmPolicyTunables, LogitConstraintTunables, OratioRuntimeConfig, RefineTunables,
    RoutingTunables, SessionTimingDefaults, resolved_runtime_config,
    runtime_config_diagnostic_json, tool_route_min_confidence,
};
pub use session::{
    CaptureState, DeadlineDiagnostics, OratioDeadlineTaxonomy, OratioSessionConfig,
    OratioSessionResult, OratioTimings, session_config_with_runtime, transcribe_path_session,
    transcribe_path_session_with_runtime,
};
pub use speech_intent::{
    SpeechIntentAction, SpeechIntentEnvelope, build_intent_envelope,
    clarification_prompt_for_slots, missing_slot_ids,
};
pub use speech_policy::clarification_recommended;
pub use streaming_partial::{StreamingStabilizationConfig, should_commit_partial};
pub use tiering::{speech_cache_key, speech_escalation_recommended};
pub use traits::{
    TranscribeDetail, Transcript, refine_raw_text, transcribe_path, transcribe_path_detailed,
    transcript_status,
};
pub use transcript_rerank::{
    pick_best_transcript_index, pick_best_transcript_index_with_raw,
    pick_best_transcript_index_with_raw_and_domain, rerank_candidates_best_first,
    rerank_candidates_best_first_with_context, rerank_candidates_best_first_with_raw,
    rerank_candidates_best_first_with_raw_and_domain,
};
