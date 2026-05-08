use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::speech_to_text::{SpeechToText, SpeechToText_TO};

struct DummyStt;

impl SpeechToText for DummyStt {
    fn transcribe(&self, _audio_pcm: RSlice<'_, u8>, _config_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(r#"{"text":"","language":"en","segments":[]}"#))
    }

    fn begin_stream(&self, _config_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other("streaming not yet supported")))
    }

    fn push_audio(&self, _session_id: RStr<'_>, _audio_pcm: RSlice<'_, u8>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other("streaming not yet supported")))
    }

    fn end_stream(&self, _session_id: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other("streaming not yet supported")))
    }
}

#[test]
fn dummy_stt_constructs() {
    let _: SpeechToText_TO<'static, RBox<()>> =
        SpeechToText_TO::from_value(DummyStt, TD_Opaque);
}
