//! Compile-only test that the CloudSync trait shape is sabi-stable.
//! Runtime behavior will be exercised in vox-plugin-cloud's tests
//! once the actual cloud code-motion completes (SP7 follow-up).

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::cloud_sync::{CloudSync, CloudSync_TO, CLOUD_SYNC_REVISION};

#[test]
fn revision_constant_is_one() {
    assert_eq!(CLOUD_SYNC_REVISION, 1);
}

struct DummyCloud;

impl CloudSync for DummyCloud {
    fn provider_id(&self) -> RString {
        RString::from("dummy-cloud")
    }
    fn upload(&self, _local_path: RStr<'_>, _remote_uri: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
    fn download(&self, _remote_uri: RStr<'_>, _local_path: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
    fn list_remote_json(&self, _remote_prefix: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
}

#[test]
fn dummy_cloud_constructs() {
    let _: CloudSync_TO<'static, RBox<()>> = CloudSync_TO::from_value(DummyCloud, TD_Opaque);
}
