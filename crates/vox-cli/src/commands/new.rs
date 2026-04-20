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
    }
}
