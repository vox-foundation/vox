//! Selection rules for which `.vox` files feed the vox-lang corpus.
//!
//! SSOT config: `mens/config/vox-source-pool.yaml` (roots, exclude globs, heldout bench).
//! The compile gate is applied by the caller (`generate::run_extract`) because it needs
//! the async frontend; everything decidable from path + text lives here.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::commands::mens::eval_gate::leakage::{
    BenchTask, leaked_bench_task_contained, load_bench_answers,
};

/// Default SSOT path, relative to the repo root.
pub(crate) const DEFAULT_CONFIG: &str = "mens/config/vox-source-pool.yaml";

/// Directory names never descended into when walking for candidates.
const PRUNE_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "archive",
    ".git",
    ".jj",
    ".claude",
    ".worktrees",
];

#[derive(Debug, serde::Deserialize)]
pub(crate) struct PoolConfig {
    #[serde(default)]
    heldout_bench: Option<PathBuf>,
    #[serde(default)]
    roots: Vec<Root>,
    #[serde(default)]
    exclude: Vec<Exclude>,
}

#[derive(Debug, serde::Deserialize)]
struct Root {
    path: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
struct Exclude {
    glob: String,
}

/// Compiled selection rules.
pub(crate) struct Selector {
    roots: Vec<PathBuf>,
    excludes: Vec<glob::Pattern>,
    bench: Vec<BenchTask>,
}

impl Selector {
    /// Load `config` (paths inside it are relative to the cwd / repo root).
    /// `roots_override` replaces the configured roots (the `corpus extract <dir>` form).
    pub(crate) fn load(config: &Path, roots_override: Option<&Path>) -> Result<Self> {
        let cfg: PoolConfig = if config.is_file() {
            let text = vox_bounded_fs::read_utf8_path_capped(config)?;
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", config.display()))?
        } else if roots_override.is_some() {
            PoolConfig {
                heldout_bench: None,
                roots: vec![],
                exclude: vec![],
            }
        } else {
            anyhow::bail!("source pool config not found: {}", config.display());
        };
        let bench = match &cfg.heldout_bench {
            Some(p) if p.is_file() => load_bench_answers(p)?,
            _ => vec![],
        };
        Self::from_config(cfg, roots_override, bench)
    }

    fn from_config(
        cfg: PoolConfig,
        roots_override: Option<&Path>,
        bench: Vec<BenchTask>,
    ) -> Result<Self> {
        let roots = match roots_override {
            Some(r) => vec![normalize(r)],
            None => cfg.roots.iter().map(|r| normalize(&r.path)).collect(),
        };
        let excludes = cfg
            .exclude
            .iter()
            .map(|e| glob::Pattern::new(&e.glob).with_context(|| format!("bad glob {}", e.glob)))
            .collect::<Result<_>>()?;
        Ok(Self {
            roots,
            excludes,
            bench,
        })
    }

    /// Why `path` (repo-relative) with `content` must not be used, or `None` if it is a
    /// candidate (still subject to the compile gate).
    pub(crate) fn skip_reason(&self, path: &Path, content: &str) -> Option<String> {
        let path = normalize(path);
        if !self
            .roots
            .iter()
            .any(|r| r.as_os_str() == "." || path.starts_with(r))
        {
            return Some("not_in_roots".into());
        }
        let opts = glob::MatchOptions {
            require_literal_separator: true,
            ..Default::default()
        };
        if let Some(p) = self
            .excludes
            .iter()
            .find(|p| p.matches_path_with(&path, opts))
        {
            return Some(format!("excluded_glob:{}", p.as_str()));
        }
        if !vox_corpus::corpus::extract_vox::is_eligible_for_training(content) {
            return Some("marked_training_eligible_false".into());
        }
        if is_generated(content) {
            return Some("generated_header".into());
        }
        if let Some((id, score)) = leaked_bench_task_contained(&self.bench, content) {
            return Some(format!("heldout_bench_overlap:{id}:{score:.2}"));
        }
        None
    }
}

/// Same rule as vox-code-audit's scanner: an `@generated` marker in the banner.
fn is_generated(content: &str) -> bool {
    content.lines().take(10).any(|l| l.contains("@generated"))
}

/// `// training_eligible:` marker value as written: "true", "false" or "absent".
pub(crate) fn marker(content: &str) -> &'static str {
    if !vox_corpus::corpus::extract_vox::is_eligible_for_training(content) {
        "false"
    } else if content.contains("training_eligible: true")
        || content.contains("training_eligible:true")
    {
        "true"
    } else {
        "absent"
    }
}

/// Strip a leading `./` so globs and root prefixes compare cleanly.
fn normalize(p: &Path) -> PathBuf {
    p.strip_prefix(".")
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| p.to_path_buf())
}

