//! Registers vox-plugin-speech's `oratio` plugin as vox-speech's Whisper transcriber
//! (Phase 3 D-08), so dictation that selects Whisper reaches Candle through the plugin
//! host. Sherpa stays in-process in vox-speech.

/// vox-speech's `ExternalWhisperTranscribe`: mono f32 LE PCM bytes + config JSON in,
/// transcription JSON out, via the plugin's `SpeechToText::transcribe`.
fn transcribe_with_oratio_plugin(pcm_le_f32: &[u8], config_json: &str) -> Result<String, String> {
    let plugin = vox_plugin_host::cached_code_plugin("oratio")
        .map_err(|e| format!("oratio plugin load: {e}"))?;
    let stt = plugin
        .plugin
        .as_speech_to_text()
        .into_option()
        .ok_or_else(|| "oratio plugin missing SpeechToText accessor".to_string())?;
    stt.transcribe(pcm_le_f32.into(), config_json.into())
        .into_result()
        .map(|json| json.into_string())
        .map_err(|e| e.to_string())
}

/// Install the plugin-backed transcriber; the plugin itself loads lazily on first use.
pub(crate) fn register() {
    vox_speech::backend_dispatch::register_whisper_transcriber(transcribe_with_oratio_plugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_installs_the_oratio_plugin_transcriber() {
        register();
        assert!(vox_speech::backend_dispatch::has_registered_whisper_transcriber());
    }
}
