//! Platforms without a backend yet (Windows: UI Automation + SendInput;
//! Linux: AT-SPI + XTest/libei). Every primitive refuses with
//! `unsupported_platform` so callers get a clear answer instead of a silent
//! no-op.

use serde_json::{Value, json};

use crate::computer::{Backend, Capture, Pointer, Route};
use crate::model::{AppInfo, Chord, Node, Rect, parse_chord};

#[derive(Default)]
pub struct Unsupported;

fn no<T>() -> Result<T, String> {
    Err(format!(
        "unsupported_platform: computer use is not implemented on {} yet",
        std::env::consts::OS
    ))
}

impl Backend for Unsupported {
    type Handle = ();

    fn platform(&self) -> &'static str {
        std::env::consts::OS
    }

    fn permissions(&self) -> Value {
        json!({
            "accessibility": "unsupported",
            "screen_recording": "unsupported",
            "background_input": false,
            "notes": ["no computer-use backend for this platform"],
        })
    }

    fn list_apps(&self) -> Result<Vec<AppInfo>, String> {
        no()
    }

    fn list_windows(&self, _pid: Option<i64>) -> Result<Vec<Value>, String> {
        no()
    }

    fn ax_tree(&self, _pid: i64, _d: u32, _n: u32) -> Result<Vec<Node<()>>, String> {
        no()
    }

    fn ax_perform(&self, _h: &(), _action: &str) -> Result<(), String> {
        no()
    }

    fn ax_set_value(&self, _h: &(), _value: &str) -> Result<(), String> {
        no()
    }

    fn ax_focus(&self, _h: &()) -> Result<(), String> {
        no()
    }

    fn screenshot(&self, _pid: Option<i64>, _r: Option<Rect>, _m: u32) -> Result<Capture, String> {
        no()
    }

    fn post_pointer(&self, _route: Route, _at: (f64, f64), _p: Pointer) -> Result<(), String> {
        no()
    }

    fn post_scroll(
        &self,
        _route: Route,
        _at: Option<(f64, f64)>,
        _dx: i64,
        _dy: i64,
    ) -> Result<(), String> {
        no()
    }

    fn post_text(&self, _route: Route, _text: &str) -> Result<(), String> {
        no()
    }

    fn post_key(&self, _route: Route, _chord: &Chord) -> Result<(), String> {
        no()
    }

    fn activate(&self, _pid: i64) -> Result<Option<i64>, String> {
        no()
    }

    fn restore(&self, _previous: Option<i64>) {}

    fn select_all(&self) -> Chord {
        parse_chord("ctrl+a").expect("static chord")
    }
}
