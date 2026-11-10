#[derive(clap::Args, Clone, Debug)]
pub struct GuiArgs {
    /// Open directly to a specific command panel.
    #[arg(
        long,
        value_name = "COMMAND",
        help = "Open to a specific command panel"
    )]
    pub command: Option<String>,
    #[command(subcommand)]
    pub cmd: Option<GuiCmd>,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum GuiCmd {
    /// Drive a dedicated debug Axis from the terminal (never the user's window).
    Drive(DriveArgs),
}

#[derive(clap::Parser, Clone, Debug)]
pub struct DriveArgs {
    #[command(subcommand)]
    pub cmd: DriveCmd,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum DriveCmd {
    /// Start a dedicated Axis Drive process.
    Start(DriveStartArgs),
    /// Stop the current Axis Drive process.
    Stop,
    /// Reveal the dedicated Axis Drive window.
    Show,
    /// Update Drive model, execution, or composer knobs.
    Set(DriveSetArgs),
    /// Send a chat turn through the real Axis composer path.
    Send(DriveSendArgs),
    /// Print the current live Drive state as JSON.
    State,
    /// Wait for a reply, error, event, or selectable model.
    Wait(DriveWaitArgs),
    /// Run a one-shot Drive request without a webview.
    Headless(DriveHeadlessArgs),
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveStartArgs {
    #[arg(long)]
    pub show: bool,
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveSetArgs {
    #[arg(long, required = true)]
    pub knob: Vec<String>,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveSendArgs {
    #[arg(long)]
    pub text: String,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveWaitArgs {
    #[arg(long)]
    pub until: String,
    #[arg(long, default_value = "90s")]
    pub timeout: String,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveHeadlessArgs {
    #[command(subcommand)]
    pub cmd: DriveHeadlessCmd,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum DriveHeadlessCmd {
    /// Apply Drive knobs in the one-shot headless plane.
    Set(DriveSetArgs),
    /// Send a one-shot headless chat turn.
    Send(DriveSendArgs),
    /// Print one-shot headless state as JSON.
    State,
}
