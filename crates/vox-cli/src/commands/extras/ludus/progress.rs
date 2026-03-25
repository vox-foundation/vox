//! Shared terminal progress rendering.

use owo_colors::OwoColorize;

/// Render a progress bar.
pub fn render_progress_bar(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}]",
        "█".repeat(filled).bright_green(),
        "░".repeat(empty).dimmed(),
    )
}
