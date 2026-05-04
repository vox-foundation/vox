//! AudioCapture implementation for the Oratio speech-to-code pipeline.
//!
//! SP7 scaffold: all non-trivial methods return "not yet implemented".
//! Actual extraction from vox-oratio is deferred to a follow-up SP.
//! TODO(SP7-followup): extract audio pipeline from vox-oratio.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef};
use vox_plugin_api::extensions::audio_capture::{AudioCapture, AudioCapture_TO};
use vox_plugin_api::host::VoxHost_TO;

#[derive(Clone)]
pub(crate) struct OratioPlugin;

impl VoxPlugin for OratioPlugin {
    fn id(&self) -> RString {
        RString::from("oratio")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_audio_capture(&self) -> ROption<AudioCapture_TO<'static, RBox<()>>> {
        ROption::RSome(AudioCapture_TO::from_value(self.clone(), TD_Opaque))
    }
}

impl AudioCapture for OratioPlugin {
    fn list_devices_json(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }

    fn start_capture(
        &self,
        _device_id: RStr<'_>,
        _config_json: RStr<'_>,
    ) -> RResult<(), RBoxError> {
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

pub(crate) fn make_plugin(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = OratioPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
