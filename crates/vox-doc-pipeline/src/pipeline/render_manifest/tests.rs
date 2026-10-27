use super::{mounts, render_diagrams};
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
    let text = std::fs::read_to_string(r.join("docs-astro/.vox-diagram/manifest.json")).unwrap();
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
    let text = std::fs::read_to_string(r.join("docs-astro/.vox-diagram/manifest.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn identical_fences_in_two_files_share_one_entry() {
    let r = root("dedupe");
    let crlf = format!("```vox-diagram\n{}   \n```\n", fence("a").trim_end()).replace('\n', "\r\n");
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
    let rendered = render_diagrams(&r, &[]);
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
    let rendered = render_diagrams(&r, &[]);
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
    let rendered = render_diagrams(&r, &[]);
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
    assert!(render_diagrams(&r, &[]).errors.is_empty());
    assert!(manifest(&r)["diagrams"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn mounted_repo_docs_are_rendered_and_attributed_to_their_repo_path() {
    let r = root("mounts");
    put(
        &r,
        "AGENTS.md",
        &format!("# Agents\n\n```vox-diagram\n{}```\n", fence("m")),
    );
    put(
        &r,
        "crates/x/README.md",
        "# X\n\n```vox-diagram\ngraph {\n    a -> ghost\n}\n```\n",
    );
    let a = args(&["--mount", "AGENTS.md", "--mount", "crates/x/README.md"]);
    assert_eq!(lint_in(&r, &a), 1);
    let v = manifest(&r);
    let sources: Vec<&str> = v["diagrams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["source"].as_str().unwrap())
        .collect();
    assert_eq!(sources, vec![vox_diagram::fence::normalize(&fence("m"))]);
    let errors = render_diagrams(&r, &mounts(&a)).errors;
    assert!(
        !errors.is_empty() && errors.iter().all(|e| e.starts_with("crates/x/README.md:")),
        "{errors:?}"
    );
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn a_mount_outside_the_repo_or_not_markdown_is_an_error() {
    let r = root("badmount");
    put(&r, "notes.txt", "x");
    // A real Markdown file just outside the root, so only the path rule can refuse it.
    let outside = format!("vox-doc-outside-{}.md", std::process::id());
    let fenced = format!("```vox-diagram\n{}```\n", fence("o"));
    std::fs::write(r.parent().unwrap().join(&outside), fenced).unwrap();
    let up = format!("../{outside}");
    for bad in [up.as_str(), "/etc/x.md", "notes.txt", "missing.md"] {
        let rendered = render_diagrams(&r, &mounts(&args(&["--mount", bad])));
        assert!(
            rendered.errors.len() == 1 && rendered.errors[0].contains("diagram-mount"),
            "{bad}: {:?}",
            rendered.errors
        );
        assert!(rendered.diagrams.is_empty(), "{bad}");
    }
    let _ = std::fs::remove_file(r.parent().unwrap().join(&outside));
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
