use std::path::Path;
use std::fs;

fn main() {
    let dist_dir = Path::new("dist");
    let index_file = dist_dir.join("index.html");

    if !dist_dir.exists() || !index_file.exists() {
        println!("cargo:warning=dist/ missing; run `pnpm install && pnpm run build` in crates/vox-dashboard before building with --features embedded-assets");
        
        let _ = fs::create_dir_all(&dist_dir);
        let _ = fs::write(&index_file, "<html><body>Dashboard bundle not built.</body></html>");
    }

    // Rebuild if dist/ changes
    println!("cargo:rerun-if-changed=dist");
}
