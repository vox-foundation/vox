//! CandleCudaPlugin — implements VoxPlugin + MlBackend.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::VoxPlugin;
use vox_plugin_api::extensions::ml_backend::{MlBackend, MlBackend_TO, MlModelHandle};

/// Convert an `anyhow::Error` to an `RBoxError`.
/// `anyhow::Error` does not implement `std::error::Error`, so we wrap it via
/// `std::io::Error` (which does).
fn anyhow_to_rbox(e: anyhow::Error) -> RBoxError {
    RBoxError::new(std::io::Error::other(e.to_string()))
}

#[derive(Clone)]
pub struct CandleCudaPlugin;

impl CandleCudaPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl VoxPlugin for CandleCudaPlugin {
    fn id(&self) -> RString {
        RString::from("mens-candle-cuda")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_ml_backend(&self) -> ROption<MlBackend_TO<'static, RBox<()>>> {
        ROption::RSome(MlBackend_TO::from_value(self.clone(), TD_Opaque))
    }
}

impl MlBackend for CandleCudaPlugin {
    fn load_model(&self, model_path: RStr<'_>) -> RResult<RBox<MlModelHandle>, RBoxError> {
        match crate::model::CandleModel::load_from_path(model_path.as_str()) {
            Ok(model) => {
                // SAFETY: we box the model and store the raw pointer as an opaque usize.
                // The pointer is reconstituted in subsequent calls (train_step, eval_step,
                // save_checkpoint). Memory is intentionally leaked here — the model lives
                // until the plugin dylib is unloaded.
                //
                // TODO(batch 4): add `unload_model(handle)` to MlBackend trait to allow
                // explicit deallocation via `Box::from_raw(handle.opaque as *mut CandleModel)`.
                let raw = Box::into_raw(Box::new(model)) as usize;
                RResult::ROk(RBox::new(MlModelHandle { opaque: raw }))
            }
            Err(e) => RResult::RErr(anyhow_to_rbox(e)),
        }
    }

    fn train_step(
        &self,
        model: &MlModelHandle,
        batch_json: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        // SAFETY: opaque was set by load_model from Box<CandleModel> — valid as long as the
        // host passes back a handle that came from this plugin instance's load_model call.
        #[allow(unsafe_code)]
        let candle_model =
            unsafe { &mut *(model.opaque as *mut crate::model::CandleModel) };
        match crate::training::run_train_step(candle_model, batch_json.as_str()) {
            Ok(stats_json) => RResult::ROk(RString::from(stats_json)),
            Err(e) => RResult::RErr(anyhow_to_rbox(e)),
        }
    }

    fn eval_step(
        &self,
        model: &MlModelHandle,
        batch_json: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        #[allow(unsafe_code)]
        let candle_model =
            unsafe { &*(model.opaque as *const crate::model::CandleModel) };
        match crate::training::run_eval_step(candle_model, batch_json.as_str()) {
            Ok(stats_json) => RResult::ROk(RString::from(stats_json)),
            Err(e) => RResult::RErr(anyhow_to_rbox(e)),
        }
    }

    fn save_checkpoint(
        &self,
        model: &MlModelHandle,
        dest: RStr<'_>,
    ) -> RResult<(), RBoxError> {
        #[allow(unsafe_code)]
        let candle_model =
            unsafe { &*(model.opaque as *const crate::model::CandleModel) };
        match crate::checkpoint::save(candle_model, dest.as_str()) {
            Ok(()) => RResult::ROk(()),
            Err(e) => RResult::RErr(anyhow_to_rbox(e)),
        }
    }
}
