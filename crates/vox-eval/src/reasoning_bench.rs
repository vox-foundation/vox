//! Held-out reasoning slices (BENCH-02): three checked-in, fixed-size JSONL files
//! (`gsm8k_think`, `math500_think`, `vox_think`) and one validating loader.
//!
//! One JSON object per line. Required keys: `id`, `question`, `answer` (non-empty
//! strings). Extra keys (`source`, `kind`, `program`, ...) are ignored here.
//! There is no download path: the files live under `mens/data/<name>/reasoning.jsonl`.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::qwen_reasoning::ReasoningScore;

/// Number of items in every held-out slice.
pub const SLICE_SIZE: usize = 32;

/// Directory names under `mens/data/` that hold the three slices.
pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];

/// Largest slice file `load_slice` will read (1 MiB; real slices are far smaller).
pub const MAX_SLICE_BYTES: u64 = 1024 * 1024;

/// One held-out item.
#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningItem {
    /// Stable unique id within the slice.
    pub id: String,
    /// Prompt shown to the model.
    pub question: String,
    /// Gold final answer.
    pub answer: String,
}

/// Load and validate one slice file.
///
/// Refuses: files over [`MAX_SLICE_BYTES`], malformed JSON, rows with a missing or
/// empty `id`, blank (whitespace-only) `question` or `answer`, duplicate ids, and
/// any row count other than `expected_n`. Blank lines are skipped. Items come back
/// in file order.
pub fn load_slice(path: &Path, expected_n: usize) -> Result<Vec<ReasoningItem>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("opening {}: {e}", path.display()))?;
    let mut text = String::new();
    file.take(MAX_SLICE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    if text.len() as u64 > MAX_SLICE_BYTES {
        return Err(format!(
            "{} exceeds the {MAX_SLICE_BYTES}-byte slice size cap",
            path.display()
        ));
    }
    let mut items: Vec<ReasoningItem> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("{}:{lineno}: invalid JSON: {e}", path.display()))?;
        let field = |key: &str| {
            v.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let (id, question, answer) = (field("id"), field("question"), field("answer"));
        if id.trim().is_empty() {
            return Err(format!("{}:{lineno} needs a non-empty id", path.display()));
        }
        if question.trim().is_empty() || answer.trim().is_empty() {
            return Err(format!(
                "{}:{lineno} ({id}) needs non-empty question and answer",
                path.display()
            ));
        }
        if let Some(first) = seen.insert(id.clone(), lineno) {
            return Err(format!(
                "{}: duplicate id {id:?} at lines {first} and {lineno}",
                path.display()
            ));
        }
        items.push(ReasoningItem {
            id,
            question,
            answer,
        });
    }
    if items.len() != expected_n {
        return Err(format!(
            "{}: found {} items, expected {expected_n}",
            path.display(),
            items.len()
        ));
    }
    Ok(items)
}

/// Result of one reasoning-bench run: a measured score, or an explicit refusal to
/// report one. There is no numeric stand-in for "not scored".
#[derive(Debug, Clone, PartialEq)]
pub enum BenchOutcome {
    /// Every requested item was generated and graded.
    Scored(ReasoningScore),
    /// The run could not produce a full, error-free score.
    NotScored {
        /// Why no score exists (names the failing item or load step).
        reason: String,
        /// Number of items the run was supposed to score.
        n_requested: usize,
    },
}

impl BenchOutcome {
    /// Turn a graded batch into an outcome. `Scored` only when at least one item was
    /// requested and every requested item was graded (`score.n == n_requested`);
    /// the empty-batch tuple `{0.0, 0.0, 0}` and any partial count are `NotScored`.
    #[must_use]
    pub fn from_score(score: ReasoningScore, n_requested: usize) -> BenchOutcome {
        if n_requested >= 1 && score.n == n_requested {
            BenchOutcome::Scored(score)
        } else {
            BenchOutcome::NotScored {
                reason: format!("scored {} of {n_requested} items", score.n),
                n_requested,
            }
        }
    }

