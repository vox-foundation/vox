//! Compile-only test that the AudioCapture trait shape is sabi-stable.
//! Runtime behavior will be exercised in vox-plugin-oratio's tests
//! once the actual audio code-motion completes (SP7 follow-up).

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::audio_capture::{
    AudioCapture, AudioCapture_TO, AUDIO_CAPTURE_REVISION,
};

#[test]
fn revision_constant_is_one() {
    assert_eq!(AUDIO_CAPTURE_REVISION, 1);
}

struct DummyAudio;

impl AudioCapture for DummyAudio {
    fn list_devices_json(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
    fn start_capture(&self, _device_id: RStr<'_>, _config_json: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
    fn stop_capture(&self) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
    fn read_chunk(&self) -> RResult<RVec<u8>, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
}

#[test]
fn dummy_audio_constructs() {
    let _: AudioCapture_TO<'static, RBox<()>> =
        AudioCapture_TO::from_value(DummyAudio, TD_Opaque);
}
