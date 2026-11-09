//! Held-out compile-and-test benchmark for the Rust spoke.
//!
//! Evaluates candidate solutions by compiling in a fresh isolated `std`-only crate
//! and running hidden tests with `cargo test --offline --quiet`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::humaneval_runner::verify::exec_cmd;

/// Single held-out benchmark item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustBenchItem {
    pub id: String,
    pub prompt: String,
    pub signature: String,
    pub hidden_tests: String,
    pub reference: String,
    pub training_eligible: bool,
}

/// Load and validate items from a jsonl file.
pub fn load_items(path: &Path) -> Result<Vec<RustBenchItem>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading rust bench items from {}", path.display()))?;
    let mut items = Vec::new();
    let mut ids = HashSet::new();

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item: RustBenchItem = serde_json::from_str(trimmed).with_context(|| {
            format!("parsing item at line {} in {}", line_no + 1, path.display())
        })?;

        if item.training_eligible {
            bail!(
                "item '{}' has training_eligible=true; held-out bench items must never be training eligible",
                item.id
            );
        }
        if !ids.insert(item.id.clone()) {
            bail!("duplicate item id '{}' in {}", item.id, path.display());
        }
        if !item.reference.contains(&item.signature) {
            bail!(
                "item '{}' reference solution does not contain required signature '{}'",
                item.id,
                item.signature
            );
        }
        items.push(item);
    }

    Ok(items)
}

/// Static scan for forbidden tokens/APIs in candidate code before execution.
pub fn scan_forbidden(src: &str) -> Option<String> {
    const FORBIDDEN: &[&str] = &[
        "std::process",
        "std::net",
        "std::fs",
        "std::env",
        "std::os",
        "Command",
        "unsafe",
        "extern crate",
        "include!",
        "include_str!",
        "include_bytes!",
        "env!",
        "#[path",
    ];

    for &tok in FORBIDDEN {
        if src.contains(tok) {
            return Some(tok.to_string());
        }
    }
    None
}

/// Outcome of evaluating a candidate solution on one item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustOutcome {
    Passed,
    CompileFailed(String),
    TestsFailed(String),
    Refused(String),
    TimedOut,
}

impl RustOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Options for compiling and executing an item.
#[derive(Debug, Clone)]
pub struct RunOpts {
    pub timeout: Duration,
    pub target_dir: PathBuf,
}

/// RAII temporary crate directory using stdlib so `vox-corpus` builds without `tempfile` in `[dependencies]`.
struct ScopedTempDir(PathBuf);

impl ScopedTempDir {
    fn new() -> Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vox_rust_bench_{}_{}_{}",
            std::process::id(),
            nanos,
            seq
        ));
        std::fs::create_dir_all(&dir).context("creating isolated tempdir for rust_bench")?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScopedTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Evaluate candidate code on a single benchmark item in an isolated crate.
pub fn run_item(item: &RustBenchItem, candidate: &str, opts: &RunOpts) -> Result<RustOutcome> {
    if let Some(tok) = scan_forbidden(candidate) {
        return Ok(RustOutcome::Refused(format!("forbidden API: {tok}")));
    }

    let temp_dir = ScopedTempDir::new()?;
    let crate_dir = temp_dir.path();

    // Isolated Cargo.toml with empty [workspace] table so it never joins repo workspace
    let cargo_toml = r#"[package]
name = "candidate_eval"
version = "0.1.0"
edition = "2024"

[workspace]
"#;
    std::fs::write(crate_dir.join("Cargo.toml"), cargo_toml)
        .context("writing Cargo.toml in tempdir")?;

    let src_dir = crate_dir.join("src");
    std::fs::create_dir_all(&src_dir).context("creating src dir in tempdir")?;

    let lib_rs = format!(
        "{candidate}\n\n#[cfg(test)]\nmod hidden_tests {{\n    use super::*;\n{}\n}}\n",
        item.hidden_tests
    );
    std::fs::write(src_dir.join("lib.rs"), lib_rs).context("writing lib.rs in tempdir")?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(crate_dir)
        .args(["test", "--offline", "--quiet"])
        .env_clear();

    // Pass through only minimal variables cargo and rustup need
    for key in ["PATH", "HOME", "RUSTUP_HOME", "CARGO_HOME", "TMPDIR"] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }
    cmd.env("CARGO_TARGET_DIR", &opts.target_dir);
    cmd.env("CARGO_NET_OFFLINE", "true");

    let out = exec_cmd(cmd, crate_dir, "rust_bench", opts.timeout)?;

    if out.detail.contains("timed out") {
        return Ok(RustOutcome::TimedOut);
    }

    if out.success {
        Ok(RustOutcome::Passed)
    } else {
        let err_file = crate_dir.join("rust_bench.err");
        let out_file = crate_dir.join("rust_bench.out");
        let stderr = std::fs::read_to_string(&err_file).unwrap_or_default();
        let stdout = std::fs::read_to_string(&out_file).unwrap_or_default();
        let combined = format!("{stderr}\n{stdout}");

        if combined.contains("error[") || combined.contains("could not compile") {
            Ok(RustOutcome::CompileFailed(out.detail))
        } else {
            Ok(RustOutcome::TestsFailed(out.detail))
        }
    }
}

