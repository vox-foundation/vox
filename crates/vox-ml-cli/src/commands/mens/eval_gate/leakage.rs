//! B7.0 — Leakage assertion: verifies no tool appears in both training and eval sets.
//!
//! Must run before any gate result is trusted.
//!
//! Task D3 (2026-09-12-mens-end-to-end-completion): `assert_no_leakage`
//! (tool-name 3-gram Jaccard, below) had zero non-test callers and — even if
//! wired up as-is — compares tool *names*, which is useless for a code
//! corpus. Rather than repurpose it, this module gained a sibling,
//! [`assert_no_text_leakage`], operating on normalized word n-grams of bench
//! **answers** vs. corpus **completions**; that sibling is what
//! `check_run.rs` wires in as a hard precondition. `assert_no_leakage` is
//! kept (dead outside its own tests) for the BFCL / tool-selection spoke,
//! where tool-name leakage is exactly the right check — wiring that spoke's
//! own hard precondition is out of scope for D3's vox-lang bench fix.
#![allow(dead_code)] // assert_no_leakage (tool-name path): not this task's spoke to wire

use anyhow::{Result, bail};
use std::collections::HashSet;
use std::path::Path;

/// Minimal split manifest — mirrors what B1.4 eval_split.rs writes.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SplitManifest {
    /// Tool names (or identifiers) present in the training partition.
    pub train_tools: Vec<String>,
    /// Tool names present in the eval partition.
    pub eval_tools: Vec<String>,
    /// Random seed used for the split.
    #[serde(default)]
    pub seed: u64,
    /// Fraction held out for eval (0.0–1.0).
    #[serde(default)]
    pub eval_frac: f64,
}

/// Load a `SplitManifest` from `split_manifest.json` in `corpus_dir`, or from
/// an explicit path if provided.
pub fn load_split_manifest(path: &Path) -> Result<SplitManifest> {
    let content = vox_bounded_fs::read_utf8_path_capped(path)?;
    let manifest: SplitManifest = serde_json::from_str(&content)?;
    Ok(manifest)
}

// ---------------------------------------------------------------------------
// 3-gram fingerprint helpers (inline — do NOT add vox-similarity dep here)
// ---------------------------------------------------------------------------

/// Normalise a tool name to a canonical form for near-dup comparison.
fn normalise(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Produce the set of character-3-grams from a string.
fn trigrams(s: &str) -> HashSet<[char; 3]> {
    let chars: Vec<char> = s.chars().collect();
    chars.windows(3).map(|w| [w[0], w[1], w[2]]).collect()
}

/// Jaccard similarity between two sets of 3-grams (0.0–1.0).
fn jaccard(a: &HashSet<[char; 3]>, b: &HashSet<[char; 3]>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 1.0 } else { inter / union }
}

/// Near-dup threshold: two tool names with Jaccard ≥ this are considered duplicates.
const NEAR_DUP_THRESHOLD: f64 = 0.75;

/// Assert that no tool name (exact or near-duplicate) appears in both the
/// training and eval partitions of `manifest`.
///
/// Also reads corpus rows from `corpus_dir` (JSONL files `*.jsonl`) if present
/// and cross-checks tool names from rows against the split.
///
/// Returns `Ok(())` if clean, `Err(...)` describing leakage if not.
pub fn assert_no_leakage(corpus_dir: &Path, manifest: &SplitManifest) -> Result<()> {
    // Step 1: exact intersection of declared tool lists
    let train_set: HashSet<String> = manifest.train_tools.iter().cloned().collect();
    let eval_set: HashSet<String> = manifest.eval_tools.iter().cloned().collect();

    let exact_leaks: Vec<String> = train_set.intersection(&eval_set).cloned().collect();
    if !exact_leaks.is_empty() {
        bail!(
            "leakage detected: {} tool(s) appear in both train and eval splits: {:?}",
            exact_leaks.len(),
            exact_leaks
        );
    }

    // Step 2: near-dup check via 3-gram Jaccard
    let train_grams: Vec<(String, HashSet<[char; 3]>)> = manifest
        .train_tools
        .iter()
        .map(|t| {
            let n = normalise(t);
            let g = trigrams(&n);
            (t.clone(), g)
        })
        .collect();

    let eval_grams: Vec<(String, HashSet<[char; 3]>)> = manifest
        .eval_tools
        .iter()
        .map(|t| {
            let n = normalise(t);
            let g = trigrams(&n);
            (t.clone(), g)
        })
        .collect();

    let mut near_dup_leaks: Vec<(String, String, f64)> = Vec::new();
    for (et, eg) in &eval_grams {
        for (tt, tg) in &train_grams {
            let sim = jaccard(eg, tg);
            if sim >= NEAR_DUP_THRESHOLD && et != tt {
                near_dup_leaks.push((et.clone(), tt.clone(), sim));
            }
        }
    }
    if !near_dup_leaks.is_empty() {
        let details: Vec<String> = near_dup_leaks
            .iter()
            .map(|(e, t, sim)| format!("eval='{}' ~ train='{}' (sim={:.2})", e, t, sim))
            .collect();
        bail!(
            "near-duplicate leakage detected ({} pair(s)):\n{}",
            near_dup_leaks.len(),
            details.join("\n")
        );
    }

    // Step 3: cross-check corpus JSONL rows (if corpus_dir contains JSONL).
    // Row-partition-aware: a tool whose ROWS appear in both the train and eval
    // partition files is real row-level leakage.
    if corpus_dir.exists() {
        let corpus_leaks = check_corpus_rows(corpus_dir)?;
        if !corpus_leaks.is_empty() {
            bail!(
                "corpus row leakage: tool(s) have rows in both the train and eval partition files: {:?}",
                corpus_leaks
            );
        }
    }

    Ok(())
}

