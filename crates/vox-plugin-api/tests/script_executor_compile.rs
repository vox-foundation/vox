//! Compile-only test that the ScriptExecutor trait shape is sabi-stable.
//! Runtime behavior will be exercised in vox-plugin-script-execution's tests
//! once the actual script executor code-motion completes (SP7 follow-up).

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::script_executor::{
    ScriptExecutor, ScriptExecutor_TO, SCRIPT_EXECUTOR_REVISION,
};

#[test]
fn revision_constant_is_one() {
    assert_eq!(SCRIPT_EXECUTOR_REVISION, 1);
}

struct DummyExecutor;

impl ScriptExecutor for DummyExecutor {
    fn execute(
        &self,
        _script_path: RStr<'_>,
        _args_json: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
    fn validate(&self, _script_path: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
}

#[test]
fn dummy_executor_constructs() {
    let _: ScriptExecutor_TO<'static, RBox<()>> =
        ScriptExecutor_TO::from_value(DummyExecutor, TD_Opaque);
}