/// Run an entire suite of benchmark items.
pub fn run_suite(
    items: &[RustBenchItem],
    candidate_for: &mut dyn FnMut(&RustBenchItem) -> String,
    opts: &RunOpts,
) -> Result<Vec<(String, RustOutcome)>> {
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let candidate = candidate_for(item);
        let outcome = run_item(item, &candidate, opts)?;
        results.push((item.id.clone(), outcome));
    }
    Ok(results)
}

pub mod controls {
    use super::*;

    pub fn oracle(_items: &[RustBenchItem], item: &RustBenchItem) -> String {
        item.reference.clone()
    }

    pub fn todo_stub(_items: &[RustBenchItem], item: &RustBenchItem) -> String {
        let sig = item.signature.trim();
        let sig = sig.strip_suffix('{').unwrap_or(sig).trim_end();
        let sig = sig.strip_suffix(';').unwrap_or(sig).trim_end();
        format!("{sig} {{\n    todo!()\n}}\n")
    }

    /// Negative control that emits an empty crate source so `cargo test` fails to compile.
    pub fn empty(_items: &[RustBenchItem], item: &RustBenchItem) -> String {
        format!("// negative control: empty candidate for {}\n", item.id)
    }

    pub fn shuffled_reference(items: &[RustBenchItem], item: &RustBenchItem) -> String {
        if items.is_empty() {
            return String::new();
        }
        let idx = items.iter().position(|it| it.id == item.id).unwrap_or(0);
        let next_idx = (idx + 1) % items.len();
        items[next_idx].reference.clone()
    }

