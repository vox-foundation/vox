//! TensorBackend implementation for Burn + wgpu.
//!
//! SP7 scaffold: all non-trivial methods return "not yet implemented".
//! Actual extraction from vox-tensor is deferred to a follow-up SP.
//! TODO(SP7-followup): extract Burn/wgpu tensor ops from vox-tensor.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef};
use vox_plugin_api::extensions::tensor_backend::{TensorBackend, TensorBackend_TO};
use vox_plugin_api::host::VoxHost_TO;

#[derive(Clone)]
pub(crate) struct TensorBurnWgpuPlugin;

impl VoxPlugin for TensorBurnWgpuPlugin {
    fn id(&self) -> RString {
        RString::from("tensor-burn-wgpu")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_tensor_backend(
        &self,
    ) -> ROption<TensorBackend_TO<'static, RBox<()>>> {
        ROption::RSome(TensorBackend_TO::from_value(self.clone(), TD_Opaque))
    }
}

impl TensorBackend for TensorBurnWgpuPlugin {
    fn name(&self) -> RString {
        RString::from("tensor-burn-wgpu")
    }

    fn supports_cuda(&self) -> bool {
        false
    }

    fn supports_wgpu(&self) -> bool {
        false // TODO(SP7-followup): return true once wgpu backend is wired
    }

    fn allocate_tensor_json(&self, _spec_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented; SP7 scaffold",
        )))
    }
}

pub(crate) fn make_plugin(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = TensorBurnWgpuPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
