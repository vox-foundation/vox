//! Narrow browser session API backed by Chromium via [`chromiumoxide`] (CDP).
//!
//! Playwright is **not** required. A Chromium-compatible binary must be available
//! (`VOX_CHROME_EXECUTABLE` optional override; otherwise auto-detection).

mod engine;

pub use engine::{BrowserEngine, global_engine};
