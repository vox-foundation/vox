//! Synthetic input. `Route::Process` posts with `CGEventPostToPid`, which
//! delivers to the app without moving the user's cursor or changing focus;
//! `Route::Global` posts to the HID stream (used only after activation) and
//! puts the cursor back afterwards.

use std::thread::sleep;
use std::time::Duration;

use objc2_core_foundation::{CFRetained, CGPoint};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGEventType, CGMouseButton, CGScrollEventUnit, CGWarpMouseCursorPosition,
};

use crate::computer::{Button, Pointer, Route};
use crate::model::Chord;

const STEP: Duration = Duration::from_millis(12);
const DRAG_STEPS: u32 = 12;
/// `CGEventKeyboardSetUnicodeString` silently truncates long strings.
const UNICODE_CHUNK: usize = 16;

fn source() -> Option<CFRetained<CGEventSource>> {
    CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
}

fn post(route: Route, ev: &CGEvent) {
    match route {
        Route::Process(pid) => CGEvent::post_to_pid(pid as i32, Some(ev)),
        Route::Global => CGEvent::post(CGEventTapLocation::HIDEventTap, Some(ev)),
    }
}

fn cursor() -> Option<CGPoint> {
    CGEvent::new(None).map(|e| CGEvent::location(Some(&e)))
}

fn mouse(
    kind: CGEventType,
    at: (f64, f64),
    button: CGMouseButton,
) -> Result<CFRetained<CGEvent>, String> {
    let src = source();
    CGEvent::new_mouse_event(src.as_deref(), kind, CGPoint { x: at.0, y: at.1 }, button)
        .ok_or_else(|| "backend_error: could not create mouse event".to_string())
}

pub fn pointer(route: Route, at: (f64, f64), p: Pointer) -> Result<(), String> {
    let saved = (route == Route::Global).then(cursor).flatten();
    let result = pointer_inner(route, at, p);
    if let Some(pos) = saved {
        let _ = CGWarpMouseCursorPosition(pos);
    }
    result
}

fn pointer_inner(route: Route, at: (f64, f64), p: Pointer) -> Result<(), String> {
    match p {
        Pointer::Move => post(
            route,
            &*mouse(CGEventType::MouseMoved, at, CGMouseButton::Left)?,
        ),
        Pointer::Click { button, count } => {
            let (down, up, cg) = match button {
                Button::Left => (
                    CGEventType::LeftMouseDown,
                    CGEventType::LeftMouseUp,
                    CGMouseButton::Left,
                ),
                Button::Right => (
                    CGEventType::RightMouseDown,
                    CGEventType::RightMouseUp,
                    CGMouseButton::Right,
                ),
            };
            post(route, &*mouse(CGEventType::MouseMoved, at, cg)?);
            for click in 1..=i64::from(count.max(1)) {
                for kind in [down, up] {
                    let ev = mouse(kind, at, cg)?;
                    CGEvent::set_integer_value_field(
                        Some(&ev),
                        CGEventField::MouseEventClickState,
                        click,
                    );
                    post(route, &ev);
                    sleep(STEP);
                }
            }
        }
        Pointer::Drag { to } => {
            post(
                route,
                &*mouse(CGEventType::LeftMouseDown, at, CGMouseButton::Left)?,
            );
            for i in 1..=DRAG_STEPS {
                let t = f64::from(i) / f64::from(DRAG_STEPS);
                let p = (at.0 + (to.0 - at.0) * t, at.1 + (to.1 - at.1) * t);
                post(
                    route,
                    &*mouse(CGEventType::LeftMouseDragged, p, CGMouseButton::Left)?,
                );
                sleep(STEP);
            }
            post(
                route,
                &*mouse(CGEventType::LeftMouseUp, to, CGMouseButton::Left)?,
            );
        }
    }
    Ok(())
}

pub fn scroll(route: Route, at: Option<(f64, f64)>, dx: i64, dy: i64) -> Result<(), String> {
    let clamp = |v: i64| -> i32 { (-v).clamp(-100, 100) as i32 };
    let src = source();
    let ev = CGEvent::new_scroll_wheel_event2(
        src.as_deref(),
        CGScrollEventUnit::Line,
        2,
        clamp(dy),
        clamp(dx),
        0,
    )
    .ok_or("backend_error: could not create scroll event")?;
    if let Some(at) = at {
        CGEvent::set_location(Some(&ev), CGPoint { x: at.0, y: at.1 });
    }
    post(route, &ev);
    Ok(())
}

