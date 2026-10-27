//! ComputerControl extension point — OS-level desktop automation: app and
//! window discovery, accessibility snapshots, window screenshots, and mouse /
//! keyboard input.
//!
//! Methods exchange JSON so actions can grow without ABI churn. Every action
//! targets one application (`{"pid": 123}` or `{"app": "Safari"}`) and reports
//! the input tier that actually ran:
//!
//! - `ax` — accessibility action or value set; no focus change, no cursor motion.
//! - `event` — input posted to the target process; the user's cursor does not move.
//! - `activate` — target raised, input sent, previous app restored.
//!
//! `require_background: true` refuses the `activate` tier.

use abi_stable::{sabi_trait, std_types::*};

pub const COMPUTER_CONTROL_REVISION: u32 = 1;

#[sabi_trait]
pub trait ComputerControl: Send + Sync {
    fn revision(&self) -> u32 {
        COMPUTER_CONTROL_REVISION
    }

    /// Permission and capability report:
    /// `{ "platform": "macos", "accessibility": "granted"|"denied"|"unknown",
    ///    "screen_recording": ..., "background_input": bool, "notes": [..] }`.
    fn permissions(&self) -> RResult<RString, RBoxError>;

    /// JSON array of regular (user-facing) running apps:
    /// `[{ "pid": 1, "name": "...", "bundle_id": "...", "active": bool }]`.
    fn list_apps(&self) -> RResult<RString, RBoxError>;

    /// JSON array of on-screen windows. `options_json` accepts
    /// `{"pid":i64,"app":str}` filters; empty lists all.
    fn list_windows(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Compact accessibility snapshot of the target app's windows with
    /// snapshot-scoped refs: `{ "snapshot_id": "s3", "pid": .., "tree": "...",
    /// "refs": { "e1": {...} }, "truncated": bool }`. `options_json` accepts
    /// `{"target":{"pid"|"app"},"max_depth":u32,"max_nodes":u32,"interactive_only":bool}`.
    fn snapshot(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError>;

    /// PNG screenshot as JSON `{ "image_base64", "width", "height", "scale",
    /// "origin": [x, y] }`. `options_json` accepts `{"target":{..}}` for the
    /// app's front window (else the main display), `"region":[x0,y0,x1,y1]`
    /// in screenshot pixels to zoom, and `"max_long_edge":u32` to downscale.
    fn screenshot(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Perform one action. `action_json`:
    /// `{ "target": {..}, "kind": "click"|"double_click"|"right_click"|"press"|
    ///    "set_value"|"type"|"key"|"scroll"|"move"|"drag"|"focus",
    ///    "snapshot_id": str, "ref": str, "x": f64, "y": f64, "to_x": f64, "to_y": f64,
    ///    "text": str, "key": str, "dx": i64, "dy": i64,
    ///    "tier": "auto"|"ax"|"event"|"activate", "require_background": bool,
    ///    "respect_sensitive": bool }`.
    /// Returns `{ "ok": bool, "tier": str, ... }`; refusals carry `reason`.
    fn act(&self, action_json: RStr<'_>) -> RResult<RString, RBoxError>;
}
