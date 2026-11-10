//! Before/after examples for a card (spec D2), picked from stored held-out reports.
//!
//! Pure: this module reads nothing from disk. `facts` loads the reports and the checkpoint
//! texts; here they are joined, picked and given a compiler verdict read from the stored
//! check fields. The pick key is `(description length in chars, id)` and nothing else.

use std::collections::BTreeMap;

use serde_json::Value;

/// What the stored checks say about one output. Counts come from the record, never a re-run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Verdict {
    Passes,
    Empty,
    ParseErrors(Option<u64>),
    TypeErrors(Option<u64>),
    Stub,
    /// Compiles and is not a stub, yet `pass` is false: the first false stored check.
    Rejected(String),
}

impl Verdict {
    /// The phrase from the verdict list on docs/src/reference/mens-models.md. Counts and the
    /// check name are added by the card template.
    pub(crate) fn phrase(&self) -> &'static str {
        match self {
            Self::Passes => "passes",
            Self::Empty => "no code",
            Self::ParseErrors(_) => "does not parse",
            Self::TypeErrors(_) => "type errors",
            Self::Stub => "stub",
            Self::Rejected(_) => "rejected by",
        }
    }

    /// The stored error count the card appends to the phrase, when one was recorded.
    pub(crate) fn count(&self) -> Option<u64> {
        match self {
            Self::ParseErrors(n) | Self::TypeErrors(n) => *n,
            _ => None,
        }
    }

    /// The name of the stored check that rejected the output.
    pub(crate) fn check(&self) -> Option<&str> {
        match self {
            Self::Rejected(name) => Some(name),
            _ => None,
        }
    }
}

/// One side (base or adapter) of an example.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Side {
    pub passed: bool,
    pub verdict: Verdict,
    /// The code the scorer checked; `None` renders as `no recorded output`.
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Example {
    pub id: String,
    pub description: String,
    pub base: Side,
    pub adapter: Side,
}

/// Totals counted from the two reports, plus the picked examples.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExampleFacts {
    /// Items present in both reports.
    pub total: usize,
    /// Adapter passes, base fails.
    pub fixes: usize,
    /// Base passes, adapter fails.
    pub breaks: usize,
    /// Ids present in only one report; in no other count.
    pub unmatched_ids: usize,
    pub wins: Vec<Example>,
    pub loss: Option<Example>,
    /// The shortest item the adapter passes that is not a shown win (same key as the wins).
    pub use_it: Option<Example>,
}

impl Side {
    /// Fence length for the shown output: longer than any backtick run in it (T-339-16).
    pub(crate) fn fence(&self) -> String {
        fence_for(self.code.as_deref().unwrap_or(""))
    }

    /// `vox` only for code that passed the scorer; everything else is `text` (Pitfall 3).
    pub(crate) fn lang(&self) -> &'static str {
        if self.passed { "vox" } else { "text" }
    }
}

impl Example {
    /// Fence length for the item description.
    pub(crate) fn prompt_fence(&self) -> String {
        fence_for(&self.description)
    }

    /// `(true, base)` then `(false, adapter)`.
    pub(crate) fn sides(&self) -> Vec<(bool, &Side)> {
        vec![(true, &self.base), (false, &self.adapter)]
    }

    /// JSON-escaped prompt string for the Hugging Face `widget:` frontmatter entry.
    pub(crate) fn widget_prompt(&self) -> String {
        serde_json::to_string(&format!(
            "{}{}",
            super::card_facts::EVAL_USER_TURN_PREFIX,
            self.description
        ))
        .unwrap_or_default()
    }

    /// JSON-escaped recorded adapter output for the Hugging Face `widget:` frontmatter entry.
    pub(crate) fn widget_output(&self) -> Option<String> {
        self.adapter
            .code
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok())
    }
}

impl ExampleFacts {
    /// The shown blocks in order: `(true, win)` for each win, then `(false, loss)`.
    pub(crate) fn blocks(&self) -> Vec<(bool, &Example)> {
        let wins = self.wins.iter().map(|e| (true, e));
        wins.chain(self.loss.iter().map(|e| (false, e))).collect()
    }

    /// True when at least one win has a recorded adapter output for the HF `widget:` frontmatter.
    pub(crate) fn has_widget(&self) -> bool {
        self.wins.iter().any(|w| w.adapter.code.is_some())
    }
}