fn flags(chord: &Chord) -> CGEventFlags {
    let mut f = CGEventFlags::empty();
    if chord.mods.cmd {
        f |= CGEventFlags::MaskCommand;
    }
    if chord.mods.ctrl {
        f |= CGEventFlags::MaskControl;
    }
    if chord.mods.alt {
        f |= CGEventFlags::MaskAlternate;
    }
    if chord.mods.shift {
        f |= CGEventFlags::MaskShift;
    }
    f
}

/// ANSI-layout virtual key codes (`HIToolbox/Events.h`).
pub fn keycode(key: &str) -> Option<u16> {
    Some(match key {
        "a" => 0,
        "s" => 1,
        "d" => 2,
        "f" => 3,
        "h" => 4,
        "g" => 5,
        "z" => 6,
        "x" => 7,
        "c" => 8,
        "v" => 9,
        "b" => 11,
        "q" => 12,
        "w" => 13,
        "e" => 14,
        "r" => 15,
        "y" => 16,
        "t" => 17,
        "1" => 18,
        "2" => 19,
        "3" => 20,
        "4" => 21,
        "6" => 22,
        "5" => 23,
        "=" => 24,
        "9" => 25,
        "7" => 26,
        "-" => 27,
        "8" => 28,
        "0" => 29,
        "]" => 30,
        "o" => 31,
        "u" => 32,
        "[" => 33,
        "i" => 34,
        "p" => 35,
        "l" => 37,
        "j" => 38,
        "'" => 39,
        "k" => 40,
        ";" => 41,
        "\\" => 42,
        "," => 43,
        "/" => 44,
        "n" => 45,
        "m" => 46,
        "." => 47,
        "`" => 50,
        "enter" => 36,
        "tab" => 48,
        "space" => 49,
        "backspace" => 51,
        "escape" => 53,
        "delete" => 117,
        "home" => 115,
        "end" => 119,
        "pageup" => 116,
        "pagedown" => 121,
        "left" => 123,
        "right" => 124,
        "down" => 125,
        "up" => 126,
        "f1" => 122,
        "f2" => 120,
        "f3" => 99,
        "f4" => 118,
        "f5" => 96,
        "f6" => 97,
        "f7" => 98,
        "f8" => 100,
        "f9" => 101,
        "f10" => 109,
        "f11" => 103,
        "f12" => 111,
        _ => return None,
    })
}

fn key_event(
    code: u16,
    down: bool,
    f: CGEventFlags,
    unicode: Option<&[u16]>,
) -> Result<CFRetained<CGEvent>, String> {
    let src = source();
    let ev = CGEvent::new_keyboard_event(src.as_deref(), code, down)
        .ok_or("backend_error: could not create key event")?;
    CGEvent::set_flags(Some(&ev), f);
    if let Some(units) = unicode {
        // SAFETY: pointer/len describe a live UTF-16 slice.
        unsafe {
            CGEvent::keyboard_set_unicode_string(Some(&ev), units.len() as _, units.as_ptr())
        };
    }
    Ok(ev)
}

pub fn key(route: Route, chord: &Chord) -> Result<(), String> {
    let f = flags(chord);
    let (code, unicode): (u16, Option<Vec<u16>>) = match keycode(&chord.key) {
        Some(code) => (code, None),
        None => (0, Some(chord.key.encode_utf16().collect())),
    };
    for down in [true, false] {
        post(route, &*key_event(code, down, f, unicode.as_deref())?);
        sleep(STEP);
    }
    Ok(())
}

pub fn text(route: Route, text: &str) -> Result<(), String> {
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            key(
                route,
                &Chord {
                    mods: Default::default(),
                    key: "enter".into(),
                },
            )?;
        }
        let units: Vec<u16> = line.encode_utf16().collect();
        for chunk in units.chunks(UNICODE_CHUNK) {
            for down in [true, false] {
                post(
                    route,
                    &*key_event(0, down, CGEventFlags::empty(), Some(chunk))?,
                );
            }
            sleep(STEP);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NAMED_KEYS, parse_chord};

    #[test]
    fn every_named_key_has_a_keycode() {
        for key in NAMED_KEYS {
            assert!(keycode(key).is_some(), "no macOS keycode for {key}");
        }
        assert_eq!(keycode("a"), Some(0));
        assert_eq!(keycode("enter"), Some(36));
    }

    #[test]
    fn chord_flags_cover_all_modifiers() {
        let f = flags(&parse_chord("cmd+ctrl+alt+shift+x").unwrap());
        for m in [
            CGEventFlags::MaskCommand,
            CGEventFlags::MaskControl,
            CGEventFlags::MaskAlternate,
            CGEventFlags::MaskShift,
        ] {
            assert!(f.contains(m));
        }
    }
}
