//! REND-04: every colour in every SVG is a `--vd-*` variable with a literal fallback, all nine
//! variables are used, and the candidate Starlight map (plus the fallback palette) is legible in
//! both themes. A failing threshold is a contract change for Phase 89, not a tweak here.

mod support;

use std::collections::BTreeSet;
use std::fs;

use support::roundtrip::{fixtures, stems};
use support::svgcheck::check;
use support::theme::{DARK, LIGHT, STARLIGHT_MAP, Theme, contrast, mapped, pane_css};
use vox_diagram::svg::THEME_VARS;

/// `(file stem, svg text)` of every case golden.
fn goldens() -> Vec<(String, String)> {
    let dir = fixtures().join("svg/cases");
    let out: Vec<(String, String)> = stems(&dir, "svg")
        .into_iter()
        .map(|stem| {
            let path = dir.join(format!("{stem}.svg"));
            match fs::read_to_string(&path) {
                Ok(svg) => (stem, svg),
                Err(e) => panic!("cannot read {}: {e}", path.display()),
            }
        })
        .collect();
    assert!(out.len() >= 10, "only {} goldens found", out.len());
    out
}

/// `var(NAME,FALLBACK)` split into its parts.
fn var_parts(v: &str) -> Option<(&str, &str)> {
    v.strip_prefix("var(")?.strip_suffix(')')?.split_once(',')
}

#[test]
fn contrast_matches_known_ratios() {
    assert!((contrast("#000", "#fff") - 21.0).abs() < 0.01);
    assert!((contrast("#57606a", "#fff") - 6.39).abs() < 0.05);
    assert!((contrast("hsl(0, 0%, 0%)", "hsl(0, 0%, 100%)") - 21.0).abs() < 0.01);
}

#[test]
fn paints_are_theme_vars() {
    for (stem, svg) in goldens() {
        let parsed = match check(&svg, "vd") {
            Ok(p) => p,
            Err(e) => panic!("{stem}: {e}"),
        };
        for (prop, value) in &parsed.paints {
            if !matches!(prop.as_str(), "fill" | "stroke" | "font-family") {
                continue;
            }
            if value == "none" {
                continue;
            }
            let Some((name, fallback)) = var_parts(value) else {
                panic!("{stem}: {prop}:{value} is not var(NAME,FALLBACK) or none");
            };
            assert!(
                THEME_VARS.contains(&(name, fallback)),
                "{stem}: {prop}:{value} is not a THEME_VARS pair"
            );
        }
        // No colour literal outside a var() fallback, anywhere in an attribute.
        for attr in &parsed.attr_values {
            let mut rest = attr.clone();
            while let Some(i) = rest.find("var(") {
                let Some(len) = rest[i..].find(')') else {
                    panic!("{stem}: unterminated var( in {attr:?}");
                };
                rest.replace_range(i..i + len + 1, "");
            }
            let bare = rest.replace("url(#", "");
            assert!(
                !bare.contains('#') && !bare.contains("rgb(") && !bare.contains("hsl("),
                "{stem}: colour literal outside var() in {attr:?}"
            );
        }
    }
}

#[test]
fn all_theme_vars_used() {
    let mut used = BTreeSet::new();
    for (stem, svg) in goldens() {
        let parsed = match check(&svg, "vd") {
            Ok(p) => p,
            Err(e) => panic!("{stem}: {e}"),
        };
        for (_, value) in &parsed.paints {
            if let Some((name, _)) = var_parts(value) {
                used.insert(name.to_owned());
            }
        }
    }
    let all: BTreeSet<String> = THEME_VARS.iter().map(|(n, _)| (*n).to_owned()).collect();
    assert_eq!(used, all);
}

type Palette<'a> = Box<dyn Fn(&str) -> String + 'a>;

fn assert_pairs(label: &str, color: Palette<'_>) {
    let text = [
        ("--vd-text", "--vd-node-fill"),
        ("--vd-text", "--vd-container-fill"),
        ("--vd-muted", "--vd-bg"),
    ];
    let graphic = [
        ("--vd-edge", "--vd-bg"),
        ("--vd-edge", "--vd-container-fill"),
        ("--vd-node-stroke", "--vd-bg"),
        ("--vd-node-stroke", "--vd-container-fill"),
        // The frame is the only visual cue that nodes are grouped (WCAG 1.4.11).
        ("--vd-container-stroke", "--vd-bg"),
        ("--vd-container-stroke", "--vd-container-fill"),
    ];
    for (pairs, min) in [(text.as_slice(), 4.5), (graphic.as_slice(), 3.0)] {
        for (fg, bg) in pairs {
            let ratio = contrast(&color(fg), &color(bg));
            println!("{label}: {fg} on {bg} = {ratio:.2} (need {min})");
            assert!(
                ratio >= min,
                "{label}: {fg} on {bg} is {ratio:.2}, need {min}"
            );
        }
    }
}

#[test]
fn contrast_holds() {
    for theme in [Theme::Dark, Theme::Light] {
        assert_pairs(
            theme.name(),
            Box::new(move |vd| mapped(theme, vd).unwrap_or_else(|| panic!("{vd} unmapped"))),
        );
    }
    assert_pairs(
        "fallback",
        Box::new(|vd| match THEME_VARS.iter().find(|(n, _)| *n == vd) {
            Some((_, f)) => (*f).to_owned(),
            None => panic!("{vd} not in THEME_VARS"),
        }),
    );
}

#[test]
fn the_map_covers_every_colour_variable_with_real_tokens() {
    let colours: BTreeSet<&str> = THEME_VARS
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| *n != "--vd-font")
        .collect();
    let mapped_names: BTreeSet<&str> = STARLIGHT_MAP.iter().map(|(v, _)| *v).collect();
    assert_eq!(colours, mapped_names);
    for (_, sl) in STARLIGHT_MAP {
        assert!(
            DARK.iter().any(|(t, _)| *t == sl) && LIGHT.iter().any(|(t, _)| *t == sl),
            "{sl}"
        );
    }
}

#[test]
fn one_markup_for_both_themes() {
    for (stem, svg) in goldens() {
        assert_eq!(svg.matches("<svg").count(), 1, "{stem}");
        for banned in ["prefers-color-scheme", "data-theme", "@media"] {
            assert!(!svg.contains(banned), "{stem} contains {banned}");
        }
    }
}

#[test]
fn pane_css_sets_tokens_and_maps_variables() {
    for theme in [Theme::Dark, Theme::Light] {
        let css = pane_css(theme);
        assert_eq!(
            css.matches("--sl-color-").count() - STARLIGHT_MAP.len(),
            8,
            "{css}"
        );
        for (vd, sl) in STARLIGHT_MAP {
            assert!(css.contains(&format!("{vd}: var({sl});")), "{vd}\n{css}");
        }
        assert!(css.contains("hsl("), "{css}");
    }
    assert!(pane_css(Theme::Dark).contains("--sl-color-gray-6: hsl(224, 14%, 16%);"));
}
