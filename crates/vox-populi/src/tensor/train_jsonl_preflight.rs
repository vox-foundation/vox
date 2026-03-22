//! Lightweight JSONL checks before native training (row shape sanity).

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Reject empty lines and lines longer than `max_line_bytes` (UTF-8 byte length).
pub fn preflight_train_jsonl(path: &Path, max_line_bytes: usize) -> anyhow::Result<usize> {
    let f = File::open(path).map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?;
    let mut reader = BufReader::new(f);
    let mut line = String::new();
    let mut n = 0usize;
    let max = max_line_bytes.max(4096);
    loop {
        line.clear();
        let r = reader.read_line(&mut line)?;
        if r == 0 {
            break;
        }
        n += 1;
        let t = line.trim_end_matches(['\r', '\n']);
        if t.is_empty() {
            anyhow::bail!(
                "train JSONL {} line {} is empty (remove blank lines or fix corpus export)",
                path.display(),
                n
            );
        }
        if t.len() > max {
            anyhow::bail!(
                "train JSONL {} line {} exceeds max length {} bytes (got {}). Split or truncate rows.",
                path.display(),
                n,
                max,
                t.len()
            );
        }
    }
    if n == 0 {
        anyhow::bail!("train JSONL {} is empty", path.display());
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn rejects_empty_line() {
        let d = tempdir().unwrap();
        let p = d.path().join("t.jsonl");
        let mut f = File::create(&p).unwrap();
        writeln!(f, r#"{{"prompt":"a","response":"b"}}"#).unwrap();
        writeln!(f).unwrap();
        let e = preflight_train_jsonl(&p, 1_000_000)
            .unwrap_err()
            .to_string();
        assert!(e.contains("empty"), "{e}");
    }

    #[test]
    fn counts_nonempty_lines() {
        let d = tempdir().unwrap();
        let p = d.path().join("t.jsonl");
        let mut f = File::create(&p).unwrap();
        writeln!(f, r#"{{"x":1}}"#).unwrap();
        writeln!(f, r#"{{"x":2}}"#).unwrap();
        assert_eq!(preflight_train_jsonl(&p, 100).unwrap(), 2);
    }
}
