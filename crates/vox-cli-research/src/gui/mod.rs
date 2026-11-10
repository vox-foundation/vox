pub mod args;
pub mod client;
/// Catalog handler `commands::gui::drive`.
pub mod drive;
pub mod launch;
pub mod session;

pub use args::*;

pub async fn run(args: GuiArgs) -> anyhow::Result<()> {
    match args.cmd {
        Some(GuiCmd::Drive(d)) => drive::run(d).await,
        None => launch::run(args).await,
    }
}
