//! CLI entry for host bootstrap (`cargo run -p vox-bootstrap`).

use std::io::{self, Write};

use clap::{Parser, Subcommand};
use vox_bootstrap::{BootstrapOptions, evaluate, run_and_print};

#[derive(Parser)]
#[command(
    name = "vox-bootstrap",
    about = "Probe/fix host toolchain for building Vox (Rust, MSVC, clang for Turso/aegis)"
)]
struct Cli {
    /// Include dev probes (rustfmt, clippy).
    #[arg(long)]
    dev: bool,
    /// When used with --apply, install clang / LLVM for Turso (aegis) builds.
    #[arg(long)]
    install_clang: bool,
    /// Perform mutations: rustup components; Windows: winget LLVM; Linux: sudo apt/dnf; macOS: brew llvm.
    #[arg(long)]
    apply: bool,
    /// Actually install the vox CLI (`cargo install --path crates/vox-cli`) after successful probe.
    #[arg(long)]
    install: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the evaluated plan (JSON default; use --human for debug).
    Plan {
        /// Pretty-print Debug instead of JSON.
        #[arg(long)]
        human: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let opts = BootstrapOptions {
        dev: cli.dev,
        install_clang: cli.install_clang,
        apply: cli.apply,
        install: cli.install,
    };

    match cli.command {
        Some(Commands::Plan { human }) => {
            let report = evaluate(BootstrapOptions {
                dev: opts.dev,
                install_clang: opts.install_clang,
                apply: false,
                install: false,
            });
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            let res = if human {
                writeln!(lock, "{report:#?}")
            } else {
                serde_json::to_writer_pretty(&mut lock, &report)
                    .map_err(io::Error::other)
                    .and_then(|()| writeln!(lock))
            };
            if let Err(e) = res {
                eprintln!("vox-bootstrap plan: {e}");
                std::process::exit(2);
            }
            let code = if report.required_ok() { 0 } else { 1 };
            std::process::exit(code);
        }
        None => match run_and_print(opts, &mut io::stdout()) {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("vox-bootstrap: {e}");
                std::process::exit(2);
            }
        },
    }
}
