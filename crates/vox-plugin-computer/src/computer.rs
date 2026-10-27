//! OS-independent dispatcher: target resolution, snapshot refs, coordinate
//! frames, tier selection, and policy refusals. Backends implement [`Backend`].

use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;
use serde_json::{Value, json};

use crate::model::{
    ActionKind, ActionRequest, AppInfo, Chord, Coords, DEFAULT_MAX_DEPTH, DEFAULT_MAX_LONG_EDGE,
    DEFAULT_MAX_NODES, Frame, Node, Rect, RefEntry, ScreenshotOptions, SnapshotOptions,
    SnapshotStore, Tier, WindowFilter, is_denied_app, parse_chord, parse_options, refusal, render,
    resolve_target,
};

/// Frame key for whole-display screenshots (no target app).
const DISPLAY_KEY: i64 = -1;

/// Error codes a backend may prefix onto an error (`"code: detail"`); they
/// surface as the refusal `reason`.
const BACKEND_REASONS: &[&str] = &[
    "accessibility_denied",
    "screen_recording_denied",
    "unsupported_platform",
    "stale_ref",
    "not_supported",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pointer {
    Click { button: Button, count: u32 },
    Move,
    Drag { to: (f64, f64) },
}

pub struct Capture {
    pub png: Vec<u8>,
    pub frame: Frame,
}

/// Where posted input goes: to one process without focusing it, or to the
/// global event stream (only after the target was activated).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Process(i64),
    Global,
}

pub trait Backend: Send + Sync + 'static {
    type Handle: Clone + Send + Sync + 'static;

    fn platform(&self) -> &'static str;
    fn permissions(&self) -> Value;
    fn list_apps(&self) -> Result<Vec<AppInfo>, String>;
    fn list_windows(&self, pid: Option<i64>) -> Result<Vec<Value>, String>;
    fn ax_tree(
        &self,
        pid: i64,
        max_depth: u32,
        max_nodes: u32,
    ) -> Result<Vec<Node<Self::Handle>>, String>;
    fn ax_perform(&self, handle: &Self::Handle, action: &str) -> Result<(), String>;
    fn ax_set_value(&self, handle: &Self::Handle, value: &str) -> Result<(), String>;
    fn ax_focus(&self, handle: &Self::Handle) -> Result<(), String>;
    /// Capture the target's front window (`pid`) or the main display, limited
    /// to `region` (global screen points) when given.
    fn screenshot(
        &self,
        pid: Option<i64>,
        region: Option<Rect>,
        max_long_edge: u32,
    ) -> Result<Capture, String>;
    fn post_pointer(&self, route: Route, at: (f64, f64), pointer: Pointer) -> Result<(), String>;
    fn post_scroll(
        &self,
        route: Route,
        at: Option<(f64, f64)>,
        dx: i64,
        dy: i64,
    ) -> Result<(), String>;
    fn post_text(&self, route: Route, text: &str) -> Result<(), String>;
    fn post_key(&self, route: Route, chord: &Chord) -> Result<(), String>;
    /// Bring `pid` to the front; returns the previously frontmost pid.
    fn activate(&self, pid: i64) -> Result<Option<i64>, String>;
    fn restore(&self, previous: Option<i64>);
    /// Platform select-all chord (`cmd+a` on macOS, `ctrl+a` elsewhere).
    fn select_all(&self) -> Chord;
}

struct State<H> {
    store: SnapshotStore<H>,
    frames: HashMap<i64, Frame>,
}

pub struct Computer<B: Backend> {
    backend: B,
    state: Mutex<State<B::Handle>>,
}

fn backend_refusal(err: String) -> Value {
    let reason = err
        .split_once(':')
        .map(|(code, _)| code.trim())
        .filter(|code| BACKEND_REASONS.contains(code))
        .unwrap_or("backend_error")
        .to_string();
    refusal(&reason, err)
}

