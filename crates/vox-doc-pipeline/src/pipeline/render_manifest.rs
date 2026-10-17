//! `--render-diagrams`: pre-render every ```` ```vox-diagram ```` fence under `docs/src` into the
//! gitignored `docs-astro/.vox-diagram/manifest.json` (VoxDiagram Phase 89, D-01/D-02).

use std::collections::BTreeMap;
use std::path::Path;

use vox_diagram::{Emit, ValidateOpts, fence, validate};

use super::lint::gather_md_files;

const INFO: &str = "vox-diagram";
const DEFAULT_OUT: &str = "docs-astro/.vox-diagram/manifest.json";

/// Pre-render every `vox-diagram` fence under `root/docs/src` (`archive/` skipped) into the
/// manifest `{schema_version: 1, diagrams: [{source, svg}]}`, keyed by the normalized fence source.
pub(crate) fn run(root: &Path, _args: &[String]) -> i32 {
    let mut files = Vec::new();
    gather_md_files(&root.join("docs/src"), &mut files);
    files.sort();
    let mut diagrams: BTreeMap<String, String> = BTreeMap::new();
    let mut errors = 0;
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("docs/src/archive/") {
            continue;
        }
        let text = match vox_bounded_fs::read_utf8_path_capped(f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{rel}: unreadable: {e}");
                return 1;
            }
        };
        for fence in fence::extract(&text, INFO) {
            let key = fence::normalize(&fence.body);
            if diagrams.contains_key(&key) {
                continue;
            }
            let v = validate(
                &key,
                ValidateOpts {
                    from_mermaid: false,
                    emit: Some(Emit::Svg),
                },
            );
            match v.emitted {
                Some(svg) => {
                    diagrams.insert(key, svg);
                }
                None => {
                    eprintln!(
                        "{rel}:{}:1: diagram-render: no SVG emitted",
                        fence.open_line
                    );
                    errors += 1;
                }
            }
        }
    }
    let entries: Vec<serde_json::Value> = diagrams
        .iter()
        .map(|(source, svg)| serde_json::json!({ "source": source, "svg": svg }))
        .collect();
    let manifest = serde_json::json!({ "schema_version": 1, "diagrams": entries });
    let out = root.join(DEFAULT_OUT);
    let written = out
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&out, format!("{manifest}\n")));
    if let Err(e) = written {
        eprintln!("{}: {e}", out.display());
        return 1;
    }
    println!(
        "render-diagrams: {} diagram(s) from {} file(s) -> {}",
        diagrams.len(),
        files.len(),
        out.display()
    );
    i32::from(errors > 0)
}

#[cfg(test)]
mod tests {
    use crate::pipeline::lint_in;

    const GRAPH: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

    fn root(name: &str) -> std::path::PathBuf {
        let r = std::env::temp_dir().join(format!("vox-doc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("docs/src")).unwrap();
        r
    }

    fn args(extra: &[&str]) -> Vec<String> {
        ["vox-doc-pipeline", "--render-diagrams"]
            .into_iter()
            .chain(extra.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn one_fence_renders_into_the_manifest() {
        let r = root("render-one");
        std::fs::write(
            r.join("docs/src/a.md"),
            format!("# A\n\n```vox-diagram\n{GRAPH}```\n"),
        )
        .unwrap();
        assert_eq!(lint_in(&r, &args(&[])), 0);
        let text =
            std::fs::read_to_string(r.join("docs-astro/.vox-diagram/manifest.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["schema_version"], 1);
        let d = v["diagrams"].as_array().unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0]["source"], vox_diagram::fence::normalize(GRAPH));
        let svg = d[0]["svg"].as_str().unwrap();
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("class=\"vd-diagram\""), "{svg}");
        let _ = std::fs::remove_dir_all(&r);
    }
}
