//! Platform-neutral request model, snapshot rendering, and ref bookkeeping.
//!
//! Backends only supply raw primitives (accessibility nodes, posted input,
//! captured pixels); everything an agent sees — refs, the compact tree,
//! coordinate mapping, refusals — is produced here so every OS behaves the same.

use std::collections::VecDeque;

use serde::Deserialize;
use serde_json::{Value, json};

pub const DEFAULT_MAX_DEPTH: u32 = 25;
pub const DEFAULT_MAX_NODES: u32 = 400;
pub const DEFAULT_MAX_LONG_EDGE: u32 = 1568;
const SNAPSHOT_HISTORY: usize = 8;
const MAX_NAME_CHARS: usize = 120;
const MAX_VALUE_CHARS: usize = 200;

/// Apps whose windows are never read or driven: password managers and the
/// system keychain. Matched case-insensitively on bundle id or app name.
const DENIED_APPS: &[&str] = &[
    "com.1password.1password",
    "com.agilebits.onepassword7",
    "com.bitwarden.desktop",
    "com.apple.keychainaccess",
    "com.apple.passwords",
    "com.lastpass.lastpass",
    "org.keepassxc.keepassxc",
    "1password",
    "bitwarden",
    "keychain access",
    "passwords",
    "lastpass",
    "keepassxc",
];

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub pid: Option<i64>,
    pub app: Option<String>,
}

