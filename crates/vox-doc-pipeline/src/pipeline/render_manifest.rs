//! `--render-diagrams`: pre-render every ```` ```vox-diagram ```` fence under `docs/src` into the
//! gitignored `docs-astro/.vox-diagram/manifest.json` (VoxDiagram Phase 89, D-01/D-02).

use std::collections::BTreeSet;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

use serde::Serialize;
use vox_diagram::{Emit, ValidateOpts, fence, validate};

use super::diagrams::fence_issues;
use super::lint::gather_md_files;

const INFO: &str = "vox-diagram";
const DEFAULT_OUT: &str = "docs-astro/.vox-diagram/manifest.json";
const FIX: &str = "render-diagrams: fix the fences above, then re-run `pnpm diagrams` (or \
`cargo run -p vox-doc-pipeline -- --render-diagrams` from the repo root)";

#[derive(Serialize)]
struct Entry<'a> {
    source: &'a str,
    svg: &'a str,
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema_version: u32,
    diagrams: Vec<Entry<'a>>,
}

/// One walk of `docs/src`: manifest entries sorted by source, error lines, files read.
#[derive(Debug, Default)]
struct Rendered {
    diagrams: Vec<(String, String)>,
    errors: Vec<String>,
    files: usize,
}

/// Walk `root/docs/src` (`archive/` skipped, files in sorted order so output is deterministic).
/// Every distinct normalized fence source is rendered once; hard fence findings and render
/// failures become `{file}:{line}:{col}: {code}: {message}` lines, attributed to the first file
/// holding the source. Valid diagrams are returned even when other fences fail.
fn render_diagrams(root: &Path) -> Rendered {
    let mut files = Vec::new();
    gather_md_files(&root.join("docs/src"), &mut files);
    files.sort();
    let mut out = Rendered::default();
    let mut tried = BTreeSet::new();
    let mut ok = std::collections::BTreeMap::new();
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("docs/src/archive/") {
            continue;
        }
        out.files += 1;
        let text = match vox_bounded_fs::read_utf8_path_capped(f) {
            Ok(t) => t,
            Err(e) => {
                out.errors.push(format!("{rel}:1:1: diagram-read: {e}"));
                continue;
            }
        };
        out.errors.extend(
            fence_issues(&text)
                .iter()
                .filter(|i| i.is_hard())
                .map(|i| i.render(Path::new(&rel))),
        );
        for fence in fence::extract(&text, INFO) {
            let key = fence::normalize(&fence.body);
            if !tried.insert(key.clone()) {
                continue;
            }
            let v = validate(
                &key,
                ValidateOpts {
                    from_mermaid: false,
                    emit: Some(Emit::Svg),
                },
            );
            match (v.emitted, v.valid) {
                (Some(svg), _) => {
                    ok.insert(key, svg);
                }
                // An invalid fence is already reported by `fence_issues` above.
                (None, false) => {}
                (None, true) => out.errors.push(format!(
                    "{rel}:{}:1: diagram-render: {}",
                    fence.open_line,
                    v.emit_error.as_deref().unwrap_or("no SVG emitted")
                )),
            }
        }
    }
    out.diagrams = ok.into_iter().collect();
    out
}

/// Write `bytes` to `path` through a temp file in the same directory plus `rename`, so a
/// concurrent `astro build` never reads a half-written manifest (T-89-01).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    match std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => Err(match std::fs::remove_file(&tmp) {
            Ok(()) => e,
            Err(r) if r.kind() == ErrorKind::NotFound => e,
            Err(r) => Error::new(
                e.kind(),
                format!("{e}; temp file {} left behind: {r}", tmp.display()),
            ),
        }),
    }
}

