//! STT backends (Candle Whisper — pure Rust stack; no whisper.cpp / clang).

#[cfg(feature = "stt-candle")]
mod audio_io;
#[cfg(feature = "stt-candle")]
mod candle_engine;
#[cfg(feature = "stt-candle")]
pub mod candle_whisper;
#[cfg(feature = "stt-candle")]
mod multilingual;
