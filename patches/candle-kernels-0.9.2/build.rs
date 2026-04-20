use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src/compatibility.cuh");
    println!("cargo::rerun-if-changed=src/cuda_utils.cuh");
    println!("cargo::rerun-if-changed=src/binary_op_macros.cuh");

    // Build for PTX
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ptx_path = out_dir.join("ptx.rs");
    let mut builder = bindgen_cuda::Builder::default()
        .arg("--expt-relaxed-constexpr")
        .arg("-std=c++17")
        .arg("-O3");
        
    let mut moe_builder = bindgen_cuda::Builder::default()
        .arg("--expt-relaxed-constexpr")
        .arg("-std=c++17")
        .arg("-O3");

    // Discover MSVC toolchain to run headless on Windows without a Developer Prompt
    let mut msvc_cl_path = None;
    if let Ok(target) = env::var("TARGET") {
        if target.contains("msvc") {
            let cl_tool = cc::Build::new().target(&target).get_compiler();
            msvc_cl_path = Some(cl_tool.path().to_path_buf());
            for (key, val) in cl_tool.env() {
                if key == "PATH" {
                    let old = std::env::var_os("PATH").unwrap_or_default();
                    let mut new_path = std::ffi::OsString::new();
                    new_path.push(&val);
                    new_path.push(";");
                    new_path.push(old);
                    std::env::set_var(key, new_path);
                } else {
                    std::env::set_var(key, val);
                }
            }
        }
    }

    if let Some(ref path) = msvc_cl_path {
        std::env::set_var("NVCC_CCBIN", path);
    }

    // Ergonomics: Fast-fail if cl.exe is missing on Windows to avoid cryptic nvcc errors.
    // We only fail if we didn't already find it via cc and passed it via -ccbin.
    if let Ok(target) = env::var("TARGET") {
        if target.contains("msvc") && msvc_cl_path.is_none() {
            if let Err(e) = std::process::Command::new("cl.exe").arg("/?").output() {
                panic!(
                    "\n\n\
                    ========================================================================\n\
                    FATAL: Microsoft C++ Compiler (cl.exe) not found in PATH!\n\
                    \n\
                    You are targeting MSVC, but 'cl.exe' is inaccessible. \n\
                    NVCC requires cl.exe to compile CUDA kernels natively.\n\
                    \n\
                    FIX:\n\
                    You MUST run this build from within a Developer Command Prompt.\n\
                    Do not use nested shell calls (e.g. `cmd /c vcvars64.bat && cargo ...`).\n\
                    \n\
                    Open your Start Menu, search for 'x64 Native Tools Command Prompt \n\
                    for VS 2022', and execute your cargo build from there.\n\
                    ========================================================================\n\
                    Error trace: {e}\n\n"
                );
            }
        }
    }

    // If the caller explicitly requests CPU-only, skip CUDA kernel compilation.
    if std::env::var("VOX_CANDLE_DEVICE").as_deref() == Ok("cpu") {
        println!("cargo:warning=VOX_CANDLE_DEVICE=cpu: skipping CUDA kernel build");
        // Write an empty ptx.rs so the downstream include! doesn't fail.
        std::fs::write(&ptx_path, "// CPU-only build: no PTX\n").unwrap();
        // Skip rest of execution
        return;
    }

    // Validate nvcc version is present and parse compute capability.
    let nvcc_version = std::process::Command::new("nvcc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());

    if let Some(ver) = &nvcc_version {
        eprintln!(
            "candle-kernels: nvcc detected: {}",
            ver.lines().last().unwrap_or("?")
        );
        let bindings = builder.build_ptx().unwrap();
        bindings.write(&ptx_path).unwrap();
    } else {
        println!("cargo:warning=nvcc not found — attempting to use pre-compiled PTX shims");
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let ptx_src_dir = manifest_dir.join("src").join("ptx");
        
        if ptx_src_dir.is_dir() {
            let mut ptx_rs = String::new();
            let entries = ["affine", "binary", "cast", "conv", "fill", "indexing", "quantized", "reduce", "sort", "ternary", "unary"];
            for name in entries {
                let ptx_file = ptx_src_dir.join(format!("{}.ptx", name));
                if ptx_file.is_file() {
                    let content = std::fs::read_to_string(&ptx_file).unwrap();
                    ptx_rs.push_str(&format!("pub const {}: &str = r#\"{}\"#;\n", name.to_uppercase(), content));
                } else {
                    println!("cargo:warning=Missing pre-compiled PTX for {}", name);
                }
            }
            std::fs::write(&ptx_path, ptx_rs).unwrap();
        } else {
            println!("cargo:warning=Pre-compiled PTX directory not found at {:?}", ptx_src_dir);
            // Write an empty ptx.rs so the downstream include! doesn't fail.
            std::fs::write(&ptx_path, "// nvcc missing and no shims found\n").unwrap();
        }
    }

    // Remove unwanted MOE PTX constants from ptx.rs
    remove_lines(&ptx_path, &["MOE_GGUF", "MOE_WMMA", "MOE_WMMA_GGUF"]);

    // Build for FFI binding (must use custom bindgen_cuda, which supports simutanously build PTX and lib)
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut is_target_msvc = false;
    if let Ok(target) = std::env::var("TARGET") {
        if target.contains("msvc") {
            is_target_msvc = true;
            moe_builder = moe_builder.arg("-D_USE_MATH_DEFINES");
        }
    }

    if !is_target_msvc {
        moe_builder = moe_builder.arg("-Xcompiler").arg("-fPIC");
    }

    if nvcc_version.is_some() {
        let moe_builder = moe_builder.kernel_paths(vec![
            "src/moe/moe_gguf.cu",
            "src/moe/moe_wmma.cu",
            "src/moe/moe_wmma_gguf.cu",
        ]);
        moe_builder.build_lib(out_dir.join("libmoe.a"));
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rustc-link-lib=moe");
        println!("cargo:rustc-link-lib=dylib=cudart");
        if !is_target_msvc {
            println!("cargo:rustc-link-lib=stdc++");
        }
    } else {
        println!("cargo:warning=nvcc not found — skipping libmoe build (some MOE features will be missing)");
    }
}

fn remove_lines<P: AsRef<std::path::Path>>(file: P, patterns: &[&str]) {
    let content = std::fs::read_to_string(&file).unwrap();
    let filtered = content
        .lines()
        .filter(|line| !patterns.iter().any(|p| line.contains(p)))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(file, filtered).unwrap();
}
