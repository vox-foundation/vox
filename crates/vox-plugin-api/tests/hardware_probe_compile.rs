//! Compile-only test that the HardwareProbe trait shape is sabi-stable.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::hardware_probe::{
    HardwareProbe, HardwareProbe_TO, HARDWARE_PROBE_REVISION,
};

#[test]
fn revision_constant_is_one() {
    assert_eq!(HARDWARE_PROBE_REVISION, 1);
}

struct DummyProbe;

impl HardwareProbe for DummyProbe {
    fn probe_summary_json(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(r#"{"devices":[]}"#))
    }
    fn device_metrics_json(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(r#"{"metrics":[]}"#))
    }
}

#[test]
fn dummy_probe_constructs() {
    let _: HardwareProbe_TO<'static, RBox<()>> =
        HardwareProbe_TO::from_value(DummyProbe, TD_Opaque);
}
