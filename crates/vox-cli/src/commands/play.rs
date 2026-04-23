use anyhow::{Context, Result};
use std::io::{self, Write};
use vox_compiler::eval::Interpreter;
use vox_compiler::pipeline::{PipelineOptions, run_frontend_str_with_options};

pub async fn run(args: crate::cli_args::PlayArgs) -> Result<()> {
    if args.repl {
        return start_repl().await;
    }

    let name = if let Some(path) = &args.path {
        path.to_string_lossy().to_string()
    } else {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id: u32 = rng.gen_range(1000..9999);
        format!("tmp-vox-{}", id)
    };

    println!("Scaffolding temporary project: {}", name);
    crate::commands::init::run(Some(&name), Some("application"), Some("web")).await?;

    let app_dir = std::env::current_dir()?.join(&name);
    println!("Entering {} and starting dev server...", name);

    let exe = std::env::current_exe().unwrap_or_else(|_| "vox".into());
    let mut child = std::process::Command::new(exe)
        .arg("run")
        .arg("src/main.vox")
        .current_dir(&app_dir)
        .spawn()
        .context("Failed to start vox run")?;

    child.wait()?;
    Ok(())
}

async fn start_repl() -> Result<()> {
    println!("Vox REPL (Zero-install mode)");
    println!("Type expressions or declarations. Type 'exit' or Ctrl+C to quit.");

    let mut interp = Interpreter::new(100_000); // 100k step limit
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();

    loop {
        print!("vox> ");
        stdout.flush()?;
        input.clear();
        if stdin.read_line(&mut input)? == 0 {
            println!();
            break;
        }
        let line = input.trim();
        if line == "exit" || line == "quit" {
            break;
        }
        if line.is_empty() {
            continue;
        }

        let options = PipelineOptions {
            script_mode: true,
            ..Default::default()
        };

        match run_frontend_str_with_options(line, "repl.vox", &options) {
            Ok(res) => {
                if res.has_errors() {
                    for diag in res.diagnostics {
                        if diag.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error
                        {
                            eprintln!("Error: {}", diag.message);
                        }
                    }
                } else {
                    // Load module into interpreter (defines functions, etc.)
                    if let Err(e) = interp.run_module(&res.hir) {
                        eprintln!("Lowering Error: {:?}", e);
                        continue;
                    }

                    // Call 'main' if it was generated (for statements/expressions)
                    // We check if "main" exists in the module functions first
                    if res.hir.functions.iter().any(|f| f.name == "main") {
                        match interp.call("main", vec![]) {
                            Ok(val) => {
                                if val != vox_compiler::eval::value::VoxValue::Null {
                                    println!("{:?}", val);
                                }
                            }
                            Err(e) => {
                                eprintln!("Eval Error: {:?}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Compiler Error: {}", e);
            }
        }
    }

    Ok(())
}