    pub fn forbidden_api(_items: &[RustBenchItem], item: &RustBenchItem) -> String {
        format!(
            "{}\n\npub fn helper_forbidden() {{\n    std::process::exit(1);\n}}\n",
            item.reference
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<RustBenchItem> {
        vec![
            RustBenchItem {
                id: "rust_01_clamp".to_string(),
                prompt: "Clamp an integer.".to_string(),
                signature: "pub fn clamp_val(val: i64, min_val: i64, max_val: i64) -> i64".to_string(),
                hidden_tests: "#[test]\nfn test_clamp() { assert_eq!(clamp_val(5, 0, 10), 5); }\n".to_string(),
                reference: "pub fn clamp_val(val: i64, min_val: i64, max_val: i64) -> i64 {\n    val.clamp(min_val, max_val)\n}\n".to_string(),
                training_eligible: false,
            },
            RustBenchItem {
                id: "rust_02_vowels".to_string(),
                prompt: "Count vowels.".to_string(),
                signature: "pub fn count_vowels(s: &str) -> usize".to_string(),
                hidden_tests: "#[test]\nfn test_vowels() { assert_eq!(count_vowels(\"hello\"), 2); }\n".to_string(),
                reference: "pub fn count_vowels(s: &str) -> usize {\n    s.chars().filter(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')).count()\n}\n".to_string(),
                training_eligible: false,
            },
        ]
    }

    #[test]
    fn test_load_items_checks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.jsonl");

        // Valid items load fine
        let valid = sample_items();
        let content = valid
            .iter()
            .map(|it| serde_json::to_string(it).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, &content).unwrap();
        let loaded = load_items(&path).unwrap();
        assert_eq!(loaded.len(), 2);

        // training_eligible true is refused
        let bad = RustBenchItem {
            training_eligible: true,
            ..valid[0].clone()
        };
        std::fs::write(&path, serde_json::to_string(&bad).unwrap()).unwrap();
        let err = load_items(&path).unwrap_err().to_string();
        assert!(
            err.contains("training_eligible=true"),
            "unexpected error: {err}"
        );

        // duplicate id is refused
        let dups = format!(
            "{}\n{}",
            serde_json::to_string(&valid[0]).unwrap(),
            serde_json::to_string(&valid[0]).unwrap()
        );
        std::fs::write(&path, dups).unwrap();
        let err = load_items(&path).unwrap_err().to_string();
        assert!(err.contains("duplicate item id"), "unexpected error: {err}");

        // signature mismatch in reference is refused
        let bad_sig = RustBenchItem {
            signature: "pub fn other_fn() -> ()".to_string(),
            ..valid[0].clone()
        };
        std::fs::write(&path, serde_json::to_string(&bad_sig).unwrap()).unwrap();
        let err = load_items(&path).unwrap_err().to_string();
        assert!(
            err.contains("reference solution does not contain required signature"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_scan_forbidden_tokens() {
        assert!(scan_forbidden("pub fn ok() -> i32 { 42 }").is_none());
        assert_eq!(
            scan_forbidden("use std::process::Command;"),
            Some("std::process".to_string())
        );
        assert_eq!(
            scan_forbidden("use std::net::TcpStream;"),
            Some("std::net".to_string())
        );
        assert_eq!(
            scan_forbidden("use std::fs::File;"),
            Some("std::fs".to_string())
        );
        assert_eq!(
            scan_forbidden("let x = std::env::var(\"FOO\");"),
            Some("std::env".to_string())
        );
        assert_eq!(
            scan_forbidden("use std::os::unix::fs;"),
            Some("std::os".to_string())
        );
        assert_eq!(
            scan_forbidden("let c = Command::new(\"ls\");"),
            Some("Command".to_string())
        );
        assert_eq!(
            scan_forbidden("unsafe { *ptr = 1; }"),
            Some("unsafe".to_string())
        );
        assert_eq!(
            scan_forbidden("extern crate alloc;"),
            Some("extern crate".to_string())
        );
        assert_eq!(
            scan_forbidden("include!(\"foo.rs\");"),
            Some("include!".to_string())
        );
        assert_eq!(
            scan_forbidden("include_str!(\"foo.txt\");"),
            Some("include_str!".to_string())
        );
        assert_eq!(
            scan_forbidden("include_bytes!(\"foo.bin\");"),
            Some("include_bytes!".to_string())
        );
        assert_eq!(scan_forbidden("env!(\"HOME\");"), Some("env!".to_string()));
        assert_eq!(
            scan_forbidden("#[path = \"sub.rs\"] mod sub;"),
            Some("#[path".to_string())
        );
    }

    #[test]
    fn test_run_item_lifecycle() {
        let target_dir = tempfile::tempdir().unwrap();
        let opts = RunOpts {
            timeout: Duration::from_secs(30),
            target_dir: target_dir.path().to_path_buf(),
        };
        let items = sample_items();

        // 1. Reference passes
        let outcome = run_item(&items[0], &items[0].reference, &opts).unwrap();
        assert_eq!(outcome, RustOutcome::Passed);

        // 2. todo_stub fails tests
        let stub = controls::todo_stub(&items, &items[0]);
        let outcome = run_item(&items[0], &stub, &opts).unwrap();
        assert!(
            matches!(outcome, RustOutcome::TestsFailed(_)),
            "got {outcome:?}"
        );

        // 3. Syntax error fails compile
        let bad_code = "pub fn clamp_val(val: i64) { this is not rust }";
        let outcome = run_item(&items[0], bad_code, &opts).unwrap();
        assert!(
            matches!(outcome, RustOutcome::CompileFailed(_)),
            "got {outcome:?}"
        );

        // 4. Forbidden API is refused before compilation
        let forbidden = controls::forbidden_api(&items, &items[0]);
        let outcome = run_item(&items[0], &forbidden, &opts).unwrap();
        assert!(
            matches!(outcome, RustOutcome::Refused(_)),
            "got {outcome:?}"
        );

        // 5. Infinite loop times out
        let timeout_opts = RunOpts {
            timeout: Duration::from_secs(2),
            target_dir: target_dir.path().to_path_buf(),
        };
        let loop_code = "pub fn clamp_val(val: i64, _min: i64, _max: i64) -> i64 { loop {} }";
        let outcome = run_item(&items[0], loop_code, &timeout_opts).unwrap();
        assert_eq!(outcome, RustOutcome::TimedOut);
    }

    #[test]
    fn rust_bench_controls_quick() {
        let items = sample_items();
        let target_dir = tempfile::tempdir().unwrap();
        let opts = RunOpts {
            timeout: Duration::from_secs(30),
            target_dir: target_dir.path().to_path_buf(),
        };

        // Oracle passes 2/2
        for item in &items {
            let oracle = controls::oracle(&items, item);
            let outcome = run_item(item, &oracle, &opts).unwrap();
            assert!(outcome.is_pass(), "oracle failed on {}", item.id);
        }

        // Stub fails 2/2
        for item in &items {
            let stub = controls::todo_stub(&items, item);
            let outcome = run_item(item, &stub, &opts).unwrap();
            assert!(
                !outcome.is_pass(),
                "stub unexpectedly passed on {}",
                item.id
            );
        }

        // Forbidden is refused 2/2
        for item in &items {
            let forbidden = controls::forbidden_api(&items, item);
            let outcome = run_item(item, &forbidden, &opts).unwrap();
            assert!(
                matches!(outcome, RustOutcome::Refused(_)),
                "forbidden API was not refused on {}",
                item.id
            );
        }
    }

    #[test]
    fn test_load_real_heldout_items() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data/rust_bench/items.v1.jsonl");
        if !path.exists() {
            return;
        }
        let items = load_items(&path).expect("failed to load real held-out items");
        assert_eq!(items.len(), 12, "must have exactly 12 held-out items");
        for item in &items {
            assert!(!item.training_eligible);
            assert!(item.reference.contains(&item.signature));
        }
    }

    /// Full 60-run control matrix (12 items x 5 controls).
    /// Run with: `cargo test -p vox-corpus --lib rust_bench_control_matrix -- --ignored`
    /// Wall-clock: ~3-6 minutes.
    #[test]
    #[ignore]
    fn rust_bench_control_matrix() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data/rust_bench/items.v1.jsonl");
        let items = load_items(&path).expect("load items");
        assert_eq!(items.len(), 12);

        let target_dir = tempfile::tempdir().unwrap();
        let opts = RunOpts {
            timeout: Duration::from_secs(30),
            target_dir: target_dir.path().to_path_buf(),
        };

        // 1. Oracle: 12/12 Passed
        let mut passed = 0;
        for item in &items {
            let candidate = controls::oracle(&items, item);
            let res = run_item(item, &candidate, &opts).unwrap();
            if res.is_pass() {
                passed += 1;
            }
        }
        assert_eq!(passed, 12, "oracle score {passed}/12");

        // 2. Todo stub: 0/12 Passed
        let mut passed = 0;
        for item in &items {
            let candidate = controls::todo_stub(&items, item);
            let res = run_item(item, &candidate, &opts).unwrap();
            if res.is_pass() {
                passed += 1;
            }
        }
        assert_eq!(passed, 0, "todo_stub score {passed}/12");

        // 3. Empty: 0/12 Passed
        let mut passed = 0;
        for item in &items {
            let candidate = controls::empty(&items, item);
            let res = run_item(item, &candidate, &opts).unwrap();
            if res.is_pass() {
                passed += 1;
            }
        }
        assert_eq!(passed, 0, "empty score {passed}/12");

        // 4. Shuffled reference: 0/12 Passed
        let mut passed = 0;
        for item in &items {
            let candidate = controls::shuffled_reference(&items, item);
            let res = run_item(item, &candidate, &opts).unwrap();
            if res.is_pass() {
                passed += 1;
            }
        }
        assert_eq!(passed, 0, "shuffled_reference score {passed}/12");

        // 5. Forbidden API: 12/12 Refused
        let mut refused = 0;
        for item in &items {
            let candidate = controls::forbidden_api(&items, item);
            let res = run_item(item, &candidate, &opts).unwrap();
            if matches!(res, RustOutcome::Refused(_)) {
                refused += 1;
            }
        }
        assert_eq!(refused, 12, "forbidden_api refused count {refused}/12");
    }
}
