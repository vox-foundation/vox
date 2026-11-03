//! Framed stdio host for one lazy Python worker.
//!
//! Owns request framing, the worker process lifecycle, and handle generations.
//! The stdlib-only worker package lives in `worker/vox_bridge/`.

pub use vox_foundation::protocol::{DispatchRequest, DispatchResponse};
