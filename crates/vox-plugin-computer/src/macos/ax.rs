//! Accessibility (AXUIElement) tree walking and element actions.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_application_services::{
    AXCopyMultipleAttributeOptions, AXError, AXIsProcessTrusted, AXUIElement, AXValue, AXValueType,
};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFNumber, CFRetained, CFString, CFType, CGPoint, CGSize,
};

use crate::model::{Node, Rect};

/// Owned AX element. AX calls are IPC to the target app and may be made from
/// any thread.
#[derive(Clone)]
pub struct AxHandle(pub CFRetained<AXUIElement>);

// SAFETY: AXUIElementRef is an immutable CF token naming a remote element; the
// AX API is documented as callable from any thread.
unsafe impl Send for AxHandle {}
unsafe impl Sync for AxHandle {}

const ATTRS: [&str; 11] = [
    "AXRole",
    "AXSubrole",
    "AXTitle",
    "AXDescription",
    "AXValue",
    "AXEnabled",
    "AXFocused",
    "AXPosition",
    "AXSize",
    "AXChildren",
    "AXPlaceholderValue",
];

pub fn trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn require_trust() -> Result<(), String> {
    if trusted() {
        Ok(())
    } else {
        Err(
            "accessibility_denied: grant Accessibility to the host app (terminal, IDE, or \
             vox daemon) in System Settings > Privacy & Security > Accessibility"
                .into(),
        )
    }
}

pub fn ax_err(err: AXError, what: &str) -> String {
    match err {
        AXError::InvalidUIElement => format!("stale_ref: {what}: element no longer exists"),
        AXError::AttributeUnsupported | AXError::ActionUnsupported | AXError::NoValue => {
            format!("not_supported: {what}: element does not support it")
        }
        AXError::APIDisabled => format!("accessibility_denied: {what}"),
        AXError::CannotComplete => format!("backend_error: {what}: app did not respond"),
        other => format!("backend_error: {what}: AXError {}", other.0),
    }
}

fn cfstr(s: &str) -> CFRetained<CFString> {
    CFString::from_str(s)
}

fn to_retained(ptr: *const CFType) -> Option<CFRetained<CFType>> {
    // SAFETY: Copy* functions return +1 references we now own.
    NonNull::new(ptr.cast_mut()).map(|p| unsafe { CFRetained::from_raw(p) })
}

pub fn application(pid: i64) -> Result<AxHandle, String> {
    let pid = i32::try_from(pid).map_err(|_| format!("invalid pid {pid}"))?;
    let app = unsafe { AXUIElement::new_application(pid) };
    unsafe {
        app.set_messaging_timeout(2.0);
        // Chromium/Electron build their AX tree only when asked.
        app.set_attribute_value(&cfstr("AXManualAccessibility"), CFBoolean::new(true));
    }
    Ok(AxHandle(app))
}

pub fn copy_attr(el: &AXUIElement, name: &str) -> Result<CFRetained<CFType>, AXError> {
    let mut out: *const CFType = std::ptr::null();
    let err = unsafe { el.copy_attribute_value(&cfstr(name), NonNull::from(&mut out)) };
    if err != AXError::Success {
        return Err(err);
    }
    to_retained(out).ok_or(AXError::NoValue)
}

/// View an untyped CF array as an array of `CFType` so elements can be
/// downcast individually.
pub fn typed(arr: &CFArray) -> &CFArray<CFType> {
    // SAFETY: every CFArray element is a CFTypeRef; `CFArray<T>` is a
    // zero-cost marker over the same CFArrayRef.
    unsafe { &*(std::ptr::from_ref(arr).cast::<CFArray<CFType>>()) }
}
fn string_of(value: &CFType) -> Option<String> {
    if let Some(s) = value.downcast_ref::<CFString>() {
        return Some(s.to_string());
    }
    if let Some(n) = value.downcast_ref::<CFNumber>() {
        return n.as_f64().map(|f| {
            if f.fract() == 0.0 {
                format!("{f:.0}")
            } else {
                f.to_string()
            }
        });
    }
    value
        .downcast_ref::<CFBoolean>()
        .map(|b| b.as_bool().to_string())
}

fn bool_of(value: Option<&CFType>) -> Option<bool> {
    let value = value?;
    value
        .downcast_ref::<CFBoolean>()
        .map(CFBoolean::as_bool)
        .or_else(|| {
            value
                .downcast_ref::<CFNumber>()
                .and_then(CFNumber::as_i64)
                .map(|n| n != 0)
        })
}