/// A backtick fence longer than any backtick run in `text`, minimum three, so output
/// pasted into a card cannot close its own fence.
pub(crate) fn fence_for(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// The text the scorer checked: `extract_code` from vox-ml-cli `metrics.rs`.
// vox:defactored-from vox-ml-cli 2026-11-06
pub(crate) fn shown_code(text: &str) -> String {
    let body = if let Some(i) = text.rfind("</think>") {
        &text[i + "</think>".len()..]
    } else if text.trim_start().starts_with("<think>") {
        ""
    } else {
        text
    };
    let Some(start) = body.find("```") else {
        return body.trim().to_string();
    };
    let after = &body[start + 3..];
    let from = after.find('\n').map_or(0, |n| n + 1);
    let code = &after[from..];
    code.find("```")
        .map_or(code, |end| &code[..end])
        .trim()
        .to_string()
}

/// The single checkpoint text whose `answer_region` starts with `preview` (240 chars, as the
/// scorer stored it). Zero or several matches give `None`: never a guess.
fn full_text<'a>(preview: &str, texts: &'a [String]) -> Option<&'a str> {
    let mut hits = texts.iter().filter(|t| {
        let region = vox_eval::qwen_reasoning::answer_region(t);
        region.chars().take(240).eq(preview.chars())
    });
    let first = hits.next()?;
    hits.next().is_none().then_some(first.as_str())
}

fn first_number(s: &str) -> Option<u64> {
    s.split(|c: char| !c.is_ascii_digit())
        .find_map(|t| t.parse().ok())
}

/// Verdict in the order the scorer builds `pass` (metrics.rs `verify_completion`).
fn verdict_of(sample: &Value) -> Verdict {
    if sample["pass"].as_bool() == Some(true) {
        return Verdict::Passes;
    }
    let c = &sample["checks"];
    if !c.is_object() {
        return Verdict::Rejected("no sample".to_string());
    }
    if c["non_empty"].as_bool() == Some(false) {
        return Verdict::Empty;
    }
    if c["parse_ok"].as_bool() == Some(false) {
        return Verdict::ParseErrors(c["parse_error"].as_str().and_then(first_number));
    }
    if c["typecheck_ok"].as_bool() == Some(false) {
        return Verdict::TypeErrors(c["diag_errors"].as_u64());
    }
    if c["anti_stub_pass"].as_bool() == Some(false) {
        return Verdict::Stub;
    }
    let first_false = ["pass_compile", "pass_ast", "pass_exec"]
        .into_iter()
        .find(|k| c[*k].as_bool() == Some(false))
        .unwrap_or("pass");
    Verdict::Rejected(first_false.to_string())
}

fn side(result: &Value, texts: Option<&[String]>) -> Side {
    let sample = result["samples"].get(0).unwrap_or(&Value::Null);
    let code = sample["completion_preview"]
        .as_str()
        .zip(texts)
        .and_then(|(preview, texts)| full_text(preview, texts))
        .map(shown_code);
    Side {
        passed: sample["pass"].as_bool() == Some(true),
        verdict: verdict_of(sample),
        code,
    }
}

fn by_id(report: &Value) -> Option<BTreeMap<&str, &Value>> {
    let rows = report["results"].as_array()?;
    Some(
        rows.iter()
            .filter_map(|r| Some((r["id"].as_str()?, r)))
            .collect(),
    )
}

/// D2: wins are the two shortest items the adapter passes and the base fails; the loss is the
/// shortest item the adapter still fails. `None` when either report has no `results`.
pub(crate) fn pick_examples(
    base: &Value,
    adapter: &Value,
    base_texts: Option<&[String]>,
    adapter_texts: Option<&[String]>,
) -> Option<ExampleFacts> {
    let (b, a) = (by_id(base)?, by_id(adapter)?);
    let mut total = 0;
    let (mut fixes, mut breaks) = (Vec::new(), 0);
    let (mut adapter_fails, mut adapter_passes) = (Vec::new(), Vec::new());
    for (id, ar) in &a {
        let Some(br) = b.get(id) else { continue };
        total += 1;
        let (bs, as_) = (side(br, None), side(ar, None));
        let key = (
            ar["description"].as_str().unwrap_or("").chars().count(),
            *id,
        );
        match (bs.passed, as_.passed) {
            (false, true) => fixes.push(key),
            (true, false) => breaks += 1,
            _ => {}
        }
        if as_.passed {
            adapter_passes.push(key);
        } else {
            adapter_fails.push(key);
        }
    }
    let unmatched_ids = a.keys().filter(|k| !b.contains_key(*k)).count()
        + b.keys().filter(|k| !a.contains_key(*k)).count();
    let example = |id: &str| -> Option<Example> {
        let (br, ar) = (b.get(id)?, a.get(id)?);
        Some(Example {
            id: id.to_string(),
            description: ar["description"].as_str().unwrap_or("").to_string(),
            base: side(br, base_texts),
            adapter: side(ar, adapter_texts),
        })
    };
    let n_fixes = fixes.len();
    fixes.sort();
    adapter_fails.sort();
    adapter_passes.sort();
    let wins: Vec<Example> = fixes
        .iter()
        .take(2)
        .filter_map(|(_, id)| example(id))
        .collect();
    let use_it = adapter_passes
        .iter()
        .find(|(_, id)| !wins.iter().any(|w| w.id == *id))
        .and_then(|(_, id)| example(id));
    Some(ExampleFacts {
        total,
        fixes: n_fixes,
        breaks,
        unmatched_ids,
        wins,
        loss: adapter_fails.first().and_then(|(_, id)| example(id)),
        use_it,
    })
}

#[cfg(test)]
#[path = "examples_tests.rs"]
mod tests;
