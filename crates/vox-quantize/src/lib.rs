//! Data-free k-quant post-training quantization engine.
//!
//! SafeTensors model in -> quantized SafeTensors-canonical artifact out.
//! Device-selectable: GPU when available (cuda/metal feature), CPU fallback.
//! The engine sits behind the `engine` feature (off by default), so CORE
//! builds that depend on this crate pull no Candle.
#[cfg(feature = "engine")]
pub mod device;
#[cfg(feature = "engine")]
pub mod engine;
#[cfg(feature = "engine")]
pub mod error;
#[cfg(feature = "engine")]
pub mod policy;
#[cfg(feature = "engine")]
pub mod read;
#[cfg(feature = "engine")]
pub mod recombine;
#[cfg(feature = "engine")]
pub mod verify;
#[cfg(feature = "engine")]
pub mod write;

// TODO(SP-1 Task 2+): re-export public API once each module defines its items.
#[cfg(feature = "engine")]
pub use device::DevicePref;
#[cfg(feature = "engine")]
pub use engine::{QuantizeRequest, quantize};
#[cfg(feature = "engine")]
pub use error::QuantizeError;
#[cfg(feature = "engine")]
pub use policy::{QuantMixture, QuantizePlan, TensorRole, plan_quantize};
#[cfg(feature = "engine")]
pub use verify::{QuantReport, TensorQuantStat};

#[cfg(all(test, feature = "engine"))]
mod semcov_wave23_tests;
