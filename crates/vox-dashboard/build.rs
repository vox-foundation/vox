use std::path::Path;
use std::fs;

fn main() {
    // TASK-7.1: compile .vox source → generated TSX before the pnpm/Vite bundle step.
    //
    // When app/src/app.vox is present, invoke `vox build` to emit TSX into
    // app/src/generated/. The Vite config then imports from that generated output.
    // Soft failure: if `vox` is not in PATH the pnpm build falls back to the
    // hand-written App.tsx until TASK-7.2 completes visual parity.
    let vox_entry = Path::new("app/src/app.vox");
    if vox_entry.exists() {
        let status = std::process::Command::new("vox")
            .args(["build", "app/src/app.vox", "--out-dir", "app/src/generated"])
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=vox-dashboard: app.vox compiled to app/src/generated/");
            }
            Ok(s) => {
                println!(
                    "cargo:warning=vox-dashboard: `vox build` exited with {s}; \
                     check app/src/app.vox for syntax errors"
                );
            }
            Err(_) => {
                println!(
                    "cargo:warning=vox-dashboard: `vox` not in PATH — skipping .vox compilation. \
                     Run `vox build app/src/app.vox --out-dir app/src/generated` manually."
                );
            }
        }

        // Rerun if any .vox source changes.
        println!("cargo:rerun-if-changed=app/src/app.vox");
        for tab in &["speak", "command", "network", "forge"] {
            println!("cargo:rerun-if-changed=app/src/tabs/{tab}.vox");
        }
    }

    let dist_dir = Path::new("dist");
    let index_file = dist_dir.join("index.html");

    if !dist_dir.exists() || !index_file.exists() {
        println!(
            "cargo:warning=dist/ missing; run `pnpm install && pnpm run build` \
             in crates/vox-dashboard before building with --features embedded-assets"
        );

        let _ = fs::create_dir_all(&dist_dir);
        let _ = fs::write(&index_file, "<html><body>Dashboard bundle not built.</body></html>");
    }

    println!("cargo:rerun-if-changed=dist");
}
