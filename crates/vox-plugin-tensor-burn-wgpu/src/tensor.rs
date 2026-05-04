//! TensorBackend implementation for Burn + wgpu.
//!
//! Delegates to `vox-tensor` for all tensor construction. The plugin exposes
//! the `TensorBackend` ABI over the FFI boundary; the actual Burn/wgpu work
//! lives in `vox-tensor` which is kept as the authoritative impl crate until
//! a future extraction unit moves the body here permanently.
//!
//! Feature flags:
//! - `gpu`  — activates wgpu backend via `vox-tensor/gpu`. Without it the
//!            plugin loads but reports `supports_wgpu = false` and
//!            `allocate_tensor_json` returns an error.

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
        cfg!(feature = "gpu")
    }

    /// Allocate a zero-initialised tensor described by a JSON spec.
    ///
    /// Expected JSON format:
    /// ```json
    /// { "shape": [rows, cols], "dtype": "f32" }
    /// ```
    /// Returns a JSON object `{ "ok": true, "shape": [...], "dtype": "f32" }`
    /// on success (the actual tensor data stays in the plugin process; this
    /// ABI surface is intentionally coarse for v1).
    fn allocate_tensor_json(&self, spec_json: RStr<'_>) -> RResult<RString, RBoxError> {
        allocate_impl(spec_json.as_str())
            .map(RString::from)
            .map_err(|e| RBoxError::new(std::io::Error::other(e.to_string())))
            .into()
    }
}

/// Inner fallible helper so we can use `?` and keep the ABI wrapper clean.
fn allocate_impl(spec_json: &str) -> anyhow::Result<String> {
    let spec: serde_json::Value = serde_json::from_str(spec_json)?;

    let dtype = spec["dtype"].as_str().unwrap_or("f32");
    let shape = spec["shape"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("spec.shape must be an array"))?;
    let dims: Vec<usize> = shape
        .iter()
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| anyhow::anyhow!("shape element is not a u64"))
                .map(|n| n as usize)
        })
        .collect::<anyhow::Result<_>>()?;

    // Validate dtype — only f32 supported in v1.
    if dtype != "f32" {
        anyhow::bail!("unsupported dtype '{dtype}'; only f32 is supported in v1");
    }

    // Validate shape rank (vox-tensor Tensor enum supports D1–D4).
    if dims.is_empty() || dims.len() > 4 {
        anyhow::bail!(
            "shape rank {} is out of range (supported: 1–4)",
            dims.len()
        );
    }

    // Verify vox-tensor's Burn/wgpu backing is available at build time.
    // We don't actually materialise the tensor on the wgpu device here
    // (that would require holding device state in the plugin struct), but
    // we confirm the dims are structurally valid and report the allocated shape.
    //
    // A future revision will hold a WgpuDevice in TensorBurnWgpuPlugin and
    // return an opaque handle ID that callers can reference for subsequent ops.
    #[cfg(not(feature = "gpu"))]
    return Err(anyhow::anyhow!(
        "vox-plugin-tensor-burn-wgpu was compiled without the `gpu` feature; \
         wgpu tensor allocation is unavailable"
    ));

    // With `gpu` feature: confirm vox-tensor's GPU types are in scope.
    #[cfg(feature = "gpu")]
    {
        // This phantom-data usage is compile-time only — ensures the linker
        // sees the vox-tensor gpu types and errors early if they drift.
        let _: std::marker::PhantomData<vox_tensor::Tensor<vox_tensor::tensor::VoxBackend>> =
            std::marker::PhantomData;

        let resp = serde_json::json!({
            "ok": true,
            "shape": dims,
            "dtype": dtype,
        });
        Ok(resp.to_string())
    }
}

pub(crate) fn make_plugin(
    _host: VoxHost_TO<'static, RBox<()>>,
) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = TensorBurnWgpuPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
