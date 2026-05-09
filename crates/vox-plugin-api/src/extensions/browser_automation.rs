//! BrowserAutomation extension point — Chrome DevTools Protocol (CDP) and
//! similar browser automation backends.
//!
//! The trait exposes a session-based model: `open` creates a new tab and
//! returns an opaque `page_id`; subsequent calls reference that id. This
//! mirrors the `vox-browser` `BrowserEngine` API.

use abi_stable::{sabi_trait, std_types::*};

pub const BROWSER_AUTOMATION_REVISION: u32 = 1;

#[sabi_trait]
pub trait BrowserAutomation: Send + Sync {
    fn revision(&self) -> u32 {
        BROWSER_AUTOMATION_REVISION
    }

    /// Open a new tab navigated to `url` (headless if `headless` is true).
    /// Returns an opaque `page_id` token.
    fn open(&self, url: RStr<'_>, headless: bool) -> RResult<RString, RBoxError>;

    /// Navigate an existing tab to a new URL.
    fn goto(&self, page_id: RStr<'_>, url: RStr<'_>) -> RResult<(), RBoxError>;

    /// Click the element identified by `target` (CSS selector or `xpath:...`).
    fn click(&self, page_id: RStr<'_>, target: RStr<'_>) -> RResult<(), RBoxError>;

    /// Fill a form field identified by `target` with `value`.
    fn fill(&self, page_id: RStr<'_>, target: RStr<'_>, value: RStr<'_>) -> RResult<(), RBoxError>;

    /// Block until `target` appears in the DOM or `timeout_secs` elapses.
    fn wait_for(
        &self,
        page_id: RStr<'_>,
        target: RStr<'_>,
        timeout_secs: u64,
    ) -> RResult<(), RBoxError>;

    /// Inner text of the element identified by `target`.
    fn text(&self, page_id: RStr<'_>, target: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Outer HTML of `target`, or full document HTML if `target` is empty.
    fn html(&self, page_id: RStr<'_>, target: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Take a PNG screenshot of the full page. Returns raw PNG bytes.
    fn screenshot_bytes(&self, page_id: RStr<'_>) -> RResult<RVec<u8>, RBoxError>;

    /// Take a PNG screenshot and save it to `path`. Returns the resolved path.
    fn screenshot(&self, page_id: RStr<'_>, path: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Trimmed page text suitable for LLM prompts (HTML tags stripped).
    /// `max_chars` 0 is treated as 256.
    fn visible_text_summary(
        &self,
        page_id: RStr<'_>,
        max_chars: u64,
    ) -> RResult<RString, RBoxError>;

    /// Full AX tree as a JSON string.
    fn ax_tree(&self, page_id: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Close the tab and release its CDP resources.
    fn close(&self, page_id: RStr<'_>) -> RResult<(), RBoxError>;
}
