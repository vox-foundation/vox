//! Normalize v0.dev (and similar) TSX so Vox `routes:` → **TanStack Router** codegen can use **named** imports (`import { Name } from "./Name.tsx"`). Used for the main generated app and for **`islands/`** when the **`island`** feature is enabled.

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
}
