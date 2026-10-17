//! Starlight 0.38.3 colour tokens, the candidate `--vd-*` to `--sl-*` map Phase 89 will ship in
//! the site CSS, WCAG 2.x contrast, and the CSS for the contact sheet's dark and light panes.
//! The map lives only here (and later in the site CSS): the crate itself stays token-agnostic.
//! Floats and `powf` are fine in this file: it is test code, not `src/`.
#![allow(dead_code)]

/// `(token, (hue, saturation %, lightness %))`.
pub type Tokens = [(&'static str, (u16, u16, u16)); 8];

/// Starlight 0.38.3 dark tokens (`style/props.css`, `:root`).
pub const DARK: Tokens = [
    ("--sl-color-white", (0, 0, 100)),
    ("--sl-color-gray-1", (224, 20, 94)),
    ("--sl-color-gray-2", (224, 6, 77)),
    ("--sl-color-gray-3", (224, 6, 56)),
    ("--sl-color-gray-4", (224, 7, 36)),
    ("--sl-color-gray-5", (224, 10, 23)),
    ("--sl-color-gray-6", (224, 14, 16)),
    ("--sl-color-black", (224, 10, 10)),
];

/// Starlight 0.38.3 light tokens (`style/props.css`, `:root[data-theme='light']`).
pub const LIGHT: Tokens = [
    ("--sl-color-white", (224, 10, 10)),
    ("--sl-color-gray-1", (224, 14, 16)),
    ("--sl-color-gray-2", (224, 10, 23)),
    ("--sl-color-gray-3", (224, 7, 36)),
    ("--sl-color-gray-4", (224, 6, 56)),
    ("--sl-color-gray-5", (224, 6, 77)),
    ("--sl-color-gray-6", (224, 20, 94)),
    ("--sl-color-black", (0, 0, 100)),
];

/// `--vd-*` colour variable to the Starlight token that feeds it. `--vd-font` maps to
/// `--sl-font-system` and is not a colour, so it is not listed.
pub const STARLIGHT_MAP: [(&str, &str); 8] = [
    ("--vd-text", "--sl-color-white"),
    ("--vd-muted", "--sl-color-gray-2"),
    ("--vd-node-fill", "--sl-color-black"),
    ("--vd-node-stroke", "--sl-color-gray-3"),
    ("--vd-edge", "--sl-color-gray-3"),
    ("--vd-container-fill", "--sl-color-gray-6"),
    ("--vd-container-stroke", "--sl-color-gray-3"),
    ("--vd-bg", "--sl-color-black"),
];

/// The two Starlight themes.
#[derive(Debug, Clone, Copy)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn tokens(self) -> &'static Tokens {
        match self {
            Self::Dark => &DARK,
            Self::Light => &LIGHT,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

fn hsl_css((h, s, l): (u16, u16, u16)) -> String {
    format!("hsl({h}, {s}%, {l}%)")
}

/// The `hsl(..)` colour of `--vd-*` variable `vd` under `theme`, through [`STARLIGHT_MAP`].
pub fn mapped(theme: Theme, vd: &str) -> Option<String> {
    let (_, sl) = STARLIGHT_MAP.iter().find(|(v, _)| *v == vd)?;
    let (_, hsl) = theme.tokens().iter().find(|(t, _)| t == sl)?;
    Some(hsl_css(*hsl))
}

fn hsl_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    let (s, l) = (s / 100.0, l / 100.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp {
        p if p < 1.0 => (c, x, 0.0),
        p if p < 2.0 => (x, c, 0.0),
        p if p < 3.0 => (0.0, c, x),
        p if p < 4.0 => (0.0, x, c),
        p if p < 5.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r + m, g + m, b + m]
}

/// `#rgb`, `#rrggbb` or `hsl(h, s%, l%)` as sRGB in 0..=1.
fn parse(color: &str) -> Option<[f64; 3]> {
    if let Some(hex) = color.strip_prefix('#') {
        let d: Vec<f64> = hex
            .chars()
            .map(|c| c.to_digit(16).map(f64::from))
            .collect::<Option<_>>()?;
        return match d.len() {
            3 => Some([
                d[0] * 17.0 / 255.0,
                d[1] * 17.0 / 255.0,
                d[2] * 17.0 / 255.0,
            ]),
            6 => Some([
                (d[0] * 16.0 + d[1]) / 255.0,
                (d[2] * 16.0 + d[3]) / 255.0,
                (d[4] * 16.0 + d[5]) / 255.0,
            ]),
            _ => None,
        };
    }
    let inner = color.strip_prefix("hsl(")?.strip_suffix(')')?;
    let n: Vec<f64> = inner
        .split(',')
        .map(|p| p.trim().trim_end_matches('%').parse::<f64>().ok())
        .collect::<Option<_>>()?;
    (n.len() == 3).then(|| hsl_rgb(n[0], n[1], n[2]))
}

fn luminance(c: [f64; 3]) -> f64 {
    let lin = |v: f64| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG 2.x contrast ratio of two colours (`#hex` or `hsl(..)`); panics on an unparsable colour,
/// which is a bug in the test tables.
pub fn contrast(a: &str, b: &str) -> f64 {
    let (Some(x), Some(y)) = (parse(a), parse(b)) else {
        panic!("cannot parse colours {a:?} / {b:?}");
    };
    let (la, lb) = (luminance(x), luminance(y));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// CSS for a contact-sheet pane: the eight `--sl-color-*` tokens as literal `hsl(..)` values and
/// every `--vd-*` colour as `var(--sl-..)` per [`STARLIGHT_MAP`], scoped to `.pane-dark` or
/// `.pane-light`.
pub fn pane_css(theme: Theme) -> String {
    let mut css = format!(".pane-{} {{\n", theme.name());
    for (token, hsl) in theme.tokens() {
        css.push_str(&format!("  {token}: {};\n", hsl_css(*hsl)));
    }
    for (vd, sl) in STARLIGHT_MAP {
        css.push_str(&format!("  {vd}: var({sl});\n"));
    }
    css.push_str("}\n");
    css
}
