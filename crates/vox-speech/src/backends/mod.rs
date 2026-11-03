//! STT backends for Oratio. In-process: Sherpa-ONNX. Candle Whisper runs in the `oratio`
//! plugin (vox-plugin-speech), reached through a host-registered transcriber
//! (`backend_dispatch::register_whisper_transcriber`).

pub mod asr_backend;
#[cfg(feature = "stt-sherpa")]
pub mod sherpa_model_config;
#[cfg(feature = "stt-sherpa")]
pub mod sherpa_onnx;

#[cfg(feature = "audio-decode")]
pub mod audio_io;

#[cfg(feature = "cloud")]
pub mod cloud_offload;
