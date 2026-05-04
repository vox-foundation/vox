//! Normalize v0.dev (and similar) TSX so Vox `routes:` → **TanStack Router** codegen can use **named** imports (`import { Name } from "./Name.tsx"`).

use regex::Regex;
use std::path::Path;

use walkdir::WalkDir;

/// Ensures `component_name` is exported as a named function suitable for
/// `import { Name } from "./Name.tsx"` in generated `App.tsx`.
#[must_use]
pub fn normalize_v0_tsx_named_export(mut tsx: String, component_name: &str) -> String {
    let df = format!("export default function {component_name}");
    let ef = format!("export function {component_name}");
    if tsx.contains(&df) {
        tsx = tsx.replace(&df, &ef);
    }

    let dc = format!("export default const {component_name}");
    let ec = format!("export const {component_name}");
    if tsx.contains(&dc) {
        tsx = tsx.replace(&dc, &ec);
    }

    let d_arrow = format!("export default {component_name}");
    if tsx.contains(&d_arrow)
        && !tsx.contains(&ef)
        && !tsx.contains(&format!("export function {component_name}"))
    {
        tsx = tsx.replace(&d_arrow, &ef);
    }

    tsx
}

/// Returns a human-readable failure reason when `tsx` cannot satisfy TanStack `routes:` imports of
/// the form `import { component_name } from "./component_name.tsx"`.
#[must_use]
pub fn v0_named_export_violation(tsx: &str, component_name: &str) -> Option<String> {
    let escaped = regex::escape(component_name);
    let re_fn = Regex::new(&format!(r"\bexport\s+function\s+{escaped}\b")).ok()?;
    let re_async_fn = Regex::new(&format!(r"\bexport\s+async\s+function\s+{escaped}\b")).ok()?;
    let re_const = Regex::new(&format!(r"\bexport\s+const\s+{escaped}\b")).ok()?;
    let re_list = Regex::new(&format!(r"export\s*\{{[^{{}}]*\b{escaped}\b[^{{}}]*\}}")).ok()?;

    if re_fn.is_match(tsx)
        || re_async_fn.is_match(tsx)
        || re_const.is_match(tsx)
        || re_list.is_match(tsx)
    {
        return None;
    }

    let hint = if tsx.contains("export default") {
        format!(
            "{}.tsx: `export default` without a matching named export (`export function {0}`, `export const {0}`, etc.). \
             Vox codegen imports a named binding; re-run `vox build` after v0 generation or apply `normalize_v0_tsx_named_export`.",
            component_name
        )
    } else {
        format!(
            "{}.tsx: no named export for `{0}` — add `export function {0}` (or const / async / `export {{ {0} }}`) so `routes:` can import it.",
            component_name
        )
    };
    Some(hint)
}

fn skip_walkdir_entry(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "node_modules" | "target" | ".git" | "dist" | "build" | ".next"
            )
        })
}

/// Collect `@v0` component names from `.vox` files under `root` (for optional `vox doctor` checks).
#[must_use]
pub fn scan_v0_component_names_from_vox_sources(root: &Path) -> Vec<String> {
    let re = Regex::new(r#"@v0\s+(?:from\s+"[^"]+"|"[^"]*")\s+(?:fn\s+)?(\w+)"#)
        .expect("static @v0 scan regex");
    let mut names = Vec::new();
    let walker = WalkDir::new(root)
        .max_depth(14)
        .into_iter()
        .filter_entry(|e| !skip_walkdir_entry(e.path()));
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("vox") {
            continue;
        }
        let Ok(content) = crate::commands::ci::bounded_read::read_utf8_path_capped(entry.path())
        else {
            continue;
        };
        for cap in re.captures_iter(&content) {
            if let Some(m) = cap.get(1) {
                names.push(m.as_str().to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_function_becomes_named_export() {
        let src = "export default function Dashboard() {\n  return <div />;\n}\n";
        let out = normalize_v0_tsx_named_export(src.to_string(), "Dashboard");
        assert!(out.contains("export function Dashboard"));
        assert!(!out.contains("export default function Dashboard"));
    }

    #[test]
    fn violation_when_only_default_export() {
        let s = "export default function Dashboard() { return null; }\n";
        assert!(v0_named_export_violation(s, "Dashboard").is_some());
    }

    #[test]
    fn no_violation_named_function() {
        let s = "export function Dashboard() { return null; }\n";
        assert!(v0_named_export_violation(s, "Dashboard").is_none());
    }

    #[test]
    fn no_violation_async_function() {
        let s = "export async function Dashboard() { return null; }\n";
        assert!(v0_named_export_violation(s, "Dashboard").is_none());
    }

    #[test]
    fn no_violation_export_const() {
        let s = "export const Dashboard = () => null;\n";
        assert!(v0_named_export_violation(s, "Dashboard").is_none());
    }

    #[test]
    fn no_violation_export_list() {
        let s = "function Dashboard() { return null; }\nexport { Dashboard };\n";
        assert!(v0_named_export_violation(s, "Dashboard").is_none());
    }

    #[test]
    fn normalized_source_passes_contract() {
        let src = "export default function Dashboard() { return null; }\n";
        let fixed = normalize_v0_tsx_named_export(src.to_string(), "Dashboard");
        assert!(v0_named_export_violation(&fixed, "Dashboard").is_none());
    }

    #[test]
    fn scan_finds_v0_declarations_in_vox_tree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sub = tmp.path().join("pkg");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::write(
            sub.join("ui.vox"),
            "@v0 \"A panel\" Stats {}\n\n@v0 from \"x.png\" fn Gallery() to Element\n",
        )
        .expect("write vox");
        let mut names = scan_v0_component_names_from_vox_sources(tmp.path());
        names.sort();
        assert_eq!(names, vec!["Gallery".to_string(), "Stats".to_string()]);
    }
}
