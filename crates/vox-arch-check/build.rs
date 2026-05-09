fn main() {
    vox_build_meta::emit();
    println!("cargo:rerun-if-changed=build.rs");
}
