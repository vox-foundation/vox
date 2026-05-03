//! Load the spike's cdylib output through `libloading` and exercise its
//! exported symbols. Documents the loader pattern that SP3's MlBackend
//! plugin will adopt.

use libloading::{Library, Symbol};
use std::ffi::{CStr, c_int};
use std::path::PathBuf;

fn dylib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("..");
    p.push("target");
    p.push("release");
    if cfg!(target_os = "windows") {
        p.push("vox_plugin_cuda_spike.dll");
    } else if cfg!(target_os = "macos") {
        p.push("libvox_plugin_cuda_spike.dylib");
    } else {
        p.push("libvox_plugin_cuda_spike.so");
    }
    p
}

#[test]
fn dlopen_resolves_smoke_symbol() {
    let path = dylib_path();
    assert!(
        path.exists(),
        "spike dylib not built at {path:?}; run `cargo build --release -p vox-plugin-cuda-spike` first"
    );
    let lib = unsafe { Library::new(&path).expect("dlopen failed") };
    let f: Symbol<unsafe extern "C" fn() -> *const u8> =
        unsafe { lib.get(b"vox_spike_smoke").expect("symbol not found") };
    let ptr = unsafe { f() };
    let s = unsafe { CStr::from_ptr(ptr.cast()) };
    assert_eq!(s.to_str().unwrap(), "vox-cuda-spike-ok");
}

#[test]
fn dlopen_calls_cuda_path() {
    let path = dylib_path();
    assert!(path.exists());
    let lib = unsafe { Library::new(&path).expect("dlopen failed") };
    let f: Symbol<unsafe extern "C" fn() -> c_int> = unsafe {
        lib.get(b"vox_spike_cuda_available")
            .expect("symbol not found")
    };
    let result = unsafe { f() };
    eprintln!("vox_spike_cuda_available returned: {result}");
    assert!(
        result == 0 || result == 1,
        "expected 0 or 1, got {result}"
    );
}
