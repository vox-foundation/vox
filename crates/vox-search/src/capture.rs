//! Injection seam for web-hit capture: vox-search never writes web text to the searchable
//! corpus. A caller that wants hits kept injects a [`WebFindingCapture`] sink (the MCP server
//! routes it into the `kb_findings` inbox); with no sink, web hits persist nowhere.

use vox_db::kb_text::FindingOrigin;

/// One web hit handed to a capture sink.
#[derive(Debug, Clone)]
pub struct CapturedWebHit {
    pub url: String,
    pub title: String,
    pub snippet: String,
    /// Full page/answer text when the provider returned more than a snippet.
    pub full_text: Option<String>,
    pub engine: String,
    pub query: String,
    pub origin: FindingOrigin,
}

/// Fire-and-forget sink; the implementor owns spawning and error logging.
pub trait WebFindingCapture: Send + Sync {
    fn capture(&self, hits: Vec<CapturedWebHit>);
}