fn ax_value<T: Default>(value: Option<&CFType>, ty: AXValueType) -> Option<T> {
    let v = value?.downcast_ref::<AXValue>()?;
    let mut out = T::default();
    let ok = unsafe { v.value(ty, NonNull::from(&mut out).cast::<c_void>()) };
    ok.then_some(out)
}

fn elements(value: Option<&CFType>) -> Vec<CFRetained<AXUIElement>> {
    let Some(arr) = value.and_then(|v| v.downcast_ref::<CFArray>()) else {
        return Vec::new();
    };
    // SAFETY: AX element arrays only contain AXUIElementRefs; we downcast each.
    let arr = typed(arr);
    arr.iter()
        .filter_map(|v| v.downcast::<AXUIElement>().ok())
        .collect()
}

pub fn action_names(el: &AXUIElement) -> Vec<String> {
    let mut out: *const CFArray = std::ptr::null();
    let err = unsafe { el.copy_action_names(NonNull::from(&mut out)) };
    if err != AXError::Success {
        return Vec::new();
    }
    let Some(arr) = to_retained(out.cast()) else {
        return Vec::new();
    };
    let Some(arr) = arr.downcast_ref::<CFArray>() else {
        return Vec::new();
    };
    let arr = typed(arr);
    arr.iter()
        .filter_map(|v| v.downcast_ref::<CFString>().map(|s| s.to_string()))
        .filter_map(|a| {
            Some(
                match a.as_str() {
                    "AXPress" => "press",
                    "AXShowMenu" => "show_menu",
                    "AXConfirm" => "confirm",
                    "AXPick" => "pick",
                    "AXIncrement" => "increment",
                    "AXDecrement" => "decrement",
                    "AXRaise" => "raise",
                    _ => return None,
                }
                .to_string(),
            )
        })
        .collect()
}

pub fn neutral_role(role: &str, subrole: &str) -> String {
    let mapped = match (role, subrole) {
        (_, "AXSecureTextField") => "secure_textbox",
        (_, "AXSearchField") => "searchbox",
        (_, "AXSwitch") => "switch",
        (_, "AXTabButton") => "tab",
        ("AXButton", _) => "button",
        ("AXCheckBox", _) => "checkbox",
        ("AXRadioButton", _) => "radio",
        ("AXTextField" | "AXTextArea", _) => "textbox",
        ("AXComboBox", _) => "combobox",
        ("AXPopUpButton" | "AXMenuButton", _) => "popup",
        ("AXMenuItem", _) => "menuitem",
        ("AXMenuBarItem", _) => "menubaritem",
        ("AXLink", _) => "link",
        ("AXSlider", _) => "slider",
        ("AXRow", _) => "row",
        ("AXCell", _) => "cell",
        ("AXDisclosureTriangle", _) => "disclosure",
        ("AXIncrementor", _) => "incrementor",
        ("AXWindow", _) => "window",
        ("AXSheet", _) => "sheet",
        ("AXApplication", _) => "application",
        ("AXGroup", _) => "group",
        ("AXScrollArea", _) => "scrollarea",
        ("AXSplitGroup", _) => "splitgroup",
        ("AXLayoutArea", _) => "layout",
        ("AXStaticText", _) => "text",
        ("AXImage", _) => "image",
        ("AXTable" | "AXOutline", _) => "table",
        ("AXWebArea", _) => "webarea",
        ("AXHeading", _) => "heading",
        _ => return role.trim_start_matches("AX").to_ascii_lowercase(),
    };
    mapped.to_string()
}

struct Walk {
    budget: u32,
    max_depth: u32,
}

