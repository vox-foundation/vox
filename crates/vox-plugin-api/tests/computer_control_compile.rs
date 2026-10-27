use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::computer_control::{
    COMPUTER_CONTROL_REVISION, ComputerControl, ComputerControl_TO,
};

struct DummyComputer;

impl ComputerControl for DummyComputer {
    fn permissions(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(r#"{"platform":"test"}"#))
    }
    fn list_apps(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
    fn list_windows(&self, _options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
    fn snapshot(&self, _options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(r#"{"snapshot_id":"s1"}"#))
    }
    fn screenshot(&self, _options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("{}"))
    }
    fn act(&self, action_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(action_json.as_str()))
    }
}

#[test]
fn computer_control_trait_object_dispatches() {
    let to = ComputerControl_TO::from_value(DummyComputer, TD_Opaque);
    assert_eq!(to.revision(), COMPUTER_CONTROL_REVISION);
    assert_eq!(to.list_apps().unwrap().as_str(), "[]");
    assert_eq!(
        to.act(RStr::from(r#"{"kind":"click"}"#)).unwrap().as_str(),
        r#"{"kind":"click"}"#
    );
}
