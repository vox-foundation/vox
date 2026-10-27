//! macOS backend: AXUIElement trees, `CGEventPostToPid` input, and
//! ScreenCaptureKit window capture. Public frameworks only.
#![allow(unsafe_code)] // FFI into AppKit, ApplicationServices, CoreGraphics and ScreenCaptureKit.

mod ax;
mod capture;
mod input;
#[cfg(test)]
mod live_tests;

use std::thread::sleep;
use std::time::{Duration, Instant};

use objc2_app_kit::{
    NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication,
    NSWorkspace,
};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_graphics::{
    CGPreflightScreenCaptureAccess, CGWindowListCopyWindowInfo, CGWindowListOption,
    kCGWindowBounds, kCGWindowLayer, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID,
};
use serde_json::{Value, json};

use crate::computer::{Backend, Capture, Pointer, Route};
use crate::model::{AppInfo, Chord, Node, Rect, is_denied_app, parse_chord};

pub use ax::AxHandle;

#[derive(Default)]
pub struct Mac;

struct WindowRow {
    id: u32,
    pid: i64,
    owner: String,
    title: Option<String>,
    bounds: Rect,
}

fn number(dict: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<f64> {
    dict.get(key)?.downcast_ref::<CFNumber>()?.as_f64()
}

fn string(dict: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<String> {
    Some(dict.get(key)?.downcast_ref::<CFString>()?.to_string())
}

/// On-screen, normal-layer windows, front to back.
fn windows() -> Vec<WindowRow> {
    let opts = CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements;
    let Some(list) = CGWindowListCopyWindowInfo(opts, 0) else {
        return Vec::new();
    };
    let list = ax::typed(&list);
    let mut rows = Vec::new();
    for item in list.iter() {
        let Ok(dict) = item.downcast::<CFDictionary>() else {
            continue;
        };
        // SAFETY: window-info dictionaries map CFString keys to CF values.
        let dict: CFRetained<CFDictionary<CFString, CFType>> =
            unsafe { CFRetained::cast_unchecked(dict) };
        let (k_layer, k_id, k_pid, k_owner, k_name, k_bounds) = unsafe {
            (
                kCGWindowLayer,
                kCGWindowNumber,
                kCGWindowOwnerPID,
                kCGWindowOwnerName,
                kCGWindowName,
                kCGWindowBounds,
            )
        };
        if number(&dict, k_layer).unwrap_or(1.0) != 0.0 {
            continue;
        }
        let bounds = dict
            .get(k_bounds)
            .and_then(|b| b.downcast::<CFDictionary>().ok())
            .map(|b| unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(b) });
        let field = |name: &str| {
            bounds
                .as_deref()
                .and_then(|b| number(b, &CFString::from_str(name)))
        };
        let bounds = Rect {
            x: field("X").unwrap_or_default(),
            y: field("Y").unwrap_or_default(),
            w: field("Width").unwrap_or_default(),
            h: field("Height").unwrap_or_default(),
        };
        if bounds.w < 2.0 || bounds.h < 2.0 {
            continue;
        }
        rows.push(WindowRow {
            id: number(&dict, k_id).unwrap_or_default() as u32,
            pid: number(&dict, k_pid).unwrap_or_default() as i64,
            owner: string(&dict, k_owner).unwrap_or_default(),
            title: string(&dict, k_name).filter(|t| !t.is_empty()),
            bounds,
        });
    }
    rows
}

fn running(pid: i64) -> Option<objc2::rc::Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid as i32)
}

fn frontmost_pid() -> Option<i64> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|a| i64::from(a.processIdentifier()))
}

impl Backend for Mac {
    type Handle = AxHandle;