impl Target {
    pub fn is_empty(&self) -> bool {
        self.pid.is_none() && self.app.is_none()
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    #[default]
    Auto,
    Ax,
    Event,
    Activate,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Auto => "auto",
            Tier::Ax => "ax",
            Tier::Event => "event",
            Tier::Activate => "activate",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Coords {
    /// Pixels of the most recent full screenshot of the target.
    #[default]
    Screenshot,
    /// Global screen points.
    Screen,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Click,
    DoubleClick,
    RightClick,
    Press,
    SetValue,
    Type,
    Key,
    Scroll,
    Move,
    Drag,
    Focus,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::Click => "click",
            ActionKind::DoubleClick => "double_click",
            ActionKind::RightClick => "right_click",
            ActionKind::Press => "press",
            ActionKind::SetValue => "set_value",
            ActionKind::Type => "type",
            ActionKind::Key => "key",
            ActionKind::Scroll => "scroll",
            ActionKind::Move => "move",
            ActionKind::Drag => "drag",
            ActionKind::Focus => "focus",
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRequest {
    #[serde(default)]
    pub target: Target,
    pub kind: ActionKind,
    pub snapshot_id: Option<String>,
    #[serde(rename = "ref")]
    pub ref_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub to_x: Option<f64>,
    pub to_y: Option<f64>,
    #[serde(default)]
    pub coords: Coords,
    pub text: Option<String>,
    pub key: Option<String>,
    pub dx: Option<i64>,
    pub dy: Option<i64>,
    #[serde(default)]
    pub tier: Tier,
    #[serde(default)]
    pub require_background: bool,
    #[serde(default = "default_true")]
    pub respect_sensitive: bool,
}

impl ActionRequest {
    pub fn has_point(&self) -> bool {
        self.x.is_some() && self.y.is_some()
    }

    /// Shape check before anything touches the OS.
    pub fn validate(&self) -> Result<(), String> {
        if self.target.is_empty() {
            return Err("target is required: {\"pid\":..} or {\"app\":..}".into());
        }
        if self.require_background && self.tier == Tier::Activate {
            return Err("tier \"activate\" contradicts require_background".into());
        }
        let has_ref = self.ref_id.is_some();
        match self.kind {
            ActionKind::Click
            | ActionKind::DoubleClick
            | ActionKind::RightClick
            | ActionKind::Move
                if !has_ref && !self.has_point() =>
            {
                Err(format!(
                    "{} needs \"ref\" or \"x\"/\"y\"",
                    self.kind.as_str()
                ))
            }
            ActionKind::Press | ActionKind::Focus if !has_ref => {
                Err(format!("{} needs \"ref\"", self.kind.as_str()))
            }
            ActionKind::SetValue if !has_ref || self.text.is_none() => {
                Err("set_value needs \"ref\" and \"text\"".into())
            }
            ActionKind::Type if self.text.as_deref().is_none_or(str::is_empty) => {
                Err("type needs non-empty \"text\"".into())
            }
            ActionKind::Key => {
                let key = self
                    .key
                    .as_deref()
                    .ok_or("key needs \"key\" (e.g. \"cmd+s\")")?;
                parse_chord(key).map(|_| ())
            }
            ActionKind::Scroll if self.dx.unwrap_or(0) == 0 && self.dy.unwrap_or(0) == 0 => {
                Err("scroll needs non-zero \"dx\" or \"dy\"".into())
            }
            ActionKind::Drag
                if !(self.has_point() && self.to_x.is_some() && self.to_y.is_some()) =>
            {
                Err("drag needs \"x\",\"y\",\"to_x\",\"to_y\"".into())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotOptions {
    #[serde(default)]
    pub target: Target,
    pub max_depth: Option<u32>,
    pub max_nodes: Option<u32>,
    #[serde(default)]
    pub interactive_only: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenshotOptions {
    pub target: Option<Target>,
    pub region: Option<[f64; 4]>,
    pub max_long_edge: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowFilter {
    pub pid: Option<i64>,
    pub app: Option<String>,
}

/// Parse JSON options; empty input means defaults.
pub fn parse_options<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, String> {
    let trimmed = json.trim();
    let src = if trimmed.is_empty() { "{}" } else { trimmed };
    serde_json::from_str(src).map_err(|e| format!("invalid options: {e}"))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub mods: Modifiers,
    /// Canonical key name: a single character, or one of [`NAMED_KEYS`].
    pub key: String,
}

pub const NAMED_KEYS: &[&str] = &[
    "enter",
    "tab",
    "escape",
    "space",
    "backspace",
    "delete",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

/// Parse `"cmd+shift+t"`, `"ctrl+s"`, `"Return"`, `"Page_Down"` (xdotool and
/// Anthropic computer-use spellings are accepted).
pub fn parse_chord(spec: &str) -> Result<Chord, String> {
    let mut mods = Modifiers::default();
    let mut key = None;
    for raw in spec.split('+').map(str::trim) {
        if raw.is_empty() {
            return Err(format!("invalid key chord {spec:?}"));
        }
        let part = raw.to_ascii_lowercase().replace(['_', '-'], "");
        match part.as_str() {
            "cmd" | "command" | "meta" | "super" | "win" | "windows" => mods.cmd = true,
            "ctrl" | "control" => mods.ctrl = true,
            "alt" | "option" | "opt" => mods.alt = true,
            "shift" => mods.shift = true,
            _ if key.is_some() => return Err(format!("chord {spec:?} has more than one key")),
            _ => key = Some(canonical_key(raw, &part)?),
        }
    }
    let key = key.ok_or_else(|| format!("chord {spec:?} has no key"))?;
    Ok(Chord { mods, key })
}

fn canonical_key(raw: &str, lowered: &str) -> Result<String, String> {
    let alias = match lowered {
        "return" | "kpenter" => "enter",
        "esc" => "escape",
        "del" | "forwarddelete" => "delete",
        "back" => "backspace",
        "arrowup" => "up",
        "arrowdown" => "down",
        "arrowleft" => "left",
        "arrowright" => "right",
        "pgup" | "prior" => "pageup",
        "pgdn" | "next" => "pagedown",
        other => other,
    };
    if NAMED_KEYS.contains(&alias) {
        return Ok(alias.to_string());
    }
    let mut chars = raw.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c.to_lowercase().collect()),
        _ => Err(format!("unknown key {raw:?}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }
}

/// Maps screenshot pixels back to global screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub origin_x: f64,
    pub origin_y: f64,
    /// Image pixels per screen point.
    pub scale: f64,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    pub fn to_screen(self, px: f64, py: f64) -> Result<(f64, f64), String> {
        if px < 0.0 || py < 0.0 || px > f64::from(self.width) || py > f64::from(self.height) {
            return Err(format!(
                "point ({px}, {py}) is outside the {}x{} screenshot",
                self.width, self.height
            ));
        }
        Ok((
            self.origin_x + px / self.scale,
            self.origin_y + py / self.scale,
        ))
    }
}

/// Output pixel size for capturing `w_pts`×`h_pts` at `native_scale`, capped so
/// the long edge is at most `max_long_edge`. Returns `(width, height, scale)`.
pub fn fit_size(w_pts: f64, h_pts: f64, native_scale: f64, max_long_edge: u32) -> (u32, u32, f64) {
    let long = w_pts.max(h_pts).max(1.0);
    let scale = native_scale.min(f64::from(max_long_edge.max(1)) / long);
    let w = (w_pts * scale).round().max(1.0) as u32;
    let h = (h_pts * scale).round().max(1.0) as u32;
    (w, h, scale)
}

/// Neutral role names; backends map native roles onto these.
pub const INTERACTIVE_ROLES: &[&str] = &[
    "button",
    "checkbox",
    "radio",
    "switch",
    "textbox",
    "secure_textbox",
    "searchbox",
    "combobox",
    "popup",
    "menuitem",
    "menubaritem",
    "link",
    "slider",
    "tab",
    "row",
    "cell",
    "disclosure",
    "incrementor",
];

#[derive(Debug, Clone)]
pub struct Node<H> {
    pub handle: H,
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub enabled: bool,
    pub focused: bool,
    pub secure: bool,
    pub frame: Option<Rect>,
    /// Native accessibility actions, normalized (`"press"`, `"show_menu"`, ...).
    pub actions: Vec<String>,
    pub children: Vec<Node<H>>,
}

impl<H> Node<H> {
    pub fn is_interactive(&self) -> bool {
        INTERACTIVE_ROLES.contains(&self.role.as_str()) || self.actions.iter().any(|a| a == "press")
    }

    fn is_noise(&self) -> bool {
        !self.is_interactive()
            && self.name.is_empty()
            && self.value.as_deref().is_none_or(str::is_empty)
            && matches!(
                self.role.as_str(),
                "group" | "other" | "scrollarea" | "splitgroup" | "layout"
            )
    }
}

#[derive(Debug, Clone)]
pub struct RefEntry<H> {
    pub handle: H,
    pub role: String,
    pub name: String,
    pub frame: Option<Rect>,
    pub secure: bool,
    pub actions: Vec<String>,
}

impl<H> RefEntry<H> {
    pub fn supports(&self, action: &str) -> bool {
        self.actions.iter().any(|a| a == action)
    }
}

pub struct Rendered<H> {
    pub tree: String,
    pub refs: Vec<(String, RefEntry<H>)>,
    pub truncated: bool,
}

/// Bidi marks and zero-width characters apps embed in labels; dropped so
/// names read (and match) as plain text.
fn invisible(c: char) -> bool {
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}')
}

fn clip(s: &str, max: usize) -> String {
    let flat: String = s
        .chars()
        .filter(|c| !invisible(*c))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if flat.chars().count() <= max {
        return flat;
    }
    let mut out: String = flat.chars().take(max).collect();
    out.push('…');
    out
}

/// Render nodes as an indented outline with `[ref=eN]` handles. Unnamed layout
/// containers are elided (their children are lifted); secure-field values are
/// never emitted.
pub fn render<H: Clone>(roots: &[Node<H>], interactive_only: bool, max_nodes: u32) -> Rendered<H> {
    let mut out = Rendered {
        tree: String::new(),
        refs: Vec::new(),
        truncated: false,
    };
    for root in roots {
        render_node(root, 0, interactive_only, max_nodes as usize, &mut out);
    }
    out
}

fn render_node<H: Clone>(
    node: &Node<H>,
    depth: usize,
    interactive_only: bool,
    max_nodes: usize,
    out: &mut Rendered<H>,
) {
    if out.truncated {
        return;
    }
    let structural = matches!(
        node.role.as_str(),
        "window" | "application" | "sheet" | "dialog"
    );
    let emit = !node.is_noise() && (!interactive_only || node.is_interactive() || structural);
    let child_depth = if emit { depth + 1 } else { depth };
    if emit {
        if out.refs.len() >= max_nodes {
            out.truncated = true;
            return;
        }
        let id = format!("e{}", out.refs.len() + 1);
        let mut line = format!("{}- {}", "  ".repeat(depth), node.role);
        if !node.name.is_empty() {
            line.push_str(&format!(" {:?}", clip(&node.name, MAX_NAME_CHARS)));
        }
        line.push_str(&format!(" [ref={id}]"));
        if node.secure {
            line.push_str(" (secure)");
        } else if let Some(v) = node.value.as_deref().filter(|v| !v.is_empty()) {
            line.push_str(&format!(" value={:?}", clip(v, MAX_VALUE_CHARS)));
        }
        if !node.enabled {
            line.push_str(" (disabled)");
        }
        if node.focused {
            line.push_str(" (focused)");
        }
        out.tree.push_str(&line);
        out.tree.push('\n');
        out.refs.push((
            id,
            RefEntry {
                handle: node.handle.clone(),
                role: node.role.clone(),
                name: node.name.clone(),
                frame: node.frame,
                secure: node.secure,
                actions: node.actions.clone(),
            },
        ));
    }
    for child in &node.children {
        render_node(child, child_depth, interactive_only, max_nodes, out);
    }
}

struct StoredSnapshot<H> {
    id: String,
    pid: i64,
    refs: Vec<(String, RefEntry<H>)>,
}

/// Recent snapshots; refs are only valid within the snapshot that issued them.
pub struct SnapshotStore<H> {
    next: u64,
    snapshots: VecDeque<StoredSnapshot<H>>,
}

impl<H> Default for SnapshotStore<H> {
    fn default() -> Self {
        Self {
            next: 0,
            snapshots: VecDeque::new(),
        }
    }
}

impl<H: Clone> SnapshotStore<H> {
    pub fn insert(&mut self, pid: i64, refs: Vec<(String, RefEntry<H>)>) -> String {
        self.next += 1;
        let id = format!("s{}", self.next);
        if self.snapshots.len() == SNAPSHOT_HISTORY {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(StoredSnapshot {
            id: id.clone(),
            pid,
            refs,
        });
        id
    }

    /// Resolve `ref_id` in `snapshot_id`, or in the newest snapshot of `pid`.
    pub fn resolve(
        &self,
        pid: i64,
        snapshot_id: Option<&str>,
        ref_id: &str,
    ) -> Result<RefEntry<H>, String> {
        let snap = match snapshot_id {
            Some(id) => self
                .snapshots
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| format!("stale_snapshot: {id} expired; call snapshot again"))?,
            None => self
                .snapshots
                .iter()
                .rev()
                .find(|s| s.pid == pid)
                .ok_or_else(|| "no snapshot for this app; call snapshot first".to_string())?,
        };
        if snap.pid != pid {
            return Err(format!(
                "snapshot {} belongs to pid {}, not {pid}",
                snap.id, snap.pid
            ));
        }
        snap.refs
            .iter()
            .find(|(id, _)| id == ref_id)
            .map(|(_, entry)| entry.clone())
            .ok_or_else(|| format!("unknown ref {ref_id:?} in snapshot {}", snap.id))
    }
}

pub fn is_denied_app(name: &str, bundle_id: Option<&str>) -> bool {
    let name = name.to_ascii_lowercase();
    let bundle = bundle_id.map(str::to_ascii_lowercase);
    DENIED_APPS
        .iter()
        .any(|d| name == *d || bundle.as_deref() == Some(*d))
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppInfo {
    pub pid: i64,
    pub name: String,
    pub bundle_id: Option<String>,
    pub active: bool,
}

impl AppInfo {
    pub fn to_json(&self) -> Value {
        json!({
            "pid": self.pid,
            "name": self.name,
            "bundle_id": self.bundle_id,
            "active": self.active,
            "denied": is_denied_app(&self.name, self.bundle_id.as_deref()),
        })
    }
}

/// Resolve a target against running apps. Names match case-insensitively,
/// exact first, then by prefix when unambiguous.
pub fn resolve_target<'a>(apps: &'a [AppInfo], target: &Target) -> Result<&'a AppInfo, String> {
    if let Some(pid) = target.pid {
        return apps
            .iter()
            .find(|a| a.pid == pid)
            .ok_or_else(|| format!("no running app with pid {pid}"));
    }
    let want = target
        .app
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if want.is_empty() {
        return Err("target is required: {\"pid\":..} or {\"app\":..}".into());
    }
    let matches_exact = |a: &&AppInfo| {
        a.name.to_ascii_lowercase() == want
            || a.bundle_id
                .as_deref()
                .map(str::to_ascii_lowercase)
                .as_deref()
                == Some(want.as_str())
    };
    if let Some(app) = apps.iter().find(matches_exact) {
        return Ok(app);
    }
    let prefixed: Vec<&AppInfo> = apps
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().starts_with(&want))
        .collect();
    match prefixed.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("no running app named {want:?}; call list_apps")),
        many => Err(format!(
            "app {want:?} is ambiguous: {}",
            many.iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn refusal(reason: &str, detail: impl Into<String>) -> Value {
    json!({ "ok": false, "reason": reason, "detail": detail.into() })
}

/// Encode 8-bit BGRA/RGBA pixels (row stride `stride` bytes) as an RGB PNG.
pub fn encode_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    bgra: bool,
) -> Result<Vec<u8>, String> {
    let (w, h) = (width as usize, height as usize);
    if stride < w * 4 || pixels.len() < stride * h.saturating_sub(1) + w * 4 {
        return Err(format!(
            "pixel buffer too small for {width}x{height} stride {stride}"
        ));
    }
    let mut rgb = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        let line = &pixels[row * stride..row * stride + w * 4];
        for px in line.as_chunks::<4>().0 {
            if bgra {
                rgb.extend_from_slice(&[px[2], px[1], px[0]]);
            } else {
                rgb.extend_from_slice(&px[..3]);
            }
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgb).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: &str, name: &str, children: Vec<Node<u32>>) -> Node<u32> {
        Node {
            handle: 0,
            role: role.into(),
            name: name.into(),
            value: None,
            enabled: true,
            focused: false,
            secure: false,
            frame: Some(Rect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 40.0,
            }),
            actions: Vec::new(),
            children,
        }
    }

    fn action(json: &str) -> ActionRequest {
        serde_json::from_str(json).expect("valid action json")
    }

    #[test]
    fn chords_accept_common_spellings() {
        let c = parse_chord("cmd+shift+T").unwrap();
        assert!(c.mods.cmd && c.mods.shift && !c.mods.ctrl);
        assert_eq!(c.key, "t");
        assert_eq!(parse_chord("Return").unwrap().key, "enter");
        assert_eq!(parse_chord("ctrl+Page_Down").unwrap().key, "pagedown");
        assert_eq!(parse_chord("super+space").unwrap().key, "space");
        assert!(parse_chord("cmd+").is_err());
        assert!(parse_chord("ctrl+a+b").is_err());
        assert!(parse_chord("shift").is_err());
        assert!(parse_chord("hyper+x").is_err());
    }

    #[test]
    fn action_validation_rejects_incomplete_requests() {
        let ok = action(r#"{"target":{"app":"Notes"},"kind":"click","ref":"e1"}"#);
        assert!(ok.validate().is_ok());
        assert!(ok.respect_sensitive, "respect_sensitive defaults on");
        let cases = [
            r#"{"kind":"click","ref":"e1"}"#,
            r#"{"target":{"pid":1},"kind":"click"}"#,
            r#"{"target":{"pid":1},"kind":"set_value","ref":"e1"}"#,
            r#"{"target":{"pid":1},"kind":"type","text":""}"#,
            r#"{"target":{"pid":1},"kind":"key","key":"nope+x+y"}"#,
            r#"{"target":{"pid":1},"kind":"scroll","dy":0}"#,
            r#"{"target":{"pid":1},"kind":"drag","x":1,"y":1}"#,
            r#"{"target":{"pid":1},"kind":"click","ref":"e1","tier":"activate","require_background":true}"#,
        ];
        for case in cases {
            assert!(action(case).validate().is_err(), "should reject {case}");
        }
        assert!(
            serde_json::from_str::<ActionRequest>(
                r#"{"target":{"pid":1},"kind":"click","bogus":1}"#
            )
            .is_err()
        );
    }

    #[test]
    fn render_assigns_refs_elides_noise_and_hides_secure_values() {
        let mut secret = node("secure_textbox", "Password", vec![]);
        secret.secure = true;
        secret.value = Some("hunter2".into());
        let mut field = node("textbox", "Search", vec![]);
        field.value = Some("cats".into());
        field.focused = true;
        let mut disabled = node("button", "Save", vec![]);
        disabled.enabled = false;
        let tree = vec![node(
            "window",
            "Main",
            vec![node(
                "group",
                "",
                vec![field, secret, disabled, node("text", "Hello", vec![])],
            )],
        )];

        let r = render(&tree, false, 100);
        assert!(!r.truncated);
        assert_eq!(r.refs.len(), 5, "unnamed group elided:\n{}", r.tree);
        assert!(r.tree.contains("- window \"Main\" [ref=e1]"));
        assert!(
            r.tree
                .contains("  - textbox \"Search\" [ref=e2] value=\"cats\" (focused)")
        );
        assert!(r.tree.contains("(secure)"));
        assert!(!r.tree.contains("hunter2"));
        assert!(r.tree.contains("\"Save\" [ref=e4] (disabled)"));

        let marked = render(&[node("text", "\u{200e}7\nx\u{200f}", vec![])], false, 10);
        assert!(
            marked.tree.contains("- text \"7 x\" [ref=e1]"),
            "{}",
            marked.tree
        );

        let interactive = render(&tree, true, 100);
        assert!(!interactive.tree.contains("Hello"));
        assert!(
            interactive.tree.contains("Main"),
            "windows kept for structure"
        );

        let capped = render(&tree, false, 2);
        assert!(capped.truncated);
        assert_eq!(capped.refs.len(), 2);
    }

    #[test]
    fn snapshot_store_scopes_refs_and_expires_old_snapshots() {
        let mut store = SnapshotStore::<u32>::default();
        let r = render(&[node("button", "OK", vec![])], false, 10);
        let first = store.insert(7, r.refs);
        assert_eq!(store.resolve(7, Some(&first), "e1").unwrap().name, "OK");
        assert_eq!(store.resolve(7, None, "e1").unwrap().name, "OK");
        assert!(store.resolve(7, None, "e9").is_err());
        assert!(
            store
                .resolve(8, Some(&first), "e1")
                .unwrap_err()
                .contains("belongs to pid 7")
        );
        assert!(store.resolve(8, None, "e1").is_err());
        for _ in 0..SNAPSHOT_HISTORY {
            store.insert(7, Vec::new());
        }
        assert!(
            store
                .resolve(7, Some(&first), "e1")
                .unwrap_err()
                .starts_with("stale_snapshot")
        );
    }

    #[test]
    fn frames_map_screenshot_pixels_to_screen_points() {
        let (w, h, scale) = fit_size(1440.0, 900.0, 2.0, 1568);
        assert_eq!((w, h), (1568, 980));
        let frame = Frame {
            origin_x: 100.0,
            origin_y: 50.0,
            scale,
            width: w,
            height: h,
        };
        let (x, y) = frame.to_screen(784.0, 490.0).unwrap();
        assert!((x - 820.0).abs() < 1e-6 && (y - 500.0).abs() < 1e-6);
        assert!(frame.to_screen(5000.0, 1.0).is_err());
        assert_eq!(fit_size(400.0, 300.0, 2.0, 1568), (800, 600, 2.0));
    }

    #[test]
    fn targets_resolve_by_pid_name_bundle_and_unique_prefix() {
        let apps = vec![
            AppInfo {
                pid: 1,
                name: "Safari".into(),
                bundle_id: Some("com.apple.Safari".into()),
                active: true,
            },
            AppInfo {
                pid: 2,
                name: "Notes".into(),
                bundle_id: None,
                active: false,
            },
            AppInfo {
                pid: 3,
                name: "Notion".into(),
                bundle_id: None,
                active: false,
            },
        ];
        let t = |pid: Option<i64>, app: Option<&str>| Target {
            pid,
            app: app.map(String::from),
        };
        assert_eq!(
            resolve_target(&apps, &t(Some(2), None)).unwrap().name,
            "Notes"
        );
        assert_eq!(
            resolve_target(&apps, &t(None, Some("safari"))).unwrap().pid,
            1
        );
        assert_eq!(
            resolve_target(&apps, &t(None, Some("com.apple.safari")))
                .unwrap()
                .pid,
            1
        );
        assert_eq!(
            resolve_target(&apps, &t(None, Some("Noti"))).unwrap().pid,
            3
        );
        assert!(
            resolve_target(&apps, &t(None, Some("No")))
                .unwrap_err()
                .contains("ambiguous")
        );
        assert!(resolve_target(&apps, &t(Some(99), None)).is_err());
    }

    #[test]
    fn password_managers_are_denied() {
        assert!(is_denied_app("1Password", None));
        assert!(is_denied_app("Whatever", Some("com.apple.keychainaccess")));
        assert!(!is_denied_app("Safari", Some("com.apple.Safari")));
    }

    #[test]
    fn png_encoding_swaps_bgra_and_honours_stride() {
        // 2x1 image, stride 12 (4 bytes padding): blue then red in BGRA.
        let px = [255, 0, 0, 255, 0, 0, 255, 255, 9, 9, 9, 9];
        let png_bytes = encode_png(&px, 2, 1, 12, true).unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(&buf[..info.buffer_size()], &[0, 0, 255, 255, 0, 0]);
        assert!(encode_png(&px, 4, 1, 12, true).is_err());
        assert!(encode_png(&px, 2, 2, 12, true).is_err());
    }
}