/// Every `.vox` file under `dir`, sorted, skipping [`PRUNE_DIRS`].
pub(crate) fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !(e.file_type().is_dir() && PRUNE_DIRS.iter().any(|d| e.file_name() == *d))
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "vox"))
        .map(|e| normalize(e.path()))
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selector(bench: &[(&str, &str)]) -> Selector {
        let cfg: PoolConfig = serde_yaml::from_str(
            "roots:\n  - path: examples\n  - path: scripts\nexclude:\n  - glob: \"examples/forbidden/**\"\n  - glob: \"examples/one.vox\"\n",
        )
        .unwrap();
        let bench = bench
            .iter()
            .map(|(id, a)| BenchTask {
                id: id.to_string(),
                answer: a.to_string(),
            })
            .collect();
        Selector::from_config(cfg, None, bench).unwrap()
    }

    const OK: &str = "fn double(x: int) to int {\n    return x * 2\n}\n";

    #[test]
    fn clean_file_under_root_is_candidate() {
        let s = selector(&[]);
        assert_eq!(s.skip_reason(Path::new("examples/golden/a.vox"), OK), None);
        assert_eq!(s.skip_reason(Path::new("./scripts/b.vox"), OK), None);
    }

    #[test]
    fn outside_roots_is_skipped() {
        let s = selector(&[]);
        assert_eq!(
            s.skip_reason(
                Path::new("contracts/eval/humaneval-vox/x/reference.vox"),
                OK
            )
            .as_deref(),
            Some("not_in_roots")
        );
        // Prefix must be a whole path component: "examples2/" is not under "examples".
        assert!(s.skip_reason(Path::new("examples2/a.vox"), OK).is_some());
    }

    #[test]
    fn exclusion_globs_match_nested_and_exact_paths() {
        let s = selector(&[]);
        let r = s
            .skip_reason(Path::new("examples/forbidden/deep/bad.vox"), OK)
            .unwrap();
        assert_eq!(r, "excluded_glob:examples/forbidden/**");
        assert!(
            s.skip_reason(Path::new("examples/one.vox"), OK)
                .unwrap()
                .starts_with("excluded_glob")
        );
        assert_eq!(s.skip_reason(Path::new("examples/sub/one.vox"), OK), None);
    }

    #[test]
    fn training_eligible_false_marker_is_skipped() {
        let s = selector(&[]);
        let src = format!("// training_eligible: false\n{OK}");
        assert_eq!(
            s.skip_reason(Path::new("examples/a.vox"), &src).as_deref(),
            Some("marked_training_eligible_false")
        );
        assert_eq!(marker(&src), "false");
        assert_eq!(marker(&format!("// training_eligible: true\n{OK}")), "true");
        assert_eq!(marker(OK), "absent");
    }

    #[test]
    fn generated_header_is_skipped() {
        let s = selector(&[]);
        let src = format!("// @generated-hash deadbeef\n{OK}");
        assert_eq!(
            s.skip_reason(Path::new("scripts/g.vox"), &src).as_deref(),
            Some("generated_header")
        );
        // A marker deep in the body is not a header.
        let body = format!("{}// @generated\n", "fn f() {}\n".repeat(20));
        assert_eq!(s.skip_reason(Path::new("scripts/g.vox"), &body), None);
    }

    #[test]
    fn heldout_bench_overlap_is_skipped() {
        let answer = "fn add(a: int, b: int) to int {\n    let sum = a + b\n    return sum\n}";
        let s = selector(&[("fn_add", answer)]);
        let src = format!("// helper file\n{OK}\n{answer}\n");
        let r = s.skip_reason(Path::new("scripts/leak.vox"), &src).unwrap();
        assert!(r.starts_with("heldout_bench_overlap:fn_add:"), "{r}");
        assert_eq!(s.skip_reason(Path::new("scripts/ok.vox"), OK), None);
    }

    #[test]
    fn walk_prunes_build_and_vendor_dirs() {
        let d = tempfile::tempdir().unwrap();
        for p in [
            "a.vox",
            "sub/b.vox",
            "target/c.vox",
            "x/node_modules/d.vox",
            "e.rs",
        ] {
            let f = d.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, OK).unwrap();
        }
        let got: Vec<_> = walk(d.path())
            .into_iter()
            .map(|p| p.strip_prefix(d.path()).unwrap().to_path_buf())
            .collect();
        assert_eq!(
            got,
            vec![PathBuf::from("a.vox"), PathBuf::from("sub/b.vox")]
        );
    }

    #[test]
    fn repo_config_parses_and_every_root_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join(DEFAULT_CONFIG)).unwrap();
        let cfg: PoolConfig = serde_yaml::from_str(&text).unwrap();
        assert!(!cfg.roots.is_empty());
        for r in &cfg.roots {
            assert!(
                root.join(&r.path).is_dir(),
                "missing root {}",
                r.path.display()
            );
        }
        for e in &cfg.exclude {
            glob::Pattern::new(&e.glob).unwrap();
        }
        assert!(root.join(cfg.heldout_bench.unwrap()).is_file());
    }
}
