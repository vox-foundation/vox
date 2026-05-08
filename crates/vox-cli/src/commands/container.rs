use anyhow::{Context, Result};
use std::path::PathBuf;
use vox_deploy_codegen::{run_py_setup, PySetupOpts};
use vox_compiler::ast::decl::Decl;

/// Manage Vox OCI images (Rust build/run). **`container init`** for `@py.import` is retired.
pub async fn run(action: ContainerAction) -> Result<()> {
    match action {
        ContainerAction::Init { file, out_dir, dockerfile, project_name } => {
            println!("🛠️ Inspecting Vox file for legacy @py.import (retired Python/UV lane)...");

            let source = read_utf8_path_capped_async(&file)
                .await
                .with_context(|| format!("Failed to read source file: {}", file.display()))?;
            let tokens = vox_compiler::lexer::cursor::lex(&source);
            let module = vox_compiler::parser::parse(tokens)
                .map_err(|e| anyhow::anyhow!("Parse errors in {}: {:?}", file.display(), e))?;

            let py_imports: Vec<String> = module
                .declarations
                .iter()
                .filter_map(|d| match d {
                    Decl::PyImport(p) => Some(p.module.clone()),
                    _ => None,
                })
                .collect();

            if py_imports.is_empty() {
                println!(
                    "ℹ️ No @py.import declarations found in {}. Nothing to do.",
                    file.display()
                );
                return Ok(());
            }

            let name = project_name.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("vox-app")
                    .to_string()
            });

            let opts = PySetupOpts {
                project_name: name,
                py_imports,
                generate_dockerfile: dockerfile,
                out_dir: out_dir.unwrap_or_else(|| PathBuf::from(".")),
            };

            tokio::task::spawn_blocking(move || run_py_setup(&opts))
                .await
                .map_err(|e| anyhow::anyhow!("run_py_setup task: {e}"))?
                .context("Python container setup failed")?;
        }
        ContainerAction::Build { tag, runtime } => {
            let tag = tag.unwrap_or_else(|| "vox-app:latest".to_string());
            println!("📦 Building container image: {}", tag);

            let pref = runtime
                .unwrap_or_else(|| "auto".to_string())
                .parse::<vox_plugin_runtime_container::RuntimePreference>()
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let cwd = std::env::current_dir()?;
            let opts = vox_container::BuildOpts {
                context_dir: cwd,
                dockerfile: None,
                tag,
                build_args: vec![],
            };

            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let rt = vox_plugin_runtime_container::detect_runtime(pref).context(
                    "No container runtime available. Install Docker or Podman.",
                )?;
                let _ = rt.build(&opts).context("Container build failed")?;
                Ok(())
            })
            .await
            .map_err(|e| anyhow::anyhow!("container build task: {e}"))??;

            println!("✓ Image built successfully.");
        }
        ContainerAction::Run { image, port, runtime } => {
            let image = image.unwrap_or_else(|| "vox-app:latest".to_string());
            println!("🚀 Running container image: {}", image);

            let pref = runtime
                .unwrap_or_else(|| "auto".to_string())
                .parse::<vox_plugin_runtime_container::RuntimePreference>()
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let opts = vox_container::RunOpts {
                image,
                ports: port.map(|p| vec![(p, p)]).unwrap_or_default(),
                env: vec![],
                volumes: vec![],
                detach: false,
                name: None,
                rm: true,
            };

            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let rt = vox_plugin_runtime_container::detect_runtime(pref).context(
                    "No container runtime available. Install Docker or Podman.",
                )?;
                rt.run(&opts).context("Container run failed")?;
                Ok(())
            })
            .await
            .map_err(|e| anyhow::anyhow!("container run task: {e}"))??;
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand, Clone)]
pub enum ContainerAction {
    /// Retired: used to scan `@py.import` (now errors — use Rust/PM flows)
    Init {
        /// Path to the .vox source file to parse for @py.import declarations
        #[arg(short, long, required = true)]
        file: std::path::PathBuf,
        /// Output directory for pyproject.toml (and optional Dockerfile)
        #[arg(short, long)]
        out_dir: Option<std::path::PathBuf>,
        /// Also generate a CUDA-aware Dockerfile in the output directory
        #[arg(long)]
        dockerfile: bool,
        /// Override the project name used in pyproject.toml
        #[arg(long)]
        project_name: Option<String>,
    },
    /// Build an OCI container image from the current directory
    Build {
        /// Image tag (default: vox-app:latest)
        #[arg(short, long)]
        tag: Option<String>,
        /// Container runtime: auto, docker, podman (default: auto)
        #[arg(long, default_value = "auto")]
        runtime: Option<String>,
    },
    /// Run a container image locally
    Run {
        /// Image to run (default: vox-app:latest)
        #[arg(short, long)]
        image: Option<String>,
        /// Host port to expose (mapped to the same container port)
        #[arg(short, long)]
        port: Option<u16>,
        /// Container runtime: auto, docker, podman (default: auto)
        #[arg(long, default_value = "auto")]
        runtime: Option<String>,
    },
}