/// Classify a JSONL filename into a corpus partition by naming convention.
/// Returns `Some(true)` for a train-partition file, `Some(false)` for an
/// eval/validation/test-partition file, and `None` when the filename carries no
/// partition marker (the row check abstains for such files — no false protection).
fn partition_of(file_stem: &str) -> Option<bool> {
    let s = file_stem.to_ascii_lowercase();
    let is_eval = s.contains("eval")
        || s.contains("valid")
        || s.contains("test")
        || s.contains("heldout")
        || s.contains("held_out");
    let is_train = s.contains("train");
    match (is_train, is_eval) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        // Neither marker, or BOTH markers (ambiguous) → cannot attribute a partition.
        _ => None,
    }
}

/// Scan JSONL files in `corpus_dir` for rows with a `tool` (or `tool_name`) field,
/// bucketing each row into the train or eval partition by the **filename** convention
/// (see [`partition_of`]), and return any tool whose ROWS appear in BOTH partitions.
///
/// This is the row-level leakage signal: the split is by tool identity, so a tool is
/// supposed to live entirely in one partition. If its rows show up under both a
/// `*train*.jsonl` and a `*eval*.jsonl` file, the corpus contradicts the split and the
/// eval set is contaminated.
///
/// Files without a partition marker in their name are skipped (we cannot attribute a
/// partition, so abstaining avoids both false protection and false positives). The
/// previous implementation bucketed by manifest membership, which made this check dead
/// (a manifest tool lands in at most one set, so the intersection was always empty).
fn check_corpus_rows(corpus_dir: &Path) -> Result<Vec<String>> {
    use std::io::{BufRead, BufReader};

    let mut seen_train: HashSet<String> = HashSet::new();
    let mut seen_eval: HashSet<String> = HashSet::new();

    let rd = std::fs::read_dir(corpus_dir)?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let is_train = match partition_of(stem) {
            Some(p) => p,
            None => continue, // unattributable file → abstain
        };

        let file = std::fs::File::open(&path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let tool_name = v
                .get("tool")
                .or_else(|| v.get("tool_name"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            if let Some(tool) = tool_name {
                if is_train {
                    seen_train.insert(tool);
                } else {
                    seen_eval.insert(tool);
                }
            }
        }
    }

    let mut leaks: Vec<String> = seen_train.intersection(&seen_eval).cloned().collect();
    leaks.sort();
    Ok(leaks)
}

// ---------------------------------------------------------------------------
// Text n-gram leakage: bench ANSWERS vs. corpus COMPLETIONS (Task D3, part 3)
// ---------------------------------------------------------------------------

/// One held-out bench task with its reference answer text, as read from
/// `mens/data/heldout_bench/manifest.json`.
#[derive(Debug, Clone)]
pub struct BenchTask {
    pub id: String,
    pub answer: String,
}

/// Word n-gram size for text leakage comparisons. Larger than the 3-*char*
/// grams used for tool-name near-dup matching above — those compare short
/// identifiers, this compares multi-line code bodies, so we n-gram over
/// whitespace-delimited tokens instead of characters.
const TEXT_NGRAM_SIZE: usize = 8;

/// Jaccard similarity at/above this threshold between a bench answer's
/// n-grams and a single corpus completion's n-grams is treated as leakage
/// (the completion is a verbatim or near-verbatim copy of the answer).
const TEXT_LEAK_THRESHOLD: f64 = 0.5;

/// Lowercase + collapse to whitespace-delimited tokens (drop punctuation-only
/// noise so formatting differences don't defeat the comparison).
fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Contiguous word n-grams (joined by a separator byte that cannot occur in a
/// normalized token) from a token stream. Bench answers are often short
/// (a handful of lines), so a token stream shorter than `n` falls back to a
/// single gram of the whole sequence rather than an empty set — that still
/// makes two short-and-identical answers compare as 100% overlap, while two
/// short-and-different answers compare as 0%, without a special-cased
/// "too short to compare" abstention hiding real leakage of tiny functions.
fn word_ngrams(tokens: &[String], n: usize) -> HashSet<String> {
    if tokens.is_empty() {
        return HashSet::new();
    }
    if tokens.len() < n {
        let mut set = HashSet::new();
        set.insert(tokens.join("\u{1}"));
        return set;
    }
    tokens.windows(n).map(|w| w.join("\u{1}")).collect()
}

/// Generic Jaccard similarity over two sets of n-grams (0.0–1.0). Empty vs.
/// empty is defined as "cannot compare" (0.0), unlike the tool-name
/// [`jaccard`] above — an empty answer or empty completion must never read as
/// "100% similar" and trip the leakage gate.
fn jaccard_ngrams(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

/// Load `{id, answer}` pairs from a `heldout_bench/manifest.json`-shaped file
/// (schema `vox_mens_bench_manifest_v1`: a top-level `benchmarks` array of
/// objects with `id` and `answer` string fields). Tasks without an `answer`
/// field are skipped (nothing to compare) rather than erroring, so older
/// manifests without answers don't hard-fail the gate.
pub fn load_bench_answers(bench_path: &Path) -> Result<Vec<BenchTask>> {
    load_bench_texts(bench_path, "answer")
}

/// Like [`load_bench_answers`] but reads any string `field` of each bench task
/// (e.g. `"description"`) into [`BenchTask::answer`]. Tasks lacking the field
/// are skipped.
pub fn load_bench_texts(bench_path: &Path, field: &str) -> Result<Vec<BenchTask>> {
    let content = vox_bounded_fs::read_utf8_path_capped(bench_path)?;
    let v: serde_json::Value = serde_json::from_str(&content)?;
    let benchmarks = v
        .get("benchmarks")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();
    let tasks = benchmarks
        .iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            let answer = item.get(field)?.as_str()?.to_string();
            Some(BenchTask { id, answer })
        })
        .collect();
    Ok(tasks)
}

/// Overlap of `other` against one bench text, using the adaptive gram size
/// rule of [`assert_no_text_leakage`] (n = min(8, bench-text tokens)).
fn bench_overlap(bench_tokens: &[String], other_tokens: &[String]) -> f64 {
    let n = TEXT_NGRAM_SIZE.min(bench_tokens.len());
    jaccard_ngrams(&word_ngrams(bench_tokens, n), &word_ngrams(other_tokens, n))
}

/// Per-row leakage check for corpus *producers* (synthesis stages): returns
/// the first bench task whose text overlaps `text` at or above the leakage
/// threshold, with its overlap score. `None` means the row is safe to emit.
pub fn leaked_bench_task<'a>(text: &str, bench: &'a [BenchTask]) -> Option<(&'a str, f64)> {
    let toks = tokenize(text);
    bench.iter().find_map(|task| {
        let task_tokens = tokenize(&task.answer);
        if task_tokens.is_empty() {
            return None;
        }
        let sim = bench_overlap(&task_tokens, &toks);
        (sim >= TEXT_LEAK_THRESHOLD).then_some((task.id.as_str(), sim))
    })
}

