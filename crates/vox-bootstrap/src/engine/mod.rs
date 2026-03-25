//! Bootstrap evaluation: probe host toolchain; optional `--apply` runs low-risk heals.

mod cmd;
mod evaluate;
mod install;

use std::io::Write;

/// CLI-driven options for probing (and optionally fixing) the host toolchain.
#[derive(Debug, Clone)]
pub struct BootstrapOptions {
    /// Include dev probes (`rustfmt`, `clippy`).
    pub dev: bool,
    /// Treat LLVM/Clang as required on Windows (Turso / aegis native builds).
    pub install_clang: bool,
    /// Run safe heals (`rustup component add`, etc.).
    pub apply: bool,
    /// Install the vox CLI via cargo after successful checks.
    pub install: bool,
    /// Skip release-binary install and force source install.
    pub source_only: bool,
    /// Optional release version/tag override (`v1.2.3`).
    pub version: Option<String>,
}

pub use evaluate::evaluate;

/// Print a human-readable report; returns process exit code (`0` = all required probes passed).
pub fn run_and_print(opts: BootstrapOptions, w: &mut impl Write) -> std::io::Result<i32> {
    let report = evaluate(opts.clone());
    writeln!(w, "Platform: {}", report.platform)?;
    for item in &report.items {
        let status = if item.ok { "OK" } else { "FAIL" };
        writeln!(w, "  [{status}] {} — {}", item.description, item.detail)?;
        if !item.ok
            && let Some(ref h) = item.heal_command
        {
            writeln!(w, "       hint: {h}")?;
        }
    }
    let ok = report.required_ok();
    if ok && opts.install {
        writeln!(w, "\nDependencies met. Installing vox-cli...")?;
        if opts.source_only {
            install::install_from_source(w)?;
        } else {
            match install::install_from_binary(opts.version.as_deref(), w) {
                Ok(()) => {}
                Err(e) => {
                    writeln!(w, "Binary install unavailable: {e}")?;
                    writeln!(
                        w,
                        "Falling back to source install (`cargo install --path crates/vox-cli`)..."
                    )?;
                    install::install_from_source(w)?;
                }
            }
        }
    }
    Ok(if ok { 0 } else { 1 })
}
