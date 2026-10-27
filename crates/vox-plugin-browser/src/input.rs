use chromiumoxide_cdp::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams, DispatchMouseEventType,
    InsertTextParams, MouseButton,
};

use crate::engine::BrowserEngine;
use crate::resolve::resolve_element;

/// Focus the element and select its current contents so the next text
/// insertion replaces the existing value instead of appending to it.
const SELECT_CONTENTS_JS: &str = r#"function () {
  this.focus();
  if (typeof this.select === "function") {
    this.select();
  } else if (this.isContentEditable) {
    const range = document.createRange();
    range.selectNodeContents(this);
    const sel = getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
  }
}"#;

impl BrowserEngine {
    pub async fn click(&self, page_id: &str, target: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let el = resolve_element(&page, target).await?;
        el.click().await.map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn click_xy(&self, page_id: &str, x: f64, y: f64) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        page.execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseMoved)
                .x(x)
                .y(y)
                .build()
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(Self::map_page_err)?;
        page.execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MousePressed)
                .x(x)
                .y(y)
                .button(MouseButton::Left)
                .buttons(1)
                .click_count(1)
                .build()
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(Self::map_page_err)?;
        page.execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseReleased)
                .x(x)
                .y(y)
                .button(MouseButton::Left)
                .buttons(0)
                .click_count(1)
                .build()
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn fill(&self, page_id: &str, target: &str, value: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let el = resolve_element(&page, target).await?;
        el.click().await.map_err(Self::map_page_err)?;
        el.call_js_fn(SELECT_CONTENTS_JS, false)
            .await
            .map_err(Self::map_page_err)?;
        if value.is_empty() {
            self.press(page_id, "Delete").await?;
        } else {
            self.type_text(page_id, value).await?;
        }
        Ok(())
    }

    pub async fn scroll(&self, page_id: &str, dx: i64, dy: i64) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let vp = self.viewport_for(page_id).await;
        let x = (vp.width as f64) / 2.0;
        let y = (vp.height as f64) / 2.0;
        page.execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseWheel)
                .x(x)
                .y(y)
                .delta_x(dx as f64)
                .delta_y(dy as f64)
                .build()
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(Self::map_page_err)?;
        page.execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseMoved)
                .x(x)
                .y(y)
                .build()
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn type_text(&self, page_id: &str, text: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        if text.is_empty() {
            return Ok(());
        }
        page.execute(InsertTextParams::new(text))
            .await
            .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn press(&self, page_id: &str, key: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let chord = KeyChord::parse(key);
        let down_type = if chord.text.is_some() {
            DispatchKeyEventType::KeyDown
        } else {
            DispatchKeyEventType::RawKeyDown
        };
        let mut down = DispatchKeyEventParams::builder()
            .r#type(down_type)
            .modifiers(chord.modifiers)
            .key(chord.key.clone())
            .code(chord.code.clone())
            .windows_virtual_key_code(chord.windows_vk)
            .native_virtual_key_code(chord.windows_vk)
            .build()
            .map_err(|e| e.to_string())?;
        down.text = chord.text.clone();
        down.unmodified_text = chord.text.clone();
        page.execute(down).await.map_err(Self::map_page_err)?;
        let up = DispatchKeyEventParams::builder()
            .r#type(DispatchKeyEventType::KeyUp)
            .modifiers(chord.modifiers)
            .key(chord.key)
            .code(chord.code)
            .windows_virtual_key_code(chord.windows_vk)
            .native_virtual_key_code(chord.windows_vk)
            .build()
            .map_err(|e| e.to_string())?;
        page.execute(up).await.map_err(Self::map_page_err)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct KeyChord {
    pub(crate) modifiers: i64,
    pub(crate) key: String,
    pub(crate) code: String,
    pub(crate) windows_vk: i64,
    pub(crate) text: Option<String>,
}

impl KeyChord {
    pub(crate) fn parse(raw: &str) -> Self {
        let mut modifiers = 0_i64;
        let mut key_token = None::<&str>;
        for token in raw.split('+').map(str::trim).filter(|t| !t.is_empty()) {
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= 2,
                "alt" | "option" => modifiers |= 1,
                "meta" | "cmd" | "command" => modifiers |= 4,
                "shift" => modifiers |= 8,
                _ => key_token = Some(token),
            }
        }
        let key = key_token.unwrap_or(raw).trim();
        let (norm_key, code, windows_vk) = key_identity(key);
        let text = key_text(&norm_key, modifiers);
        Self {
            modifiers,
            key: norm_key,
            code,
            windows_vk,
            text,
        }
    }
}

/// Text a key press inserts, which Chrome needs on `keyDown` to type a
/// character or submit a form on Enter. Chords with Ctrl, Alt or Meta are
/// shortcuts and insert nothing.
pub(crate) fn key_text(key: &str, modifiers: i64) -> Option<String> {
    const CTRL_ALT_META: i64 = 1 | 2 | 4;
    const SHIFT: i64 = 8;
    if modifiers & CTRL_ALT_META != 0 {
        return None;
    }
    match key {
        "Enter" => Some("\r".to_string()),
        " " => Some(" ".to_string()),
        _ if key.chars().count() == 1 => {
            if modifiers & SHIFT != 0 {
                Some(key.to_uppercase())
            } else {
                Some(key.to_string())
            }
        }
        _ => None,
    }
}

pub(crate) fn key_identity(key: &str) -> (String, String, i64) {
    match key {
        "Enter" => ("Enter".into(), "Enter".into(), 13),
        "Tab" => ("Tab".into(), "Tab".into(), 9),
        "Escape" | "Esc" => ("Escape".into(), "Escape".into(), 27),
        "Backspace" => ("Backspace".into(), "Backspace".into(), 8),
        "Delete" => ("Delete".into(), "Delete".into(), 46),
        "Home" => ("Home".into(), "Home".into(), 36),
        "End" => ("End".into(), "End".into(), 35),
        "PageUp" => ("PageUp".into(), "PageUp".into(), 33),
        "PageDown" => ("PageDown".into(), "PageDown".into(), 34),
        "ArrowUp" => ("ArrowUp".into(), "ArrowUp".into(), 38),
        "ArrowDown" => ("ArrowDown".into(), "ArrowDown".into(), 40),
        "ArrowLeft" => ("ArrowLeft".into(), "ArrowLeft".into(), 37),
        "ArrowRight" => ("ArrowRight".into(), "ArrowRight".into(), 39),
        "Space" | " " => (" ".into(), "Space".into(), 32),
        _ if key.chars().count() == 1 => {
            let ch = key.chars().next().unwrap_or_default();
            if ch.is_ascii_alphabetic() {
                // DOM `key` preserves the case as given (Shift is a separate
                // modifier); `code` and the Windows VK use the uppercase identity.
                let upper = ch.to_ascii_uppercase();
                let vk = upper as i64;
                (ch.to_string(), format!("Key{upper}"), vk)
            } else if ch.is_ascii_digit() {
                let vk = ch as i64;
                (ch.to_string(), format!("Digit{ch}"), vk)
            } else {
                (key.to_string(), key.to_string(), 0)
            }
        }
        _ => (key.to_string(), key.to_string(), 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_keys_and_enter_carry_text() {
        assert_eq!(KeyChord::parse("a").text.as_deref(), Some("a"));
        assert_eq!(KeyChord::parse("Shift+a").text.as_deref(), Some("A"));
        assert_eq!(KeyChord::parse("Enter").text.as_deref(), Some("\r"));
        assert_eq!(KeyChord::parse("Space").text.as_deref(), Some(" "));
        assert_eq!(KeyChord::parse("7").text.as_deref(), Some("7"));
    }

    #[test]
    fn shortcuts_and_navigation_keys_carry_no_text() {
        assert_eq!(KeyChord::parse("Ctrl+a").text, None);
        assert_eq!(KeyChord::parse("Meta+Enter").text, None);
        assert_eq!(KeyChord::parse("Alt+x").text, None);
        assert_eq!(KeyChord::parse("Tab").text, None);
        assert_eq!(KeyChord::parse("ArrowDown").text, None);
        assert_eq!(KeyChord::parse("Backspace").text, None);
    }
}