fn node(el: CFRetained<AXUIElement>, depth: u32, walk: &mut Walk) -> Option<Node<AxHandle>> {
    if walk.budget == 0 {
        return None;
    }
    walk.budget -= 1;
    let names: Vec<CFRetained<CFString>> = ATTRS.iter().map(|a| cfstr(a)).collect();
    let attrs = CFArray::from_retained_objects(&names);
    let mut out: *const CFArray = std::ptr::null();
    let err = unsafe {
        el.copy_multiple_attribute_values(
            AsRef::<CFArray>::as_ref(&*attrs),
            AXCopyMultipleAttributeOptions::empty(),
            NonNull::from(&mut out),
        )
    };
    if err != AXError::Success {
        return None;
    }
    let values = to_retained(out.cast())?;
    let values = values.downcast_ref::<CFArray>()?;
    let values = typed(values);
    // Errors come back as AXValue(kAXValueAXErrorType); `get` keeps them, and
    // the typed readers below simply reject them.
    let at = |i: usize| values.get(i);
    let text = |i: usize| at(i).as_deref().and_then(string_of).unwrap_or_default();

    let role = text(0);
    let subrole = text(1);
    let neutral = neutral_role(&role, &subrole);
    let secure = neutral == "secure_textbox";
    let raw_value = if secure {
        None
    } else {
        at(4).as_deref().and_then(string_of)
    };
    let mut name = text(2);
    if name.is_empty() {
        name = text(3);
    }
    if name.is_empty() && neutral == "text" {
        name = raw_value.clone().unwrap_or_default();
    }
    if name.is_empty() {
        name = text(10);
    }
    let value = if neutral == "text" { None } else { raw_value };
    let pos: Option<CGPoint> = ax_value(at(7).as_deref(), AXValueType::CGPoint);
    let size: Option<CGSize> = ax_value(at(8).as_deref(), AXValueType::CGSize);
    let frame = pos.zip(size).map(|(p, s)| Rect {
        x: p.x,
        y: p.y,
        w: s.width,
        h: s.height,
    });
    let children = if depth < walk.max_depth {
        elements(at(9).as_deref())
            .into_iter()
            .filter_map(|c| node(c, depth + 1, walk))
            .collect()
    } else {
        Vec::new()
    };
    let actions = action_names(&el);
    Some(Node {
        handle: AxHandle(el),
        role: neutral,
        name,
        value,
        enabled: bool_of(at(5).as_deref()).unwrap_or(true),
        focused: bool_of(at(6).as_deref()).unwrap_or(false),
        secure,
        frame,
        actions,
        children,
    })
}

/// Windows of `pid` as accessibility trees (focused window first).
pub fn tree(pid: i64, max_depth: u32, max_nodes: u32) -> Result<Vec<Node<AxHandle>>, String> {
    require_trust()?;
    let app = application(pid)?;
    let windows = copy_attr(&app.0, "AXWindows").map_err(|e| ax_err(e, "AXWindows"))?;
    let mut windows = elements(Some(&windows));
    if let Ok(focused) = copy_attr(&app.0, "AXFocusedWindow")
        && let Ok(focused) = focused.downcast::<AXUIElement>()
        && let Some(i) = windows.iter().position(|w| **w == *focused)
    {
        let w = windows.remove(i);
        windows.insert(0, w);
    }
    let mut walk = Walk {
        budget: max_nodes,
        max_depth,
    };
    Ok(windows
        .into_iter()
        .filter_map(|w| node(w, 0, &mut walk))
        .collect())
}

pub fn perform(h: &AxHandle, action: &str) -> Result<(), String> {
    require_trust()?;
    let native = match action {
        "press" => "AXPress",
        "show_menu" => "AXShowMenu",
        "confirm" => "AXConfirm",
        "pick" => "AXPick",
        "raise" => "AXRaise",
        other => return Err(format!("not_supported: unknown action {other}")),
    };
    let err = unsafe { h.0.perform_action(&cfstr(native)) };
    if err == AXError::Success {
        Ok(())
    } else {
        Err(ax_err(err, native))
    }
}

pub fn set_value(h: &AxHandle, value: &str) -> Result<(), String> {
    require_trust()?;
    let v = cfstr(value);
    let err = unsafe { h.0.set_attribute_value(&cfstr("AXValue"), &v) };
    if err != AXError::Success {
        return Err(ax_err(err, "AXValue"));
    }
    // Some controls accept the write and ignore it; read back to be sure.
    match copy_attr(&h.0, "AXValue")
        .ok()
        .as_deref()
        .and_then(string_of)
    {
        Some(now) if now == value => Ok(()),
        _ => Err("not_supported: AXValue write was ignored by the app".into()),
    }
}

pub fn focus(h: &AxHandle) -> Result<(), String> {
    require_trust()?;
    let err = unsafe {
        h.0.set_attribute_value(&cfstr("AXFocused"), CFBoolean::new(true))
    };
    if err == AXError::Success {
        Ok(())
    } else {
        Err(ax_err(err, "AXFocused"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_neutral_names() {
        assert_eq!(
            neutral_role("AXTextField", "AXSecureTextField"),
            "secure_textbox"
        );
        assert_eq!(neutral_role("AXTextField", ""), "textbox");
        assert_eq!(neutral_role("AXRadioButton", "AXTabButton"), "tab");
        assert_eq!(neutral_role("AXButton", "AXCloseButton"), "button");
        assert_eq!(neutral_role("AXValueIndicator", ""), "valueindicator");
    }
}