    fn platform(&self) -> &'static str {
        "macos"
    }

    fn permissions(&self) -> Value {
        let grant = |ok: bool| if ok { "granted" } else { "denied" };
        json!({
            "accessibility": grant(ax::trusted()),
            "screen_recording": grant(CGPreflightScreenCaptureAccess()),
            "background_input": true,
            "notes": [
                "Permissions belong to the process hosting the plugin (terminal, IDE, or vox daemon).",
                "Event-tier input reaches most Cocoa apps without focus; some apps ignore background clicks — use tier \"activate\" for those.",
            ],
        })
    }

    fn list_apps(&self) -> Result<Vec<AppInfo>, String> {
        let apps = NSWorkspace::sharedWorkspace().runningApplications();
        Ok(apps
            .iter()
            .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .map(|a| AppInfo {
                pid: i64::from(a.processIdentifier()),
                name: a.localizedName().map(|s| s.to_string()).unwrap_or_default(),
                bundle_id: a.bundleIdentifier().map(|s| s.to_string()),
                active: a.isActive(),
            })
            .collect())
    }

    fn list_windows(&self, pid: Option<i64>) -> Result<Vec<Value>, String> {
        Ok(windows()
            .into_iter()
            .filter(|w| pid.is_none_or(|p| p == w.pid))
            .map(|w| {
                let denied = is_denied_app(&w.owner, None);
                json!({
                    "window_id": w.id,
                    "pid": w.pid,
                    "app": w.owner,
                    "title": if denied { None } else { w.title },
                    "bounds": [w.bounds.x, w.bounds.y, w.bounds.w, w.bounds.h],
                })
            })
            .collect())
    }

    fn ax_tree(
        &self,
        pid: i64,
        max_depth: u32,
        max_nodes: u32,
    ) -> Result<Vec<Node<AxHandle>>, String> {
        ax::tree(pid, max_depth, max_nodes)
    }

    fn ax_perform(&self, handle: &AxHandle, action: &str) -> Result<(), String> {
        ax::perform(handle, action)
    }

    fn ax_set_value(&self, handle: &AxHandle, value: &str) -> Result<(), String> {
        ax::set_value(handle, value)
    }

    fn ax_focus(&self, handle: &AxHandle) -> Result<(), String> {
        ax::focus(handle)
    }

    fn screenshot(
        &self,
        pid: Option<i64>,
        region: Option<Rect>,
        max_long_edge: u32,
    ) -> Result<Capture, String> {
        let window = match pid {
            Some(pid) => Some(
                windows()
                    .into_iter()
                    .filter(|w| w.pid == pid)
                    .max_by(|a, b| (a.bounds.w * a.bounds.h).total_cmp(&(b.bounds.w * b.bounds.h)))
                    .map(|w| w.id)
                    .ok_or("backend_error: app has no on-screen window (minimized or hidden?)")?,
            ),
            None => None,
        };
        capture::screenshot(window, region, max_long_edge)
    }

    fn post_pointer(&self, route: Route, at: (f64, f64), pointer: Pointer) -> Result<(), String> {
        ax::require_trust()?;
        input::pointer(route, at, pointer)
    }

    fn post_scroll(
        &self,
        route: Route,
        at: Option<(f64, f64)>,
        dx: i64,
        dy: i64,
    ) -> Result<(), String> {
        ax::require_trust()?;
        input::scroll(route, at, dx, dy)
    }

    fn post_text(&self, route: Route, text: &str) -> Result<(), String> {
        ax::require_trust()?;
        input::text(route, text)
    }

    fn post_key(&self, route: Route, chord: &Chord) -> Result<(), String> {
        ax::require_trust()?;
        input::key(route, chord)
    }

    fn activate(&self, pid: i64) -> Result<Option<i64>, String> {
        let previous = frontmost_pid();
        let app = running(pid).ok_or_else(|| format!("no running app with pid {pid}"))?;
        #[allow(deprecated)]
        let _ = app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
        let deadline = Instant::now() + Duration::from_secs(1);
        while frontmost_pid() != Some(pid) {
            if Instant::now() > deadline {
                return Err("backend_error: app did not come to the front".into());
            }
            sleep(Duration::from_millis(25));
        }
        sleep(Duration::from_millis(80));
        Ok(previous.filter(|p| *p != pid))
    }

    fn restore(&self, previous: Option<i64>) {
        if let Some(app) = previous.and_then(running) {
            #[allow(deprecated)]
            let _ =
                app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
        }
    }

    fn select_all(&self) -> Chord {
        parse_chord("cmd+a").expect("static chord")
    }
}