/// Scan every `*.jsonl` file directly under each of `corpus_dirs` for rows
/// carrying a text completion, under any of the field names this workspace's
/// mix pipeline uses for the "model output" side of a training row
/// (`response`, `completion`, `vox_code`). Missing directories are skipped
/// (optional corpus roots — e.g. `target/dogfood` doesn't exist before the
/// first corpus build), not an error.
fn load_corpus_completions(corpus_dirs: &[&Path]) -> Result<Vec<String>> {
    use std::io::{BufRead, BufReader};

    let mut completions = Vec::new();
    for dir in corpus_dirs {
        if !dir.exists() {
            continue;
        }
        let rd = std::fs::read_dir(dir)?;
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let file = std::fs::File::open(&path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.is_empty() {
                    continue;
                }
                let v: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let text = v
                    .get("response")
                    .or_else(|| v.get("completion"))
                    .or_else(|| v.get("vox_code"))
                    .and_then(|t| t.as_str());
                if let Some(t) = text {
                    completions.push(t.to_string());
                }
            }
        }
    }
    Ok(completions)
}

/// Assert that no bench task's `answer` text is verbatim (or near-verbatim,
/// by normalized word-8-gram Jaccard overlap) present in any corpus
/// completion under `corpus_dirs`.
///
/// This is the fix for the class of leakage [`assert_no_leakage`] cannot see:
/// a code corpus doesn't leak by *tool name*, it leaks when a benchmark's
/// reference solution (or a close paraphrase of it) is itself a training
/// example — at which point pass@1 measures memorization, not generalization.
///
/// Returns `Ok(())` when clean, `Err(...)` naming every leaked task id and
/// its overlap score otherwise.
pub fn assert_no_text_leakage(bench_path: &Path, corpus_dirs: &[&Path]) -> Result<()> {
    let tasks = load_bench_answers(bench_path)?;
    let completions = load_corpus_completions(corpus_dirs)?;
    if completions.is_empty() {
        // No corpus rows found (e.g. corpus not built yet) — nothing to leak
        // against. Mirrors `assert_no_leakage`'s `corpus_dir.exists()` guard.
        return Ok(());
    }
    let completion_tokens: Vec<Vec<String>> = completions.iter().map(|c| tokenize(c)).collect();

    let mut leaks: Vec<(String, f64)> = Vec::new();
    for task in &tasks {
        let task_tokens = tokenize(&task.answer);
        if task_tokens.is_empty() {
            continue; // empty answer — nothing to compare
        }
        // Gram size adapts to the answer's own length: a short function must
        // still be caught if it appears verbatim inside a longer completion,
        // so both sides are n-grammed at the SAME (possibly small) n rather
        // than a fixed n that would only ever match same-length text.
        let mut best = 0.0_f64;
        for ctoks in &completion_tokens {
            let sim = bench_overlap(&task_tokens, ctoks);
            if sim > best {
                best = sim;
            }
        }
        if best >= TEXT_LEAK_THRESHOLD {
            leaks.push((task.id.clone(), best));
        }
    }

    if !leaks.is_empty() {
        let details: Vec<String> = leaks
            .iter()
            .map(|(id, sim)| format!("{id} (overlap={sim:.2})"))
            .collect();
        bail!(
            "bench-answer / corpus-completion leakage detected ({} task(s)): {}",
            leaks.len(),
            details.join(", ")
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(train: &[&str], eval: &[&str]) -> SplitManifest {
        SplitManifest {
            train_tools: train.iter().map(|s| s.to_string()).collect(),
            eval_tools: eval.iter().map(|s| s.to_string()).collect(),
            seed: 42,
            eval_frac: 0.2,
        }
    }

    #[test]
    fn clean_corpus_passes() {
        let dir = tempfile::tempdir().unwrap();
        let m = manifest(
            &["read_file", "write_file"],
            &["search_code", "grep_pattern"],
        );
        assert_no_leakage(dir.path(), &m).expect("clean corpus should pass");
    }

    #[test]
    fn exact_leakage_fails() {
        let dir = tempfile::tempdir().unwrap();
        let m = manifest(
            &["read_file", "write_file", "shell_exec"],
            &["search_code", "write_file"], // write_file in both
        );
        let err = assert_no_leakage(dir.path(), &m).unwrap_err();
        assert!(
            err.to_string().contains("leakage detected"),
            "expected leakage error, got: {err}"
        );
    }

    #[test]
    fn near_dup_leakage_fails() {
        let dir = tempfile::tempdir().unwrap();
        // "read_file" and "read_files" are very similar (high Jaccard on 3-grams)
        let m = manifest(
            &["read_file_content"],
            &["read_file_contents"], // near-dup
        );
        let result = assert_no_leakage(dir.path(), &m);
        // These names have high trigram similarity — should fail
        assert!(
            result.is_err(),
            "near-duplicate tool names should trigger leakage check"
        );
    }

    #[test]
    fn distinct_names_pass_near_dup_check() {
        let dir = tempfile::tempdir().unwrap();
        // Completely different names
        let m = manifest(
            &["write_file", "shell_exec"],
            &["search_semantic", "list_branches"],
        );
        assert_no_leakage(dir.path(), &m).expect("distinct names should pass");
    }

    #[test]
    fn corpus_row_leakage_detected_across_partition_files() {
        // F5: REAL row-level leakage — the SAME tool has rows in BOTH the train and
        // eval partition files (filenames carry the partition). This is the failure the
        // old `check_corpus_rows` could never catch (it bucketed by manifest membership,
        // so a tool could only ever land in one bucket → intersection always empty).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("corpus.train.jsonl"),
            "{\"tool\":\"shared_tool\",\"output\":\"x\"}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("corpus.eval.jsonl"),
            "{\"tool\":\"shared_tool\",\"output\":\"y\"}\n",
        )
        .unwrap();
        // Manifest itself is clean (no exact/near-dup overlap) — the leakage is purely
        // at the row level across the two partition files.
        let m = manifest(&["alpha_tool"], &["beta_tool"]);
        let err = assert_no_leakage(dir.path(), &m).unwrap_err();
        assert!(
            err.to_string().contains("row leakage"),
            "row-level cross-partition leakage should be detected: {err}"
        );
    }

    #[test]
    fn corpus_rows_partitioned_cleanly_pass() {
        // A tool confined to a single partition file is NOT leakage.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("corpus.train.jsonl"),
            "{\"tool\":\"train_only\",\"output\":\"x\"}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("corpus.eval.jsonl"),
            "{\"tool\":\"eval_only\",\"output\":\"y\"}\n",
        )
        .unwrap();
        let m = manifest(&["train_only"], &["eval_only"]);
        assert_no_leakage(dir.path(), &m).expect("cleanly partitioned rows must pass");
    }

    #[test]
    fn corpus_without_partition_filenames_is_skipped() {
        // Files with no partition marker can't be attributed to a partition; the row
        // check abstains (no false protection, no false positive).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("corpus.jsonl"),
            "{\"tool\":\"ambiguous_tool\",\"output\":\"x\"}\n",
        )
        .unwrap();
        let m = manifest(&["alpha_tool"], &["beta_tool"]);
        assert_no_leakage(dir.path(), &m).expect("unpartitioned corpus must not false-fail");
    }

    #[test]
    fn split_manifest_round_trips() {
        let m = manifest(&["tool_a", "tool_b"], &["tool_c"]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("split_manifest.json");
        std::fs::write(&path, serde_json::to_string(&m).unwrap()).unwrap();
        let loaded = load_split_manifest(&path).unwrap();
        assert_eq!(loaded.train_tools, m.train_tools);
        assert_eq!(loaded.eval_tools, m.eval_tools);
        assert_eq!(loaded.seed, 42);
    }

    // -----------------------------------------------------------------
    // Text n-gram leakage (Task D3, step 1: the failing test written
    // before `assert_no_text_leakage` existed — kept as the permanent
    // regression test now that it passes).
    // -----------------------------------------------------------------

    fn write_bench(dir: &Path, tasks: &[(&str, &str)]) -> std::path::PathBuf {
        let benchmarks: Vec<serde_json::Value> = tasks
            .iter()
            .map(|(id, answer)| serde_json::json!({"id": id, "answer": answer}))
            .collect();
        let doc = serde_json::json!({
            "schema": "vox_mens_bench_manifest_v1",
            "benchmarks": benchmarks,
        });
        let path = dir.join("manifest.json");
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        path
    }

    fn write_corpus_jsonl(dir: &Path, name: &str, completions: &[&str]) {
        let body: String = completions
            .iter()
            .map(|c| serde_json::json!({"response": c}).to_string() + "\n")
            .collect();
        std::fs::write(dir.join(name), body).unwrap();
    }

    const FN_ADD_BODY: &str =
        "fn add(a: int, b: int) to int {\n    let sum = a + b\n    return sum\n}";

    #[test]
    fn text_leakage_detected_when_answer_appears_verbatim_in_corpus() {
        let bench_dir = tempfile::tempdir().unwrap();
        let bench_path = write_bench(bench_dir.path(), &[("fn_add", FN_ADD_BODY)]);

        let corpus_dir = tempfile::tempdir().unwrap();
        write_corpus_jsonl(corpus_dir.path(), "train.jsonl", &[FN_ADD_BODY]);

        let err = assert_no_text_leakage(&bench_path, &[corpus_dir.path()]).unwrap_err();
        assert!(
            err.to_string().contains("fn_add"),
            "expected leaked task id in error, got: {err}"
        );
    }

    #[test]
    fn text_leakage_clean_when_answer_absent_from_corpus() {
        let bench_dir = tempfile::tempdir().unwrap();
        let bench_path = write_bench(bench_dir.path(), &[("fn_add", FN_ADD_BODY)]);

        let corpus_dir = tempfile::tempdir().unwrap();
        write_corpus_jsonl(
            corpus_dir.path(),
            "train.jsonl",
            &["fn totally_unrelated_thing(x: str) to str {\n    return x.to_upper()\n}"],
        );

        assert_no_text_leakage(&bench_path, &[corpus_dir.path()])
            .expect("distinct completion must not trip the leakage gate");
    }

    #[test]
    fn text_leakage_ignores_missing_corpus_dir() {
        let bench_dir = tempfile::tempdir().unwrap();
        let bench_path = write_bench(bench_dir.path(), &[("fn_add", FN_ADD_BODY)]);
        let missing = std::path::Path::new("/does/not/exist/target/dogfood");
        assert_no_text_leakage(&bench_path, &[missing])
            .expect("a missing (not-yet-built) corpus dir must not fail the gate");
    }

    #[test]
    fn leaked_bench_task_flags_verbatim_text_and_passes_distinct_text() {
        let bench = vec![BenchTask {
            id: "fn_add".into(),
            answer: FN_ADD_BODY.into(),
        }];
        let hit = leaked_bench_task(&format!("// wrapper\n{FN_ADD_BODY}"), &bench);
        assert_eq!(hit.map(|(id, _)| id), Some("fn_add"));
        assert!(
            leaked_bench_task("fn mul(a: int, b: int) to int { return a * b }", &bench).is_none()
        );
    }

    #[test]
    fn load_bench_texts_reads_any_string_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        std::fs::write(
            &path,
            r#"{"benchmarks":[{"id":"a","description":"define fn a","answer":"fn a() {}"},{"id":"b"}]}"#,
        )
        .unwrap();
        let descs = load_bench_texts(&path, "description").unwrap();
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].answer, "define fn a");
    }

    /// Step 5/6 mutation-verification companion: reproduces the 4 tasks
    /// named leaked in docs/superpowers/plans/2026-09-12-mens-end-to-end-completion.md
    /// (§L-4) — `fn_add`, `fn_greet`, `query_list_items`, `component_button`
    /// — against a corpus containing their (former) verbatim answers, and
    /// confirms every one is caught.
    #[test]
    fn all_four_originally_leaked_tasks_are_caught() {
        let bench_dir = tempfile::tempdir().unwrap();
        let bench_path = write_bench(
            bench_dir.path(),
            &[
                ("fn_add", FN_ADD_BODY),
                (
                    "fn_greet",
                    "fn greet(name: str) to str {\n    return \"Hello, \" + name\n}",
                ),
                (
                    "query_list_items",
                    "query list_items() to list[Item] {\n    return db.query(\"select * from items\")\n}",
                ),
                (
                    "component_button",
                    "component Button(label: str) {\n    render button(label)\n}",
                ),
            ],
        );

        let corpus_dir = tempfile::tempdir().unwrap();
        write_corpus_jsonl(
            corpus_dir.path(),
            "train.jsonl",
            &[
                FN_ADD_BODY,
                "fn greet(name: str) to str {\n    return \"Hello, \" + name\n}",
                "query list_items() to list[Item] {\n    return db.query(\"select * from items\")\n}",
                "component Button(label: str) {\n    render button(label)\n}",
            ],
        );

        let err = assert_no_text_leakage(&bench_path, &[corpus_dir.path()]).unwrap_err();
        let msg = err.to_string();
        for id in ["fn_add", "fn_greet", "query_list_items", "component_button"] {
            assert!(msg.contains(id), "expected '{id}' in leakage error: {msg}");
        }
    }
}
