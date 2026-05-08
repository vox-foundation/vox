//! AudioCapture extension point — microphone / audio pipeline plugins.
//! Used by Oratio for speech-to-code.

use abi_stable::{sabi_trait, std_types::*};

pub const AUDIO_CAPTURE_REVISION: u32 = 1;

#[sabi_trait]
pub trait AudioCapture: Send + Sync {
    fn revision(&self) -> u32 {
        AUDIO_CAPTURE_REVISION
    }
    fn list_devices_json(&self) -> RResult<RString, RBoxError>;
    fn start_capture(&self, device_id: RStr<'_>, config_json: RStr<'_>) -> RResult<(), RBoxError>;
    fn stop_capture(&self) -> RResult<(), RBoxError>;
    fn read_chunk(&self) -> RResult<RVec<u8>, RBoxError>;
}
