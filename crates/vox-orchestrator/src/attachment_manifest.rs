use serde::{Deserialize, Serialize};

/// AttachmentManifest: Explicit routing for visual and blob attachments.
/// Eradicates heuristic-based vision inference in favor of SHA-256 gated capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AttachmentManifest {
    pub attachments: Vec<AttachmentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AttachmentEntry {
    /// Unique SHA-256 hash of the attachment content.
    pub sha256: String,
    /// MIME type (e.g. image/png, text/html, application/json).
    pub mime_type: String,
    /// Human-friendly name or relative path.
    pub label: String,
    /// Bounding boxes or coordinate hints associated with this attachment (optional).
    pub visual_segments: Option<Vec<VisualSegment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VisualSegment {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl AttachmentManifest {
    /// Returns true if the manifest contains any vision-eligible attachments (images).
    pub fn has_vision_vitals(&self) -> bool {
        self.attachments.iter().any(|a| a.mime_type.starts_with("image/"))
    }
}
