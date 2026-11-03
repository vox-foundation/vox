pub mod client;
/// Catalog handler `commands::gui::drive`.
pub mod drive;
pub mod launch;
pub mod session;

pub async fn run(args: crate::cli_args::GuiArgs) -> anyhow::Result<()> {
    match args.cmd {
        Some(crate::cli_args::GuiCmd::Drive(d)) => drive::run(d).await,
        None => launch::run(args).await,
    }
}