    /// JSON body for `reasoning_report.json`. `NotScored` carries `status`, `n`
    /// (always 0), `n_requested` and `reason` only: the `accuracy` and
    /// `think_initiation_rate` keys are absent, never null or zero.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            BenchOutcome::Scored(s) => serde_json::json!({
                "status": "scored",
                "n": s.n,
                "accuracy": s.accuracy,
                "think_initiation_rate": s.think_initiation_rate,
            }),
            BenchOutcome::NotScored {
                reason,
                n_requested,
            } => serde_json::json!({
                "status": "not_scored",
                "n": 0,
                "n_requested": n_requested,
                "reason": reason,
            }),
        }
    }
}

/// Schema tag of the before/after comparison report (BENCH-01). The report is an
/// operator record only: it is never a serve-gate or collateral-pass input.
pub const BEFORE_AFTER_SCHEMA: &str = "reasoning_before_after.v1";

/// The base a comparison was asked for, split from a `repo_id@revision` pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseLabel {
    /// The pin exactly as requested.
    pub requested: String,
    /// Part before the last `@` (or the whole input).
    pub repo_id: String,
    /// The revision, only when it is exactly 40 lowercase hex characters.
    pub revision: Option<String>,
}

impl BaseLabel {
    /// Split `pin` on its last `@`. Anything that is not a 40-hex commit sha
    /// (a branch, a tag, a short sha) leaves `revision` as `None`.
    #[must_use]
    pub fn from_pin(pin: &str) -> BaseLabel {
        let (repo_id, rev) = match pin.rsplit_once('@') {
            Some((repo, rev)) => (repo, Some(rev)),
            None => (pin, None),
        };
        let revision = rev
            .filter(|r| {
                r.len() == 40
                    && r.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .map(str::to_string);
        BaseLabel {
            requested: pin.to_string(),
            repo_id: repo_id.to_string(),
            revision,
        }
    }

    /// One path-safe file name per base: `<last repo segment>-<rev12>.json`, or
    /// `-unpinned.json` without a revision. Every character outside
    /// `[A-Za-z0-9._-]` and a leading `.` become `_`, so the name is one plain segment.
    #[must_use]
    pub fn report_file_name(&self) -> String {
        let last = self.repo_id.rsplit('/').next().unwrap_or_default();
        let mut name: String = last
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if name.starts_with('.') {
            name.replace_range(..1, "_");
        }
        match &self.revision {
            Some(rev) => format!("{name}-{}.json", &rev[..12]),
            None => format!("{name}-unpinned.json"),
        }
    }
}

/// Why a before/after comparison was not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsenceReason {
    /// The pinned snapshot directory is not in the Hugging Face cache.
    BaseSnapshotAbsent,
    /// The pinned snapshot exists but lacks config, tokenizer, or a weight shard.
    BaseSnapshotIncomplete,
    /// The requested base has no 40-hex revision, so no exact snapshot can be checked.
    PinUnverifiable,
    /// No adapter was given, or neither adapter file is present.
    AdapterAbsent,
    /// Only one of the two adapter files is present, or the manifest is unreadable.
    AdapterIncomplete,
    /// The adapter manifest `base_model` is not the pinned snapshot directory.
    AdapterBaseNotPinnedSnapshot,
}

impl AbsenceReason {
    /// Stable snake_case tag written to the record.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            AbsenceReason::BaseSnapshotAbsent => "base_snapshot_absent",
            AbsenceReason::BaseSnapshotIncomplete => "base_snapshot_incomplete",
            AbsenceReason::PinUnverifiable => "pin_unverifiable",
            AbsenceReason::AdapterAbsent => "adapter_absent",
            AbsenceReason::AdapterIncomplete => "adapter_incomplete",
            AbsenceReason::AdapterBaseNotPinnedSnapshot => "adapter_base_not_pinned_snapshot",
        }
    }
}

