//! Remote Axis: attach the GUI to a `vox serve` gateway instead of the local daemon.

pub mod backend;
pub mod error;
pub mod gate;
pub mod gateway_client;
pub mod manifest;
pub mod tls_pin;