fn denied(app: &AppInfo) -> Option<Value> {
    is_denied_app(&app.name, app.bundle_id.as_deref()).then(|| {
        refusal(
            "denied_app",
            format!(
                "{} is on the computer-use denylist (credential manager)",
                app.name
            ),
        )
    })
}

impl<B: Backend> Computer<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: Mutex::new(State {
                store: SnapshotStore::default(),
                frames: HashMap::new(),
            }),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State<B::Handle>> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    pub fn permissions(&self) -> Value {
        let mut report = self.backend.permissions();
        report["platform"] = json!(self.backend.platform());
        report
    }

    pub fn list_apps(&self) -> Value {
        match self.backend.list_apps() {
            Ok(apps) => {
                json!({ "ok": true, "apps": apps.iter().map(AppInfo::to_json).collect::<Vec<_>>() })
            }
            Err(e) => backend_refusal(e),
        }
    }

    pub fn list_windows(&self, options_json: &str) -> Value {
        let filter: WindowFilter = match parse_options(options_json) {
            Ok(f) => f,
            Err(e) => return refusal("invalid_request", e),
        };
        let pid = match (filter.pid, filter.app) {
            (Some(pid), _) => Some(pid),
            (None, Some(app)) => match self.app_for(&crate::model::Target {
                pid: None,
                app: Some(app),
            }) {
                Ok(app) => Some(app.pid),
                Err(v) => return v,
            },
            (None, None) => None,
        };
        match self.backend.list_windows(pid) {
            Ok(windows) => json!({ "ok": true, "windows": windows }),
            Err(e) => backend_refusal(e),
        }
    }

    fn app_for(&self, target: &crate::model::Target) -> Result<AppInfo, Value> {
        let apps = self.backend.list_apps().map_err(backend_refusal)?;
        let app = match resolve_target(&apps, target) {
            Ok(app) => app.clone(),
            // Helper processes (Electron/Steam helpers, agents) own windows
            // without being regular apps; a pid from list_windows must work.
            Err(e) => target
                .pid
                .and_then(|pid| self.window_owner(pid))
                .ok_or_else(|| refusal("target_not_found", e))?,
        };
        match denied(&app) {
            Some(v) => Err(v),
            None => Ok(app),
        }
    }

    fn window_owner(&self, pid: i64) -> Option<AppInfo> {
        let windows = self.backend.list_windows(Some(pid)).ok()?;
        let name = windows.first()?["app"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        Some(AppInfo {
            pid,
            name,
            bundle_id: None,
            active: false,
        })
    }

    pub fn snapshot(&self, options_json: &str) -> Value {
        let opts: SnapshotOptions = match parse_options(options_json) {
            Ok(o) => o,
            Err(e) => return refusal("invalid_request", e),
        };
        let app = match self.app_for(&opts.target) {
            Ok(app) => app,
            Err(v) => return v,
        };
        let max_nodes = opts.max_nodes.unwrap_or(DEFAULT_MAX_NODES).clamp(1, 5000);
        let max_depth = opts.max_depth.unwrap_or(DEFAULT_MAX_DEPTH).clamp(1, 100);
        let roots = match self
            .backend
            .ax_tree(app.pid, max_depth, max_nodes.saturating_mul(4))
        {
            Ok(r) => r,
            Err(e) => return backend_refusal(e),
        };
        let rendered = render(&roots, opts.interactive_only, max_nodes);
        let ref_count = rendered.refs.len();
        let snapshot_id = self.state().store.insert(app.pid, rendered.refs);
        json!({
            "ok": true,
            "snapshot_id": snapshot_id,
            "pid": app.pid,
            "app": app.name,
            "tree": rendered.tree,
            "ref_count": ref_count,
            "truncated": rendered.truncated,
        })
    }

    pub fn screenshot(&self, options_json: &str) -> Value {
        let opts: ScreenshotOptions = match parse_options(options_json) {
            Ok(o) => o,
            Err(e) => return refusal("invalid_request", e),
        };
        let pid = match opts.target.as_ref().filter(|t| !t.is_empty()) {
            Some(target) => match self.app_for(target) {
                Ok(app) => Some(app.pid),
                Err(v) => return v,
            },
            None => None,
        };
        let key = pid.unwrap_or(DISPLAY_KEY);
        let region = match opts.region {
            None => None,
            Some([x0, y0, x1, y1]) => {
                let Some(frame) = self.state().frames.get(&key).copied() else {
                    return refusal("no_screenshot", "take a full screenshot before zooming");
                };
                let (a, b) = match (
                    frame.to_screen(x0.min(x1), y0.min(y1)),
                    frame.to_screen(x0.max(x1), y0.max(y1)),
                ) {
                    (Ok(a), Ok(b)) => (a, b),
                    (Err(e), _) | (_, Err(e)) => return refusal("invalid_request", e),
                };
                let rect = Rect {
                    x: a.0,
                    y: a.1,
                    w: b.0 - a.0,
                    h: b.1 - a.1,
                };
                if rect.is_empty() {
                    return refusal("invalid_request", "zoom region is empty");
                }
                Some(rect)
            }
        };
        let max_long_edge = opts
            .max_long_edge
            .unwrap_or(DEFAULT_MAX_LONG_EDGE)
            .clamp(64, 4096);
        let capture = match self.backend.screenshot(pid, region, max_long_edge) {
            Ok(c) => c,
            Err(e) => return backend_refusal(e),
        };
        if region.is_none() {
            self.state().frames.insert(key, capture.frame);
        }
        let f = capture.frame;
        json!({
            "ok": true,
            "mime": "image/png",
            "image_base64": base64::engine::general_purpose::STANDARD.encode(&capture.png),
            "width": f.width,
            "height": f.height,
            "scale": f.scale,
            "origin": [f.origin_x, f.origin_y],
            "zoomed": region.is_some(),
        })
    }

    pub fn act(&self, action_json: &str) -> Value {
        let req: ActionRequest = match parse_options(action_json) {
            Ok(r) => r,
            Err(e) => return refusal("invalid_request", e),
        };
        if let Err(e) = req.validate() {
            return refusal("invalid_request", e);
        }
        let app = match self.app_for(&req.target) {
            Ok(app) => app,
            Err(v) => return v,
        };
        let entry = match req.ref_id.as_deref() {
            Some(r) => match self
                .state()
                .store
                .resolve(app.pid, req.snapshot_id.as_deref(), r)
            {
                Ok(e) => Some(e),
                Err(e) => return refusal("stale_ref", e),
            },
            None => None,
        };
        if let Some(e) = &entry {
            let writes = matches!(req.kind, ActionKind::SetValue | ActionKind::Type);
            if e.secure && writes && req.respect_sensitive {
                return refusal(
                    "sensitive_field",
                    format!(
                        "{} {:?} is a secure field; ask the user to enter it",
                        e.role, e.name
                    ),
                );
            }
        }
        match self.perform(&app, &req, entry.as_ref()) {
            Ok(mut done) => {
                done["ok"] = json!(true);
                done["kind"] = json!(req.kind.as_str());
                done
            }
            Err(v) => v,
        }
    }

    fn point(
        &self,
        pid: i64,
        req: &ActionRequest,
        entry: Option<&RefEntry<B::Handle>>,
        to: bool,
    ) -> Result<(f64, f64), Value> {
        if !to && let Some(e) = entry {
            return e
                .frame
                .filter(|f| !f.is_empty())
                .map(|f| f.center())
                .ok_or_else(|| {
                    refusal(
                        "not_supported",
                        format!("ref {:?} has no on-screen frame", e.name),
                    )
                });
        }
        let (x, y) = if to {
            (req.to_x, req.to_y)
        } else {
            (req.x, req.y)
        };
        let (x, y) = (x.unwrap_or_default(), y.unwrap_or_default());
        match req.coords {
            Coords::Screen => Ok((x, y)),
            Coords::Screenshot => {
                let state = self.state();
                let frame = state
                    .frames
                    .get(&pid)
                    .or_else(|| state.frames.get(&DISPLAY_KEY));
                let frame = frame.ok_or_else(|| {
                    refusal(
                        "no_screenshot",
                        "take a screenshot first, or pass \"coords\":\"screen\"",
                    )
                })?;
                frame
                    .to_screen(x, y)
                    .map_err(|e| refusal("invalid_request", e))
            }
        }
    }

    fn perform(
        &self,
        app: &AppInfo,
        req: &ActionRequest,
        entry: Option<&RefEntry<B::Handle>>,
    ) -> Result<Value, Value> {
        let b = &self.backend;
        let ax_ok = matches!(req.tier, Tier::Auto | Tier::Ax);

        // Accessibility tier: no cursor motion, no focus change.
        if ax_ok {
            let ax = match (req.kind, entry) {
                (ActionKind::Click | ActionKind::Press, Some(e)) if e.supports("press") => {
                    Some(b.ax_perform(&e.handle, "press"))
                }
                (ActionKind::RightClick, Some(e)) if e.supports("show_menu") => {
                    Some(b.ax_perform(&e.handle, "show_menu"))
                }
                (ActionKind::SetValue, Some(e)) => {
                    Some(b.ax_set_value(&e.handle, req.text.as_deref().unwrap_or_default()))
                }
                (ActionKind::Focus, Some(e)) => Some(b.ax_focus(&e.handle)),
                _ => None,
            };
            match ax {
                Some(Ok(())) => return Ok(json!({ "tier": Tier::Ax.as_str() })),
                Some(Err(e)) if req.tier == Tier::Ax => return Err(backend_refusal(e)),
                None if req.tier == Tier::Ax => {
                    return Err(refusal(
                        "tier_unavailable",
                        format!(
                            "{} has no accessibility action here; use tier \"event\"",
                            req.kind.as_str()
                        ),
                    ));
                }
                _ => {}
            }
        }
        if req.kind == ActionKind::Focus {
            return Err(refusal("tier_unavailable", "focus is accessibility-only"));
        }

        let activated = req.tier == Tier::Activate;
        if activated && req.require_background {
            return Err(refusal(
                "needs_foreground",
                "action requires activating the app",
            ));
        }
        let route = if activated {
            Route::Global
        } else {
            Route::Process(app.pid)
        };
        let previous = if activated {
            b.activate(app.pid).map_err(backend_refusal)?
        } else {
            None
        };
        let result = self.post(app.pid, req, entry, route);
        if activated {
            b.restore(previous);
        }
        let mut done = result?;
        done["tier"] = json!(if activated {
            Tier::Activate.as_str()
        } else {
            Tier::Event.as_str()
        });
        if !activated {
            done["note"] = json!(
                "posted to the app without focusing it; some apps ignore background input — \
                 verify with a fresh snapshot or screenshot, then retry with tier \"activate\""
            );
        }
        Ok(done)
    }

    fn post(
        &self,
        pid: i64,
        req: &ActionRequest,
        entry: Option<&RefEntry<B::Handle>>,
        route: Route,
    ) -> Result<Value, Value> {
        let b = &self.backend;
        let pointer = |p: Pointer| -> Result<Value, Value> {
            let at = self.point(pid, req, entry, false)?;
            b.post_pointer(route, at, p).map_err(backend_refusal)?;
            Ok(json!({ "point": [at.0, at.1] }))
        };
        match req.kind {
            ActionKind::Click | ActionKind::Press => pointer(Pointer::Click {
                button: Button::Left,
                count: 1,
            }),
            ActionKind::DoubleClick => pointer(Pointer::Click {
                button: Button::Left,
                count: 2,
            }),
            ActionKind::RightClick => pointer(Pointer::Click {
                button: Button::Right,
                count: 1,
            }),
            ActionKind::Move => pointer(Pointer::Move),
            ActionKind::Drag => {
                let to = self.point(pid, req, None, true)?;
                pointer(Pointer::Drag { to })
            }
            ActionKind::Scroll => {
                let at = if entry.is_some() || req.has_point() {
                    Some(self.point(pid, req, entry, false)?)
                } else {
                    None
                };
                b.post_scroll(route, at, req.dx.unwrap_or(0), req.dy.unwrap_or(0))
                    .map_err(backend_refusal)?;
                Ok(json!({}))
            }
            ActionKind::Type | ActionKind::SetValue => {
                if let Some(e) = entry {
                    b.ax_focus(&e.handle).map_err(backend_refusal)?;
                }
                if req.kind == ActionKind::SetValue {
                    b.post_key(route, &b.select_all())
                        .map_err(backend_refusal)?;
                }
                b.post_text(route, req.text.as_deref().unwrap_or_default())
                    .map_err(backend_refusal)?;
                Ok(json!({}))
            }
            ActionKind::Key => {
                let chord = parse_chord(req.key.as_deref().unwrap_or_default())
                    .map_err(|e| refusal("invalid_request", e))?;
                b.post_key(route, &chord).map_err(backend_refusal)?;
                Ok(json!({}))
            }
            ActionKind::Focus => Err(refusal("tier_unavailable", "focus is accessibility-only")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[derive(Default)]
    pub struct Fake {
        pub calls: Mutex<Vec<String>>,
        pub ax_fails: bool,
    }

    impl Fake {
        fn log(&self, s: String) {
            self.calls.lock().unwrap().push(s);
        }
        fn take(&self) -> Vec<String> {
            std::mem::take(&mut *self.calls.lock().unwrap())
        }
    }

    fn leaf(handle: u32, role: &str, name: &str, actions: &[&str]) -> Node<u32> {
        Node {
            handle,
            role: role.into(),
            name: name.into(),
            value: None,
            enabled: true,
            focused: false,
            secure: role == "secure_textbox",
            frame: Some(Rect {
                x: 100.0,
                y: 200.0,
                w: 50.0,
                h: 20.0,
            }),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            children: vec![],
        }
    }

    impl Backend for Fake {
        type Handle = u32;
        fn platform(&self) -> &'static str {
            "fake"
        }
        fn permissions(&self) -> Value {
            json!({ "accessibility": "granted" })
        }
        fn list_apps(&self) -> Result<Vec<AppInfo>, String> {
            Ok(vec![
                AppInfo {
                    pid: 10,
                    name: "Notes".into(),
                    bundle_id: Some("com.apple.Notes".into()),
                    active: false,
                },
                AppInfo {
                    pid: 11,
                    name: "1Password".into(),
                    bundle_id: None,
                    active: false,
                },
            ])
        }
        fn list_windows(&self, pid: Option<i64>) -> Result<Vec<Value>, String> {
            Ok(match pid {
                Some(12) => vec![json!({ "pid": 12, "app": "Steam Helper" })],
                Some(_) => vec![],
                None => vec![json!({ "pid": 10, "app": "Notes" })],
            })
        }
        fn ax_tree(&self, _pid: i64, _d: u32, _n: u32) -> Result<Vec<Node<u32>>, String> {
            let mut window = leaf(0, "window", "Note", &[]);
            window.children = vec![
                leaf(1, "button", "New", &["press"]),
                leaf(2, "textbox", "Body", &[]),
                leaf(3, "secure_textbox", "Password", &[]),
                leaf(4, "image", "Canvas", &[]),
            ];
            Ok(vec![window])
        }
        fn ax_perform(&self, h: &u32, action: &str) -> Result<(), String> {
            self.log(format!("ax_perform {h} {action}"));
            if self.ax_fails {
                Err("stale_ref: element gone".into())
            } else {
                Ok(())
            }
        }
        fn ax_set_value(&self, h: &u32, v: &str) -> Result<(), String> {
            self.log(format!("ax_set_value {h} {v}"));
            if self.ax_fails {
                Err("not_supported: read-only".into())
            } else {
                Ok(())
            }
        }
        fn ax_focus(&self, h: &u32) -> Result<(), String> {
            self.log(format!("ax_focus {h}"));
            Ok(())
        }
        fn screenshot(
            &self,
            pid: Option<i64>,
            region: Option<Rect>,
            max: u32,
        ) -> Result<Capture, String> {
            self.log(format!("screenshot {pid:?} {region:?} {max}"));
            Ok(Capture {
                png: vec![1, 2, 3],
                frame: Frame {
                    origin_x: 0.0,
                    origin_y: 25.0,
                    scale: 2.0,
                    width: 1000,
                    height: 600,
                },
            })
        }
        fn post_pointer(&self, route: Route, at: (f64, f64), p: Pointer) -> Result<(), String> {
            self.log(format!("pointer {route:?} {at:?} {p:?}"));
            Ok(())
        }
        fn post_scroll(
            &self,
            route: Route,
            at: Option<(f64, f64)>,
            dx: i64,
            dy: i64,
        ) -> Result<(), String> {
            self.log(format!("scroll {route:?} {at:?} {dx} {dy}"));
            Ok(())
        }
        fn post_text(&self, route: Route, text: &str) -> Result<(), String> {
            self.log(format!("text {route:?} {text}"));
            Ok(())
        }
        fn post_key(&self, route: Route, chord: &Chord) -> Result<(), String> {
            self.log(format!("key {route:?} {}", chord.key));
            Ok(())
        }
        fn activate(&self, pid: i64) -> Result<Option<i64>, String> {
            self.log(format!("activate {pid}"));
            Ok(Some(99))
        }
        fn restore(&self, previous: Option<i64>) {
            self.log(format!("restore {previous:?}"));
        }
        fn select_all(&self) -> Chord {
            parse_chord("cmd+a").unwrap()
        }
    }

    fn computer() -> Computer<Fake> {
        Computer::new(Fake::default())
    }

    fn snap(c: &Computer<Fake>) -> String {
        let s = c.snapshot(r#"{"target":{"app":"Notes"}}"#);
        assert_eq!(s["ok"], true, "{s}");
        s["snapshot_id"].as_str().unwrap().to_string()
    }

    #[test]
    fn click_prefers_accessibility_press_then_falls_back_to_events() {
        let c = computer();
        let id = snap(&c);
        // e1 window, e2 New(press), e3 Body, e4 Password, e5 Canvas
        let r = c.act(&format!(
            r#"{{"target":{{"app":"Notes"}},"kind":"click","snapshot_id":"{id}","ref":"e2"}}"#
        ));
        assert_eq!(
            (r["ok"].as_bool(), r["tier"].as_str()),
            (Some(true), Some("ax")),
            "{r}"
        );
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","ref":"e5"}"#);
        assert_eq!(r["tier"], "event", "{r}");
        assert_eq!(r["point"], json!([125.0, 210.0]));
        let calls = c.backend.take();
        assert!(calls.iter().any(|s| s == "ax_perform 1 press"));
        assert!(
            calls
                .iter()
                .any(|s| s.starts_with("pointer Process(10) (125.0, 210.0)")),
            "{calls:?}"
        );
        assert!(
            !calls.iter().any(|s| s.starts_with("activate")),
            "auto never activates"
        );
    }

    #[test]
    fn explicit_ax_tier_refuses_instead_of_escalating() {
        let c = computer();
        snap(&c);
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","ref":"e5","tier":"ax"}"#);
        assert_eq!(r["reason"], "tier_unavailable", "{r}");
        assert!(c.backend.take().iter().all(|s| !s.starts_with("pointer")));
    }

    #[test]
    fn set_value_falls_back_to_select_all_and_typing() {
        let c = Computer::new(Fake {
            ax_fails: true,
            ..Fake::default()
        });
        snap(&c);
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"set_value","ref":"e3","text":"hi"}"#);
        assert_eq!(r["tier"], "event", "{r}");
        let calls = c.backend.take();
        let tail: Vec<&str> = calls.iter().map(String::as_str).skip(1).collect();
        assert_eq!(
            tail,
            ["ax_focus 2", "key Process(10) a", "text Process(10) hi"]
        );
    }

    #[test]
    fn secure_fields_and_denied_apps_are_refused() {
        let c = computer();
        snap(&c);
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"type","ref":"e4","text":"hunter2"}"#);
        assert_eq!(r["reason"], "sensitive_field", "{r}");
        assert!(c.backend.take().iter().all(|s| !s.contains("hunter2")));
        for call in [
            c.snapshot(r#"{"target":{"pid":11}}"#),
            c.screenshot(r#"{"target":{"app":"1Password"}}"#),
            c.act(r#"{"target":{"pid":11},"kind":"key","key":"cmd+c"}"#),
        ] {
            assert_eq!(call["reason"], "denied_app", "{call}");
        }
    }

    #[test]
    fn stale_refs_and_unknown_targets_are_reported() {
        let c = computer();
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","ref":"e2"}"#);
        assert_eq!(r["reason"], "stale_ref", "no snapshot yet: {r}");
        let r = c.act(r#"{"target":{"app":"Nope"},"kind":"key","key":"enter"}"#);
        assert_eq!(r["reason"], "target_not_found");
        let r =
            c.act(r#"{"target":{"app":"Notes"},"kind":"click","snapshot_id":"s99","ref":"e2"}"#);
        assert!(r["detail"].as_str().unwrap().starts_with("stale_snapshot"));
    }

    #[test]
    fn coordinates_map_through_the_last_screenshot_and_zoom_keeps_the_frame() {
        let c = computer();
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","x":10,"y":10}"#);
        assert_eq!(r["reason"], "no_screenshot");
        let shot = c.screenshot(r#"{"target":{"app":"Notes"}}"#);
        assert_eq!(
            (shot["width"].as_u64(), shot["zoomed"].as_bool()),
            (Some(1000), Some(false))
        );
        let zoom = c.screenshot(r#"{"target":{"app":"Notes"},"region":[100,100,300,200]}"#);
        assert_eq!(zoom["zoomed"], true);
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","x":200,"y":50}"#);
        assert_eq!(r["point"], json!([100.0, 50.0]), "{r}");
        let calls = c.backend.take();
        assert!(
            calls
                .iter()
                .any(|s| s.contains("Some(Rect { x: 50.0, y: 75.0, w: 100.0, h: 50.0 })")),
            "{calls:?}"
        );
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"click","x":5,"y":6,"coords":"screen"}"#);
        assert_eq!(r["point"], json!([5.0, 6.0]));
    }

    #[test]
    fn window_owning_helper_processes_are_targetable_by_pid() {
        let c = computer();
        let shot = c.screenshot(r#"{"target":{"pid":12}}"#);
        assert_eq!(shot["ok"], true, "{shot}");
        let none = c.screenshot(r#"{"target":{"pid":13}}"#);
        assert_eq!(none["reason"], "target_not_found", "{none}");
        let by_name = c.screenshot(r#"{"target":{"app":"Steam Helper"}}"#);
        assert_eq!(
            by_name["reason"], "target_not_found",
            "names only match regular apps"
        );
    }
    #[test]
    fn activate_tier_routes_globally_and_restores_the_previous_app() {
        let c = computer();
        let r = c.act(r#"{"target":{"app":"Notes"},"kind":"key","key":"cmd+s","tier":"activate"}"#);
        assert_eq!(r["tier"], "activate", "{r}");
        assert_eq!(
            c.backend.take(),
            ["activate 10", "key Global s", "restore Some(99)"]
        );
    }
}