/// A comparison that did not run. It carries no score of any kind and is not a
/// serve-gate input (`gate_input` is always false).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsenceRecord {
    /// The requested base.
    pub base: BaseLabel,
    /// Why nothing was compared.
    pub reason: AbsenceReason,
    /// The snapshot directory that was checked, when the pin had a revision.
    pub searched: Option<String>,
    /// Whether `--adapter` was passed.
    pub adapter_given: bool,
    /// Whether both adapter files are present.
    pub adapter_on_disk: bool,
}

impl AbsenceRecord {
    /// The `not_compared` record. No `n`, no score key, no verdict.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let snapshot_on_disk = matches!(
            self.reason,
            AbsenceReason::AdapterAbsent
                | AbsenceReason::AdapterIncomplete
                | AbsenceReason::AdapterBaseNotPinnedSnapshot
        );
        serde_json::json!({
            "schema": BEFORE_AFTER_SCHEMA,
            "status": "not_compared",
            "reason": self.reason.as_str(),
            "base_requested": self.base.requested,
            "base_model": self.base.repo_id,
            "base_revision": self.base.revision,
            "snapshot_on_disk": snapshot_on_disk,
            "searched": self.searched,
            "adapter": {
                "given": self.adapter_given,
                "on_disk": self.adapter_on_disk,
            },
            "slices": SLICE_NAMES,
            "detail": format!(
                "{} was not compared ({}); nothing was scored",
                self.base.repo_id,
                self.reason.as_str()
            ),
            "gate_input": false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_slice(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../mens/data")
            .join(name)
            .join("reasoning.jsonl")
    }

    /// Render `n` synthetic rows `t-0000..`, one per line, trailing newline.
    fn rows(n: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "{{\"id\":\"t-{i:04}\",\"question\":\"q{i}\",\"answer\":\"{i}\",\"source\":\"x\"}}\n"
                )
            })
            .collect()
    }

    fn write_tmp(content: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("reasoning.jsonl");
        std::fs::write(&p, content).expect("write fixture");
        (dir, p)
    }

    // ---- Task 1: checked-in slices ----

    #[test]
    fn checked_in_gsm8k_and_math500_slices_have_32_pinned_rows() {
        let g = load_slice(&repo_slice("gsm8k_think"), SLICE_SIZE).expect("gsm8k loads");
        assert_eq!(g.len(), 32);
        assert_eq!(g.first().map(|i| i.id.as_str()), Some("gsm8k-test-0000"));
        assert_eq!(g.last().map(|i| i.id.as_str()), Some("gsm8k-test-0031"));
        let m = load_slice(&repo_slice("math500_think"), SLICE_SIZE).expect("math500 loads");
        assert_eq!(m.len(), 32);
        assert_eq!(
            m.first().map(|i| i.id.as_str()),
            Some("test/precalculus/807.json")
        );
        assert_eq!(
            m.last().map(|i| i.id.as_str()),
            Some("test/prealgebra/1302.json")
        );
    }

    #[test]
    fn gsm8k_source_no_longer_claims_the_old_row_count() {
        let text = std::fs::read_to_string(repo_slice("gsm8k_think")).expect("read gsm8k");
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line).expect("json row");
            let source = v["source"].as_str().unwrap_or("");
            assert!(
                !source.contains("64"),
                "stale row count in source: {source}"
            );
        }
    }

    #[test]
    fn load_slice_on_a_missing_file_is_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(load_slice(&dir.path().join("absent.jsonl"), SLICE_SIZE).is_err());
    }

    // ---- Task 2: refusal rules ----

    #[test]
    fn wrong_count_is_refused() {
        for n in [31usize, 33] {
            let (_d, p) = write_tmp(rows(n).as_bytes());
            let err = load_slice(&p, SLICE_SIZE).expect_err("wrong count must fail");
            assert!(
                err.contains(&format!("found {n}")) && err.contains("expected 32"),
                "{err}"
            );
        }
        let (_d, p) = write_tmp(rows(32).as_bytes());
        assert_eq!(load_slice(&p, SLICE_SIZE).expect("32 ok").len(), 32);
    }

    #[test]
    fn duplicate_id_is_refused() {
        let mut s = rows(31);
        s.push_str("{\"id\":\"t-0000\",\"question\":\"q\",\"answer\":\"1\"}\n");
        let (_d, p) = write_tmp(s.as_bytes());
        let err = load_slice(&p, SLICE_SIZE).expect_err("duplicate id must fail");
        assert!(err.contains("duplicate id"), "{err}");
    }

    #[test]
    fn empty_inputs_are_refused() {
        let bad = [
            "".to_string(),
            "{\"id\":\"a\",\"question\":\"  \\t\",\"answer\":\"1\"}\n".to_string(),
            "{\"id\":\"a\",\"question\":\"q\",\"answer\":\"\"}\n".to_string(),
            "{\"id\":\"a\",\"question\":\"q\",\"answer\":\"  \"}\n".to_string(),
            "{\"question\":\"q\",\"answer\":\"1\"}\n".to_string(),
            "{\"id\":\"\",\"question\":\"q\",\"answer\":\"1\"}\n".to_string(),
        ];
        for (i, content) in bad.iter().enumerate() {
            let (_d, p) = write_tmp(content.as_bytes());
            assert!(load_slice(&p, 1).is_err(), "case {i} should be refused");
        }
        // empty file with expected_n 32 is also refused (count mismatch)
        let (_d, p) = write_tmp(b"");
        assert!(load_slice(&p, SLICE_SIZE).is_err());
    }

    #[test]
    fn blank_lines_are_skipped() {
        let s: String = rows(32).lines().map(|l| format!("{l}\n\n   \n")).collect();
        let (_d, p) = write_tmp(s.as_bytes());
        assert_eq!(
            load_slice(&p, SLICE_SIZE).expect("blank lines ok").len(),
            32
        );
    }

    #[test]
    fn items_come_back_in_file_order() {
        let (_d, p) = write_tmp(rows(32).as_bytes());
        let ids: Vec<String> = load_slice(&p, SLICE_SIZE)
            .expect("loads")
            .into_iter()
            .map(|i| i.id)
            .collect();
        let want: Vec<String> = (0..32).map(|i| format!("t-{i:04}")).collect();
        assert_eq!(ids, want);
    }

    #[test]
    fn malformed_json_names_the_line() {
        let s = format!("{}not json\n", rows(2));
        let (_d, p) = write_tmp(s.as_bytes());
        let err = load_slice(&p, 3).expect_err("malformed must fail");
        assert!(
            err.contains(":3:") && err.contains("reasoning.jsonl"),
            "{err}"
        );
    }

    #[test]
    fn size_cap_boundary() {
        let over = vec![b'x'; (MAX_SLICE_BYTES + 1) as usize];
        let (_d, p) = write_tmp(&over);
        let err = load_slice(&p, SLICE_SIZE).expect_err("oversize must fail");
        assert!(err.contains("size cap"), "{err}");
        let (_d2, ok) = write_tmp(rows(32).as_bytes());
        assert!(load_slice(&ok, SLICE_SIZE).is_ok());
    }

    // ---- Task 3: vox_think shape, template, and golden-copy guards ----

    const FINAL_LINE: &str = "Give the final integer alone on the last line.";

    fn raw_rows(name: &str) -> Vec<serde_json::Value> {
        std::fs::read_to_string(repo_slice(name))
            .expect("read slice")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("json row"))
            .collect()
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn vox_think_shape() {
        let items = load_slice(&repo_slice("vox_think"), SLICE_SIZE).expect("vox_think loads");
        let raw = raw_rows("vox_think");
        assert_eq!(items.len(), 32);
        let mut kinds: HashMap<String, usize> = HashMap::new();
        for (i, (item, row)) in items.iter().zip(&raw).enumerate() {
            assert_eq!(item.id, format!("vox-think-{i:04}"));
            let kind = row["kind"].as_str().unwrap_or("").to_string();
            assert!(
                ["trace", "debug", "plan-check"].contains(&kind.as_str()),
                "{}: bad kind {kind:?}",
                item.id
            );
            *kinds.entry(kind).or_default() += 1;
            assert!(
                item.answer.parse::<i64>().is_ok(),
                "{}: gold not i64",
                item.id
            );
            assert_eq!(row["source"].as_str(), Some("hand-authored"), "{}", item.id);
            assert!(
                row["program"]
                    .as_str()
                    .is_some_and(|p| !p.trim().is_empty()),
                "{}: missing program",
                item.id
            );
            let last = item.question.lines().rev().find(|l| !l.trim().is_empty());
            assert_eq!(last, Some(FINAL_LINE), "{}: final-answer line", item.id);
        }
        for k in ["trace", "debug", "plan-check"] {
            assert!(
                kinds.get(k).copied().unwrap_or(0) >= 8,
                "kind {k}: {kinds:?}"
            );
        }
    }

    #[test]
    fn vox_think_avoids_the_template() {
        for item in load_slice(&repo_slice("vox_think"), SLICE_SIZE).expect("loads") {
            let q = item.question.to_lowercase();
            for marker in [
                "plan:",
                "syntax considerations",
                "implementation:",
                "construct in vox",
            ] {
                assert!(
                    !q.contains(marker),
                    "{}: template marker {marker:?}",
                    item.id
                );
            }
        }
    }

    fn collect_vox(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let p = entry.expect("entry").path();
            if p.is_dir() {
                collect_vox(&p, out);
            } else if p.extension().is_some_and(|e| e == "vox") {
                out.push(p);
            }
        }
    }

    #[test]
    fn vox_think_is_not_a_golden_copy() {
        let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden");
        let mut files = Vec::new();
        collect_vox(&golden_dir, &mut files);
        assert!(
            !files.is_empty(),
            "no golden examples found at {}",
            golden_dir.display()
        );
        let golden: Vec<String> = files
            .iter()
            .map(|p| squash(&std::fs::read_to_string(p).unwrap_or_default()))
            .collect();
        let mut seen = std::collections::HashSet::new();
        for (i, row) in raw_rows("vox_think").iter().enumerate() {
            let program = squash(row["program"].as_str().unwrap_or(""));
            assert!(!program.is_empty(), "row {i}: empty program");
            assert!(
                golden.iter().all(|g| !g.contains(&program)),
                "vox-think-{i:04} is a verbatim golden example"
            );
            assert!(
                seen.insert(program),
                "vox-think-{i:04} duplicates another program"
            );
        }
    }

    // ---- 314-02: BenchOutcome ----

    #[test]
    fn not_scored_json_has_no_score_keys() {
        let o = BenchOutcome::NotScored {
            reason: "load failed".to_string(),
            n_requested: SLICE_SIZE,
        };
        let v = o.to_json();
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        assert_eq!(v["n_requested"], 32);
        assert_eq!(v["reason"], "load failed");
        let obj = v.as_object().expect("object");
        assert!(!obj.contains_key("accuracy"), "accuracy key must be absent");
        assert!(
            !obj.contains_key("think_initiation_rate"),
            "think_initiation_rate key must be absent"
        );
    }

    #[test]
    fn from_score_requires_every_requested_item() {
        let full = ReasoningScore {
            accuracy: 1.0,
            think_initiation_rate: 1.0,
            n: 32,
        };
        assert_eq!(
            BenchOutcome::from_score(full, 32),
            BenchOutcome::Scored(full)
        );
        for n in [0usize, 31] {
            let partial = ReasoningScore {
                accuracy: 0.0,
                think_initiation_rate: 0.0,
                n,
            };
            match BenchOutcome::from_score(partial, 32) {
                BenchOutcome::NotScored {
                    reason,
                    n_requested,
                } => {
                    assert_eq!(n_requested, 32);
                    assert!(
                        reason.contains(&format!("scored {n} of 32 items")),
                        "reason was: {reason}"
                    );
                }
                other => panic!("n={n} must be NotScored, got {other:?}"),
            }
        }
        // The empty-batch tuple can never be Scored, even when nothing was requested.
        let empty = crate::qwen_reasoning::score_completions(std::iter::empty());
        assert_eq!(empty.n, 0);
        assert!(matches!(
            BenchOutcome::from_score(empty, 0),
            BenchOutcome::NotScored { .. }
        ));
        assert!(matches!(
            BenchOutcome::from_score(empty, 32),
            BenchOutcome::NotScored { .. }
        ));
    }

    #[test]
    fn scored_json_carries_n_and_exact_accuracy() {
        let owned: Vec<(String, String)> = (0..32)
            .map(|i| {
                let completion = if i < 3 {
                    format!("\\boxed{{{i}}}")
                } else {
                    "\\boxed{999999}".to_string()
                };
                (completion, i.to_string())
            })
            .collect();
        let score = crate::qwen_reasoning::score_completions(
            owned.iter().map(|(c, g)| (c.as_str(), g.as_str())),
        );
        assert_eq!(score.accuracy, 3.0 / 32.0);
        let outcome = BenchOutcome::from_score(score, 32);
        assert!(matches!(outcome, BenchOutcome::Scored(_)));
        let v = outcome.to_json();
        assert_eq!(v["status"], "scored");
        assert_eq!(v["n"], 32);
        assert_eq!(v["accuracy"].as_f64(), Some(3.0 / 32.0));
        let text = serde_json::to_string(&v).expect("serialize");
        let back: serde_json::Value = serde_json::from_str(&text).expect("parse");
        assert_eq!(back["accuracy"].as_f64(), Some(3.0 / 32.0));
        assert_eq!(back["n"], 32);
    }

    #[test]
    fn every_gold_round_trips_through_the_grader() {
        use crate::qwen_reasoning::grade_final_answer;
        let mut total = 0;
        for name in SLICE_NAMES {
            for item in load_slice(&repo_slice(name), SLICE_SIZE).expect("loads") {
                total += 1;
                assert!(
                    grade_final_answer(&format!("\\boxed{{{}}}", item.answer), &item.answer),
                    "{}: boxed gold must grade true",
                    item.id
                );
                assert!(
                    !grade_final_answer("\\boxed{999999}", &item.answer),
                    "{}: boxed 999999 must grade false",
                    item.id
                );
            }
        }
        assert_eq!(total, 96);
    }

    // ---- 316-01: before/after comparison record ----

    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";
    const PIN_0_6B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";

    const ALL_REASONS: [AbsenceReason; 6] = [
        AbsenceReason::BaseSnapshotAbsent,
        AbsenceReason::BaseSnapshotIncomplete,
        AbsenceReason::PinUnverifiable,
        AbsenceReason::AdapterAbsent,
        AbsenceReason::AdapterIncomplete,
        AbsenceReason::AdapterBaseNotPinnedSnapshot,
    ];

    fn record(pin: &str, reason: AbsenceReason) -> serde_json::Value {
        AbsenceRecord {
            base: BaseLabel::from_pin(pin),
            reason,
            searched: Some("/hub/x".to_string()),
            adapter_given: true,
            adapter_on_disk: false,
        }
        .to_json()
    }

    /// Every object key anywhere in `v`, plus every string value under a `status` key.
    fn keys_and_statuses(
        v: &serde_json::Value,
        keys: &mut Vec<String>,
        statuses: &mut Vec<String>,
    ) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, child) in map {
                    keys.push(k.clone());
                    if k == "status"
                        && let Some(s) = child.as_str()
                    {
                        statuses.push(s.to_string());
                    }
                    keys_and_statuses(child, keys, statuses);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    keys_and_statuses(child, keys, statuses);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn absence_record_names_the_requested_pin() {
        let v = record(PIN_8B, AbsenceReason::BaseSnapshotAbsent);
        assert_eq!(v["schema"], "reasoning_before_after.v1");
        assert_eq!(v["schema"], BEFORE_AFTER_SCHEMA);
        assert_eq!(v["status"], "not_compared");
        assert_eq!(v["reason"], "base_snapshot_absent");
        assert_eq!(v["base_requested"], PIN_8B);
        assert_eq!(v["base_model"], "Qwen/Qwen3-8B");
        assert_eq!(
            v["base_revision"],
            "b968826d9c46dd6066d109eabc6255188de91218"
        );
        assert_eq!(v["snapshot_on_disk"], false);
        assert_eq!(v["gate_input"], false);
        assert_eq!(v["slices"], serde_json::json!(SLICE_NAMES));
        let detail = v["detail"].as_str().expect("detail");
        assert!(detail.contains("Qwen/Qwen3-8B"), "{detail}");
        assert!(detail.ends_with("nothing was scored"), "{detail}");
    }

    #[test]
    fn absence_record_has_no_score_keys_anywhere() {
        for reason in ALL_REASONS {
            let v = record(PIN_8B, reason);
            let (mut keys, mut statuses) = (Vec::new(), Vec::new());
            keys_and_statuses(&v, &mut keys, &mut statuses);
            for banned in ["accuracy", "think_initiation_rate", "n"] {
                assert!(
                    !keys.iter().any(|k| k == banned),
                    "{reason:?}: key {banned} present"
                );
            }
            assert!(
                !statuses.iter().any(|s| s == "scored"),
                "{reason:?}: a scored status"
            );
            assert_eq!(v["reason"], reason.as_str());
            assert_eq!(
                v["snapshot_on_disk"],
                matches!(
                    reason,
                    AbsenceReason::AdapterAbsent
                        | AbsenceReason::AdapterIncomplete
                        | AbsenceReason::AdapterBaseNotPinnedSnapshot
                )
            );
        }
    }

    #[test]
    fn absence_record_for_another_size_names_that_size() {
        let v = record(PIN_0_6B, AbsenceReason::BaseSnapshotAbsent);
        assert_eq!(v["base_model"], "Qwen/Qwen3-0.6B");
        assert_eq!(
            v["base_revision"],
            "c1899de289a04d12100db370d81485cdf75e47ca"
        );
        let unpinned = record("Qwen/Qwen3-8B@main", AbsenceReason::PinUnverifiable);
        assert_eq!(unpinned["base_model"], "Qwen/Qwen3-8B");
        assert!(unpinned["base_revision"].is_null());
    }

    #[test]
    fn report_file_name_is_per_base_and_path_safe() {
        assert_eq!(
            BaseLabel::from_pin(PIN_8B).report_file_name(),
            "Qwen3-8B-b968826d9c46.json"
        );
        assert_eq!(
            BaseLabel::from_pin(PIN_0_6B).report_file_name(),
            "Qwen3-0.6B-c1899de289a0.json"
        );
        for hostile in [
            "../evil@main",
            "..@main",
            "a/..@main",
            "x/.hidden",
            "a\\..\\b",
        ] {
            let name = BaseLabel::from_pin(hostile).report_file_name();
            assert!(
                !name.contains('/') && !name.contains('\\'),
                "{hostile}: {name}"
            );
            let parts: Vec<_> = Path::new(&name).components().collect();
            assert!(
                matches!(parts.as_slice(), [std::path::Component::Normal(_)]),
                "{hostile}: {name} is not one plain segment"
            );
            assert!(!name.starts_with('.'), "{hostile}: {name}");
            assert!(name.ends_with("-unpinned.json"), "{hostile}: {name}");
        }
    }
}
