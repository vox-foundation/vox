//! `--render-diagrams`: pre-render every ```` ```vox-diagram ```` fence under `docs/src`, and in
//! each repo file passed with `--mount` (the out-of-tree pages docs-astro mounts under `repo/`),
//! into the gitignored `docs-astro/.vox-diagram/manifest.json` (VoxDiagram Phase 89, D-01/D-02).

use std::collections::BTreeSet;
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

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

/// Walk `root/docs/src` (`archive/` skipped, files in sorted order so output is deterministic),
/// then each mounted repo file in `mounts` (repo-relative `.md`/`.mdx` paths that docs-astro mounts
/// under `repo/`; anything else is a `diagram-mount` error). Every distinct normalized fence
/// source is rendered once; hard fence findings and render failures become
/// `{file}:{line}:{col}: {code}: {message}` lines, attributed to the first file holding the
/// source. Valid diagrams are returned even when other fences fail.
fn render_diagrams(root: &Path, mounts: &[String]) -> Rendered {
    let mut docs = Vec::new();
    gather_md_files(&root.join("docs/src"), &mut docs);
    docs.sort();
    let mut out = Rendered::default();
    let mut files: Vec<(String, PathBuf)> = docs
        .into_iter()
        .map(|f| {
            let rel = f
                .strip_prefix(root)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            (rel, f)
        })
        .filter(|(rel, _)| !rel.starts_with("docs/src/archive/"))
        .collect();
    let mut mounted: Vec<&String> = mounts.iter().collect();
    mounted.sort();
    mounted.dedup();
    for m in mounted {
        let p = Path::new(m);
        let markdown = matches!(p.extension().and_then(|e| e.to_str()), Some("md" | "mdx"));
        let inside = p.components().all(|c| matches!(c, Component::Normal(_)));
        if markdown && inside && root.join(p).is_file() {
            files.push((m.clone(), root.join(p)));
        } else {
            out.errors.push(format!(
                "{m}:1:1: diagram-mount: not a Markdown file inside the repository"
            ));
        }
    }
    let mut tried = BTreeSet::new();
    let mut ok = std::collections::BTreeMap::new();
    for (rel, f) in &files {
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

/// The value of every `--mount <path>` pair.
fn mounts(args: &[String]) -> Vec<String> {
    args.windows(2)
        .filter(|w| w[0] == "--mount")
        .map(|w| w[1].clone())
        .collect()
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

/// `--render-diagrams [--out <path>] [--mount <repo path>]...`: write the manifest
/// `{schema_version: 1, diagrams: [{source, svg}]}` (default `docs-astro/.vox-diagram/manifest.json`)
/// and return 1 when any fence failed. The manifest of the valid diagrams is always written so
/// `astro dev` keeps working (D-01, D-03).
pub(crate) fn run(root: &Path, args: &[String]) -> i32 {
    let out: PathBuf = root.join(
        args.windows(2)
            .find(|w| w[0] == "--out")
            .map_or(DEFAULT_OUT, |w| w[1].as_str()),
    );
    let rendered = render_diagrams(root, &mounts(args));
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
mod tests;
