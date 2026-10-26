//! Held-out reasoning slices (BENCH-02): three checked-in, fixed-size JSONL files
//! (`gsm8k_think`, `math500_think`, `vox_think`) and one validating loader.
//!
//! One JSON object per line. Required keys: `id`, `question`, `answer` (non-empty
//! strings). Extra keys (`source`, `kind`, `program`, ...) are ignored here.
//! There is no download path: the files live under `mens/data/<name>/reasoning.jsonl`.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

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
}
