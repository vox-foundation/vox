//! ScriptExecutor implementation for sandboxed .vox script execution.
//!
//! SP7 scaffold: all methods return "not yet implemented".
//! Actual extraction from vox-eval/vox-exec-grammar is deferred to a follow-up SP.
//! TODO(SP7-followup): wire vox-eval sandbox runtime for execute() and validate().

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef};
use vox_plugin_api::extensions::script_executor::{ScriptExecutor, ScriptExecutor_TO};
use vox_plugin_api::host::VoxHost_TO;

#[derive(Clone)]
pub(crate) struct ScriptExecutionPlugin;

impl VoxPlugin for ScriptExecutionPlugin {
    fn id(&self) -> RString {
        RString::from("script-execution")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_script_executor(&self) -> ROption<ScriptExecutor_TO<'static, RBox<()>>> {
        ROption::RSome(ScriptExecutor_TO::from_value(self.clone(), TD_Opaque))
    }
}

impl ScriptExecutor for ScriptExecutionPlugin {
    fn execute(&self, _script_path: RStr<'_>, _args_json: RStr<'_>) -> RResult<RString, RBoxError> {
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

pub(crate) fn make_plugin(
    _host: VoxHost_TO<'static, RBox<()>>,
) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = ScriptExecutionPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
