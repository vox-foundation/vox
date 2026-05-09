use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum NewCmd {
    /// Scaffold a production-ready TanStack Start web application
    Web {
        /// Project / directory name
        name: Option<String>,
    },
    /// Scaffold a new skill
    Skill {
        /// Skill name
        name: Option<String>,
    },
    /// Scaffold a Vox fn stub paired with a failing `@test` block (Test-First Policy).
    Fn {
        /// Function name (Vox identifier).
        name: String,
        /// File to append to (created if missing). Defaults to `src/main.vox`.
        #[arg(long = "in", value_name = "PATH")]
        in_path: Option<PathBuf>,
        /// Parameter list, e.g. "a: int, b: int".
        #[arg(long, value_name = "LIST")]
        params: Option<String>,
        /// Return type, e.g. "int". Omit for Unit-returning fns.
        #[arg(long, value_name = "TYPE")]
        returns: Option<String>,
        /// Print the rendered stub to stdout instead of writing to a file.
        #[arg(long)]
        stdout: bool,
        /// Reserved for future use; today this flag still refuses to clobber
        /// an existing fn of the same name (replacement is not yet supported).
        #[arg(long)]
        force: bool,
    },
}

pub async fn run(cmd: NewCmd) -> Result<()> {
    match cmd {
        NewCmd::Web { name } => {
            // "web" is an alias for our TanStack Start (dashboard/application) template
            crate::commands::init::run(name.as_deref(), Some("application"), Some("web")).await
        }
        NewCmd::Skill { name } => {
            crate::commands::init::run(name.as_deref(), Some("skill"), None).await
        }
        NewCmd::Fn {
            name,
            in_path,
            params,
            returns,
            stdout,
            force,
        } => run_fn_scaffold(&name, in_path, params.as_deref(), returns.as_deref(), stdout, force),
    }
}

fn run_fn_scaffold(
    name: &str,
    in_path: Option<PathBuf>,
    params: Option<&str>,
    returns: Option<&str>,
    stdout: bool,
    force: bool,
) -> Result<()> {
    if stdout {
        let rendered = vox_project_scaffold::render_fn_stub(name, params, returns)?;
        print!("{rendered}");
        return Ok(());
    }

    let target = in_path.unwrap_or_else(|| PathBuf::from("src/main.vox"));
    let bytes = vox_project_scaffold::append_fn_stub(&target, name, params, returns, force)?;
    eprintln!(
        "Scaffolded `fn {name}` + `@test fn test_{name}` into {} ({bytes} bytes). \
         Open the file and replace `_` / `_expected` with concrete values, then implement until the test passes.",
        target.display()
    );
    Ok(())
}
