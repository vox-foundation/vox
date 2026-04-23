pub mod cuda_shim;

#[cfg(feature = "mens-candle-qlora-cuda")]
pub use cuda_shim::load_kernels;
