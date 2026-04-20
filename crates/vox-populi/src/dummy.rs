use candle_core::CudaDevice;
fn main() {
    let d: CudaDevice = unimplemented!();
    d.load_ptx_doesnt_exist();
}
