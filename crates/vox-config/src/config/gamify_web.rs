//! Gamification and web run-mode enums for `VoxConfig`.

use serde::{Deserialize, Serialize};

/// Gamification UX / reward tuning (consumed by `vox-ludus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GamifyMode {
    #[default]
    Balanced,
    Serious,
    Learning,
}

impl GamifyMode {
    /// Lower-case slug persisted in `Vox.toml` / `~/.vox/config.toml` (`[vox].gamify_mode`).
    #[must_use]
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Serious => "serious",
            Self::Learning => "learning",
        }
    }

    /// Multiplier applied to XP / crystal grants where ludus uses mode-aware scaling.
    #[must_use]
    pub fn reward_multiplier(self) -> f64 {
        match self {
            Self::Balanced => 1.0,
            Self::Serious => 0.5,
            Self::Learning => 1.25,
        }
    }

    /// Relative hint frequency in `[0.0, 1.0]` for coaching surfaces.
    #[must_use]
    pub fn hint_frequency(self) -> f64 {
        match self {
            Self::Balanced => 0.5,
            Self::Serious => 0.0,
            Self::Learning => 1.0,
        }
    }

    /// Whether level-up / quest-complete style overlays should render.
    #[must_use]
    pub fn show_overlays(self) -> bool {
        !matches!(self, Self::Serious)
    }
}

/// How `vox run` (and compilerd) choose the **script** lane vs **web app** lane when the CLI mode is `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebRunMode {
    #[default]
    Auto,
    App,
    Script,
}

impl WebRunMode {
    /// Lower-case slug in `Vox.toml` `[web] run_mode` and `VOX_WEB_RUN_MODE`.
    #[must_use]
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::App => "app",
            Self::Script => "script",
        }
    }
}
