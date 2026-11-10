//! Diagnostic output helpers for the Vox CLI.

use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::OnceLock;

/// Global color choice for the CLI.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ColorChoice {
    /// Detect if stdout/stderr is a TTY (default)
    #[default]
    Auto,
    /// Always use ANSI colors
    Always,
    /// Never use ANSI colors
    Never,
}

static COLOR_CHOICE: OnceLock<ColorChoice> = OnceLock::new();

/// Set the global color choice.
pub fn set_color_choice(choice: ColorChoice) {
    let _ = COLOR_CHOICE.set(choice);
}

/// Returns true if ANSI colors should be used for diagnostics (stderr).
pub fn should_color_stderr() -> bool {
    match COLOR_CHOICE.get().copied().unwrap_or(ColorChoice::Auto) {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if std::env::var("NO_COLOR").is_ok() {
                return false;
            }
            std::io::stderr().is_terminal()
        }
    }
}

/// Returns true if ANSI colors should be used for data output (stdout).
pub fn should_color_stdout() -> bool {
    match COLOR_CHOICE.get().copied().unwrap_or(ColorChoice::Auto) {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if std::env::var("NO_COLOR").is_ok() {
                return false;
            }
            std::io::stdout().is_terminal()
        }
    }
}

/// Print a formatted error to stderr.
#[inline(never)]
pub fn print_error(message: &str) {
    if should_color_stderr() {
        eprintln!("{} {message}", "❌ error:".red().bold());
    } else {
        eprintln!("❌ error: {message}");
    }
}

/// Print a formatted success line to stdout.
#[inline(never)]
pub fn print_success(message: &str) {
    if should_color_stdout() {
        println!("{} {message}", "✓".green().bold());
    } else {
        println!("✓ {message}");
    }
}

/// Print a neutral info line to stdout (no color, plain).
#[inline(never)]
pub fn print_info(message: &str) {
    println!("{message}");
}

/// One environment check row for human or JSON output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoctorCheck {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

impl DoctorCheck {
    pub fn new(name: impl Into<String>, pass: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pass,
            detail: detail.into(),
        }
    }

    pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pass: true,
            detail: detail.into(),
        }
    }

    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pass: false,
            detail: detail.into(),
        }
    }
}
