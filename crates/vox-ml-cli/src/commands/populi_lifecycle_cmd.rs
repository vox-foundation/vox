//! Clap surface for `vox populi up|down|status`.

use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_BIND: &str = "127.0.0.1:9847";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PopuliConnectivityMode {
    Lan,
    Overlay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OverlayProviderArg {
    Auto,
    Tailscale,
    Wireguard,
    Tunnel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayProvider {
    Tailscale,
    Wireguard,
    Tunnel,
}

impl OverlayProvider {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Tailscale => "tailscale",
            Self::Wireguard => "wireguard",
            Self::Tunnel => "tunnel",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PopuliDaemonState {
    pub(crate) pid: Option<u32>,
    pub(crate) bind: String,
    pub(crate) mode: String,
    pub(crate) control_url: String,
    pub(crate) env_file: String,
    pub(crate) overlay_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OverlayDiagnostics {
    pub(crate) provider: String,
    pub(crate) available: bool,
    pub(crate) connected: bool,
    pub(crate) detail: String,
}

#[derive(Subcommand)]
pub enum PopuliLifecycleCmd {
    /// Start a private local/overlay mesh with secure defaults.
    Up {
        /// Connectivity strategy.
        #[arg(long, value_enum, default_value_t = PopuliConnectivityMode::Lan)]
        mode: PopuliConnectivityMode,
        /// Mesh scope id (auto-generated when omitted).
        #[arg(long)]
        scope: Option<String>,
        /// GPU advertisement policy (`auto` currently maps to env-driven probe defaults).
        #[arg(long, default_value = "auto")]
        gpus: String,
        /// Control-plane bind address.
        #[arg(long, default_value = DEFAULT_BIND)]
        bind: String,
        /// Overlay provider selection (`auto` probes available providers).
        #[arg(long, value_enum, default_value_t = OverlayProviderArg::Auto)]
        overlay_provider: OverlayProviderArg,
        /// Allow local insecure mode (disables required mesh token).
        #[arg(long, default_value_t = false)]
        insecure_local: bool,
        /// Node visibility for task scheduling (`private`, `public`, `hybrid`).
        #[arg(long, default_value = "private")]
        visibility: String,
        /// Opt-in to processing public mesh tasks when idle.
        #[arg(long, default_value_t = false)]
        public_mesh: bool,
        /// Minimum priority for public mesh tasks (0-255).
        #[arg(long, default_value_t = 128)]
        donate_min_priority: u8,
        /// Task kinds allowed for donation (comma-separated, e.g. "text_infer,image_gen").
        #[arg(long, value_delimiter = ',')]
        donate_kinds: Vec<String>,
        /// Explicit whitelist of user IDs allowed to run tasks.
        #[arg(long, value_delimiter = ',')]
        allow_users: Vec<String>,
        /// Explicit blacklist of user IDs denied from running tasks.
        #[arg(long, value_delimiter = ',')]
        deny_users: Vec<String>,
        /// Explicit whitelist of federated mesh networks (scope IDs) to accept tasks from.
        #[arg(long, value_delimiter = ',')]
        allow_meshes: Vec<String>,
        /// Known peer mesh URLs to gossip federation status with (comma-separated).
        #[arg(long, value_delimiter = ',')]
        bootstrap_peers: Vec<String>,
    },
    /// Stop the populi control-plane process started by `vox populi up`.
    Down,
    /// Show populi health, security posture, and overlay diagnostics.
    Status {
        /// Emit JSON output (also implied by root `--json`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
