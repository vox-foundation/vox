//! WCAG 2.1 relative luminance and contrast ratio computation.

use std::str::FromStr;

/// A declared contrast pair from vox.tokens.json.
#[derive(Debug, Clone)]
pub struct ContrastPair {
    /// CSS-var-style key of the foreground token (e.g. "color-text").
    pub foreground_key: String,
    /// CSS-var-style key of the background token (e.g. "color-background").
    pub background_key: String,
    pub text_role: TextRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    /// Normal body text: warn <4.5:1, error <3:1.
    Body,
    /// Large text (≥18pt or ≥14pt bold): warn <3:1, error <3:1.
    Large,
    /// Non-text UI components and graphical objects: warn <3:1, error <3:1.
    Ui,
}

impl FromStr for TextRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "body" => Ok(TextRole::Body),
            "large" => Ok(TextRole::Large),
            "ui" => Ok(TextRole::Ui),
            _ => Err(()),
        }
    }
}

impl TextRole {
    /// Ratio below which a warning is emitted.
    pub fn warn_threshold(self) -> f64 {
        match self {
            TextRole::Body => 4.5,
            TextRole::Large | TextRole::Ui => 3.0,
        }
    }

    /// Ratio below which an error is emitted (always 3:1 per WCAG 2.1 §1.4.3 minimum).
    pub fn error_threshold(self) -> f64 {
        3.0
    }
}

/// WCAG 2.1 relative luminance of a hex color.
///
/// Returns `None` if the string is not a recognized hex format (#RGB, #RRGGBB, #RRGGBBAA).
pub fn wcag21_relative_luminance(hex: &str) -> Option<f64> {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        (r, g, b)
    } else if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        (r, g, b)
    } else {
        return None;
    };

    let linearize = |c: u8| -> f64 {
        let s = c as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };

    Some(0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b))
}

/// WCAG 2.1 contrast ratio between two hex colors.
///
/// Returns `None` if either value cannot be parsed as a hex color.
pub fn wcag21_contrast_ratio(fg_hex: &str, bg_hex: &str) -> Option<f64> {
    let l1 = wcag21_relative_luminance(fg_hex)?;
    let l2 = wcag21_relative_luminance(bg_hex)?;
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    Some((lighter + 0.05) / (darker + 0.05))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21_to_1() {
        let ratio = wcag21_contrast_ratio("#000000", "#ffffff").unwrap();
        assert!((ratio - 21.0).abs() < 0.01, "got {}", ratio);
    }

    #[test]
    fn white_on_white_is_1_to_1() {
        let ratio = wcag21_contrast_ratio("#ffffff", "#ffffff").unwrap();
        assert!((ratio - 1.0).abs() < 0.01, "got {}", ratio);
    }

    #[test]
    fn shorthand_hex_parses() {
        let ratio = wcag21_contrast_ratio("#000", "#fff").unwrap();
        assert!((ratio - 21.0).abs() < 0.01, "got {}", ratio);
    }

    #[test]
    fn navy_on_white_passes_body_threshold() {
        // #1d3557 on #ffffff — deep navy, should be well above 4.5:1
        let ratio = wcag21_contrast_ratio("#1d3557", "#ffffff").unwrap();
        assert!(
            ratio >= 4.5,
            "expected ≥4.5:1 but got {:.2}:1 — update vox.tokens.json if this token changed",
            ratio
        );
    }

    #[test]
    fn text_role_thresholds() {
        assert_eq!(TextRole::Body.warn_threshold(), 4.5);
        assert_eq!(TextRole::Body.error_threshold(), 3.0);
        assert_eq!(TextRole::Large.warn_threshold(), 3.0);
        assert_eq!(TextRole::Ui.warn_threshold(), 3.0);
    }
}
