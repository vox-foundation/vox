fn main() {
    println!("cargo:rerun-if-env-changed=VOX_PRECOMPILED_KERNELS");

    let precompiled = std::env::var("VOX_PRECOMPILED_KERNELS").is_ok();

    if precompiled {
        println!("cargo:rustc-cfg=feature=\"vox-precompiled-kernels\"");
        // Skip nvcc checks and compilation
        return;
    }

    // Default behavior (if any was intended here)
}
