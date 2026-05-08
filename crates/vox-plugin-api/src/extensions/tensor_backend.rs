//! TensorBackend extension point — generic tensor compute backend
//! (Burn + wgpu, candle, etc.). For training plugins that don't fit
//! the higher-level MlBackend abstraction.

use abi_stable::{sabi_trait, std_types::*};

pub const TENSOR_BACKEND_REVISION: u32 = 1;

#[sabi_trait]
pub trait TensorBackend: Send + Sync {
    fn revision(&self) -> u32 {
        TENSOR_BACKEND_REVISION
    }
    fn name(&self) -> RString;
    fn supports_cuda(&self) -> bool;
    fn supports_wgpu(&self) -> bool;
    fn allocate_tensor_json(&self, spec_json: RStr<'_>) -> RResult<RString, RBoxError>;
}
