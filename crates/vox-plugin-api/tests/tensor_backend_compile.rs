//! Compile-only test that the TensorBackend trait shape is sabi-stable.
//! Runtime behavior will be exercised in vox-plugin-tensor-burn-wgpu's tests
//! once the actual tensor code-motion completes (SP7 follow-up).

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::tensor_backend::{
    TENSOR_BACKEND_REVISION, TensorBackend, TensorBackend_TO,
};

#[test]
fn revision_constant_is_one() {
    assert_eq!(TENSOR_BACKEND_REVISION, 1);
}

struct DummyTensor;

impl TensorBackend for DummyTensor {
    fn name(&self) -> RString {
        RString::from("dummy-tensor")
    }
    fn supports_cuda(&self) -> bool {
        false
    }
    fn supports_wgpu(&self) -> bool {
        false
    }
    fn allocate_tensor_json(&self, _spec_json: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("{}"))
    }
}

#[test]
fn dummy_tensor_constructs() {
    let _: TensorBackend_TO<'static, RBox<()>> =
        TensorBackend_TO::from_value(DummyTensor, TD_Opaque);
}