/// `--render-diagrams [--out <path>]`: write the manifest
/// `{schema_version: 1, diagrams: [{source, svg}]}` (default `docs-astro/.vox-diagram/manifest.json`)
/// and return 1 when any fence failed. The manifest of the valid diagrams is always written so
/// `astro dev` keeps working (D-01, D-03).
pub(crate) fn run(root: &Path, args: &[String]) -> i32 {
    let out: PathBuf = root.join(
        args.windows(2)
            .find(|w| w[0] == "--out")
            .map_or(DEFAULT_OUT, |w| w[1].as_str()),
    );
    let rendered = render_diagrams(root);
    for e in &rendered.errors {
        eprintln!("{e}");
    }
    let manifest = Manifest {
        schema_version: 1,
        diagrams: rendered
            .diagrams
            .iter()
            .map(|(source, svg)| Entry { source, svg })
            .collect(),
    };
    let mut text = match serde_json::to_string(&manifest) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("render-diagrams: cannot serialize the manifest: {e}");
            return 1;
        }
    };
    text.push('\n');
    if let Err(e) = write_atomic(&out, text.as_bytes()) {
        eprintln!("render-diagrams: cannot write {}: {e}", out.display());
        return 1;
    }
    println!(
        "render-diagrams: {} diagram(s) from {} file(s) -> {}",
        rendered.diagrams.len(),
        rendered.files,
        out.display()
    );
    if rendered.errors.is_empty() {
        0
    } else {
        eprintln!("{FIX}");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::render_diagrams;
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

    fn fence(x: &str) -> String {
        format!("graph {{\n    node {x}: \"{x}\"\n    node y: \"Y\"\n    {x} -> y\n}}\n")
    }

    fn put(r: &std::path::Path, rel: &str, text: &str) {
        let p = r.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn manifest(r: &std::path::Path) -> serde_json::Value {
        let text =
            std::fs::read_to_string(r.join("docs-astro/.vox-diagram/manifest.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn identical_fences_in_two_files_share_one_entry() {
        let r = root("dedupe");
        let crlf =
            format!("```vox-diagram\n{}   \n```\n", fence("a").trim_end()).replace('\n', "\r\n");
        put(&r, "docs/src/a.md", &crlf);
        put(
            &r,
            "docs/src/b.mdx",
            &format!("# B\n\n```vox-diagram\n{}```\n", fence("a")),
        );
        assert_eq!(lint_in(&r, &args(&[])), 0);
        assert_eq!(manifest(&r)["diagrams"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn entries_are_sorted_and_runs_are_byte_identical() {
        let r = root("sorted");
        let md: String = ["z", "m", "a"]
            .iter()
            .map(|x| format!("```vox-diagram\n{}```\n\n", fence(x)))
            .collect();
        put(&r, "docs/src/a.md", &md);
        let out = r.join("docs-astro/.vox-diagram/manifest.json");
        assert_eq!(lint_in(&r, &args(&[])), 0);
        let first = std::fs::read(&out).unwrap();
        assert_eq!(lint_in(&r, &args(&[])), 0);
        assert_eq!(first, std::fs::read(&out).unwrap());
        let v = manifest(&r);
        let sources: Vec<&str> = v["diagrams"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["source"].as_str().unwrap())
            .collect();
        let mut sorted = sources.clone();
        sorted.sort_unstable();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources, sorted);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn invalid_fence_reports_file_line_col_and_still_writes_the_valid_ones() {
        let r = root("invalid");
        put(
            &r,
            "docs/src/a.md",
            &format!("# A\n\n```vox-diagram\n{}```\n", fence("a")),
        );
        put(
            &r,
            "docs/src/b.md",
            "# B\n\n```vox-diagram\ngraph {\n    node a: \"A\"\n    a -> ghost\n}\n```\n",
        );
        let rendered = render_diagrams(&r);
        assert_eq!(rendered.errors.len(), 1, "{:?}", rendered.errors);
        assert!(
            rendered.errors[0].starts_with("docs/src/b.md:6:")
                && rendered.errors[0].contains(": diagram/undeclared-node: "),
            "{:?}",
            rendered.errors
        );
        assert_eq!(lint_in(&r, &args(&[])), 1);
        assert_eq!(manifest(&r)["diagrams"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn near_miss_info_string_is_an_error() {
        let r = root("nearmiss");
        put(
            &r,
            "docs/src/n.md",
            "# N\n\n```vox-diagram title=\"x\"\ngraph {}\n```\n",
        );
        let rendered = render_diagrams(&r);
        assert_eq!(rendered.errors.len(), 1, "{:?}", rendered.errors);
        assert!(
            rendered.errors[0].starts_with("docs/src/n.md:3:1: diagram-fence-info: "),
            "{:?}",
            rendered.errors
        );
        assert_eq!(lint_in(&r, &args(&[])), 1);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn render_error_names_the_fence() {
        let r = root("rendererr");
        let nodes: String = (0..101)
            .map(|i| format!("    node n{i}: \"N{i}\"\n"))
            .collect();
        put(
            &r,
            "docs/src/c.md",
            &format!("# C\n\n```vox-diagram\ngraph {{\n{nodes}}}\n```\n"),
        );
        let rendered = render_diagrams(&r);
        assert_eq!(rendered.errors.len(), 1, "{:?}", rendered.errors);
        assert!(
            rendered.errors[0].starts_with("docs/src/c.md:3:1: diagram-render: "),
            "{:?}",
            rendered.errors
        );
        assert_eq!(lint_in(&r, &args(&[])), 1);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn archive_is_skipped() {
        let r = root("archive");
        put(
            &r,
            "docs/src/archive/old.md",
            "```vox-diagram\ngraph {\n    a -> ghost\n}\n```\n",
        );
        assert_eq!(lint_in(&r, &args(&[])), 0);
        assert!(render_diagrams(&r).errors.is_empty());
        assert!(manifest(&r)["diagrams"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn no_fences_writes_an_empty_manifest() {
        let r = root("empty");
        put(&r, "docs/src/a.md", "# A\n\nno diagrams\n");
        assert_eq!(lint_in(&r, &args(&[])), 0);
        let v = manifest(&r);
        assert_eq!(v["schema_version"], 1);
        assert!(v["diagrams"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn out_flag_is_honored_and_the_write_is_atomic() {
        let r = root("out");
        put(
            &r,
            "docs/src/a.md",
            &format!("```vox-diagram\n{}```\n", fence("a")),
        );
        assert_eq!(lint_in(&r, &args(&["--out", "custom/m.json"])), 0);
        assert!(r.join("custom/m.json").is_file());
        let siblings: Vec<_> = std::fs::read_dir(r.join("custom"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(siblings.len(), 1, "temp file left behind: {siblings:?}");
        assert!(!r.join("docs-astro/.vox-diagram/manifest.json").exists());
        let _ = std::fs::remove_dir_all(&r);
    }
}
