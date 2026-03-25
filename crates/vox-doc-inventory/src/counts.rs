//! Per-language line / comment heuristics.

pub(crate) fn count_rust_lines(text: &str) -> (u64, u64, u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let n_total = lines.len() as u64;
    let mut triple = 0u64;
    let mut inner = 0u64;
    let mut plain = 0u64;
    for line in &lines {
        let s = line.trim_start();
        if s.starts_with("//!") {
            inner += 1;
        } else if s.starts_with("///") {
            triple += 1;
        } else if let Some(idx) = line.find("//") {
            let before = &line[..idx];
            if before.contains('"') || before.contains('\'') {
                continue;
            }
            let rest = &line[idx + 2..];
            if rest.starts_with('/') || rest.is_empty() {
                continue;
            }
            plain += 1;
        }
    }
    (n_total, triple, inner, plain)
}

pub(crate) fn count_md(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let headings = lines
        .iter()
        .filter(|l| l.trim_start().starts_with('#'))
        .count() as u64;
    (lines.len() as u64, headings)
}

pub(crate) fn count_ts(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let mut plain = 0u64;
    for line in &lines {
        if let Some(idx) = line.find("//") {
            let before = &line[..idx];
            if before.contains('"') || before.contains('\'') {
                continue;
            }
            let rest = &line[idx + 2..];
            if rest.starts_with('/') {
                continue;
            }
            plain += 1;
        }
    }
    (lines.len() as u64, plain)
}

pub(crate) fn count_shell(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let plain = lines
        .iter()
        .filter(|l| l.trim_start().starts_with('#'))
        .count() as u64;
    (lines.len() as u64, plain)
}

pub(crate) fn count_python(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let plain = lines
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('#') && !t.starts_with("#!")
        })
        .count() as u64;
    (lines.len() as u64, plain)
}
