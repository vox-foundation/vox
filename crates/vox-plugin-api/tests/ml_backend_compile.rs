//! Compile-only test that the MlBackend trait shape is sabi-stable.
//! Runtime behavior is exercised in vox-plugin-mens-candle-cuda's tests.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::ml_backend::{
    MlBackend, MlBackend_TO, MlModelHandle, ML_BACKEND_REVISION,
};

#[test]
fn revision_constant_is_one() {
    assert_eq!(ML_BACKEND_REVISION, 1);
}

struct DummyBackend;

impl MlBackend for DummyBackend {
    fn load_model(&self, _path: RStr<'_>) -> RResult<RBox<MlModelHandle>, RBoxError> {
        RResult::ROk(RBox::new(MlModelHandle { opaque: 0 }))
    }
    fn train_step(
        &self,
        _model: &MlModelHandle,
        _batch: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("{}"))
    }
    fn eval_step(
        &self,
        _model: &MlModelHandle,
        _batch: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("{}"))
    }
    fn save_checkpoint(
        &self,
        _model: &MlModelHandle,
        _dest: RStr<'_>,
    ) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
}

#[test]
fn dummy_backend_constructs_as_trait_object() {
    let _: MlBackend_TO<'static, RBox<()>> =
        MlBackend_TO::from_value(DummyBackend, TD_Opaque);
}
