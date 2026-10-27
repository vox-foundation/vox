//! Tallies -> markdown tables, deterministic (fixed number formats), so the report can be checked
//! to contain them verbatim instead of hand-transcribed numbers.

use std::fmt::Write as _;

use vox_eval::paired_ni::NiVerdict;

use super::tally::{Paired, Rate, Tallies};

/// Percent with one decimal.
fn pct(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

/// `valid/n (p%)` and its Wilson interval in percent.
fn rate_cells(r: &Rate) -> String {
    format!(
        "{}/{} ({}) | [{:.1}, {:.1}]",
        r.valid,
        r.n,
        pct(r.ci.point),
        r.ci.low * 100.0,
        r.ci.high * 100.0
    )
}

/// `n/d` with a percent, or a dash when `d` is 0.
fn share(n: usize, d: usize) -> String {
    if d == 0 {
        "-".into()
    } else {
        format!("{n}/{d} ({})", pct(n as f64 / d as f64))
    }
}

/// Verdict words with the descriptive flags.
fn verdict_cell(p: &Paired) -> String {
    let mut s = match p.verdict {
        NiVerdict::NonInferior => "non-inferior",
        NiVerdict::Inconclusive => "inconclusive",
        NiVerdict::Inferior => "inferior",
    }
    .to_string();
    if p.superior {
        s.push_str(" (superior)");
    }
    if p.worse_within_margin {
        s.push_str(" (worse, within margin)");
    }
    s
}

/// The primary verdict as printed: non-inferior, inconclusive, inferior or not determined.
fn primary_words(key: &str) -> &str {
    match key {
        "non_inferior" => "non-inferior",
        "not_determined" => "not determined",
        other => other,
    }
}

/// Render the report tables.
#[must_use]
pub fn render_tables(t: &Tallies) -> String {
    let mut s = String::new();
    if t.synthetic {
        s.push_str("SYNTHETIC FIXTURE: not a measurement\n\n");
    }
    for (slot, st) in &t.slots {
        let _ = writeln!(s, "### Model {slot}\n");
        let drift = if st.model_drift {
            " (MODEL CHANGED MID-RUN)"
        } else {
            ""
        };
        let _ = writeln!(
            s,
            "Resolved model ids: {}{drift}\n",
            st.resolved_models.join(", ")
        );
        let _ = writeln!(
            s,
            "| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |"
        );
        let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|---|");
        for (arm, at) in &st.arms {
            let f = &at.fidelity;
            let _ = writeln!(
                s,
                "| {arm} | {} | {} | {} | {} | {} | {} | {} | {:.4} |",
                rate_cells(&at.first_pass),
                rate_cells(&at.after_repair),
                share(f.matched, f.valid),
                share(f.matched, f.scored),
                at.attempts,
                at.prompt_tokens,
                at.completion_tokens,
                at.cost_usd
            );
        }
        s.push('\n');
    }
    s.push_str("### Paired difference (VoxDiagram minus Mermaid)\n\n");
    s.push_str("| Endpoint | Model | a | b | c | d | Δ (points) | Newcombe 95% CI | Verdict | McNemar p |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for (endpoint, label) in [
        ("first_pass", "first pass"),
        ("after_repair", "after repair"),
    ] {
        for (slot, st) in &t.slots {
            let p = &st.paired[endpoint];
            let _ = writeln!(
                s,
                "| {label} | {slot} | {} | {} | {} | {} | {:+.1} | [{:+.1}, {:+.1}] | {} | {:.4} |",
                p.a,
                p.b,
                p.c,
                p.d,
                p.ci.point * 100.0,
                p.ci.low * 100.0,
                p.ci.high * 100.0,
                verdict_cell(p),
                p.mcnemar_p
            );
        }
    }
    s.push_str("\n### Per kind\n\n| Model | Kind | Arm | n | First pass | After repair |\n|---|---|---|---|---|---|\n");
    for (slot, st) in &t.slots {
        for (kind, arms) in &st.per_kind {
            for (arm, k) in arms {
                let _ = writeln!(
                    s,
                    "| {slot} | {kind} | {arm} | {} | {} | {} |",
                    k.n,
                    share(k.first_pass, k.n),
                    share(k.after_repair, k.n)
                );
            }
        }
    }
    s.push('\n');
    for (slot, st) in &t.slots {
        let state = match &st.incomplete_reason {
            None => "complete".to_string(),
            Some(r) => format!("incomplete ({r})"),
        };
        let _ = writeln!(
            s,
            "Completeness, model {slot}: {state}; {} scored, {} excluded for transport errors{}, {} missing.",
            st.scored_tasks,
            st.excluded.len(),
            if st.excluded.is_empty() {
                String::new()
            } else {
                format!(" ({})", st.excluded.join(", "))
            },
            st.missing.len()
        );
    }
    if let Some(reason) = &t.stopped {
        let _ = writeln!(s, "Run stopped early: {reason}.");
    }
    let _ = writeln!(
        s,
        "\nPrimary endpoint verdict (model A, first pass): {}",
        primary_words(&t.primary.verdict)
    );
    if let Some(reason) = &t.primary.reason {
        let _ = writeln!(s, "Verdict withheld: {reason}.");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::score::tests::{MM, VD_OK, attempt, header, task};
    use crate::commands::model::eval_diagram::stats::tests::rule;
    use crate::commands::model::eval_diagram::tally::tally;
    use crate::commands::model::eval_diagram::{Arm, Line, OracleOutput, RunHeader};

    fn tallies(synthetic: bool) -> Tallies {
        let mut h = header();
        if let Line::Run(RunHeader { synthetic: s, .. }) = &mut h {
            *s = synthetic;
        }
        let v = attempt("t1", Arm::Voxdiagram, "A", 0, Some(VD_OK));
        let m = attempt("t1", Arm::Mermaid, "A", 0, Some(MM));
        let out = |id: String| OracleOutput {
            id,
            ok: true,
            error: None,
            sets: serde_json::Value::Null,
            matched: Some(true),
        };
        let outs = [out(v.oracle_id()), out(m.oracle_id())];
        let lines = [h, Line::Attempt(v), Line::Attempt(m)];
        tally(&rule(0.05), &[task("t1")], &lines, &outs).unwrap()
    }

    #[test]
    fn synthetic_tables_say_so_on_the_first_line() {
        let out = render_tables(&tallies(true));
        assert_eq!(
            out.lines().next(),
            Some("SYNTHETIC FIXTURE: not a measurement")
        );
        assert!(!render_tables(&tallies(false)).contains("SYNTHETIC"));
    }

    #[test]
    fn tables_carry_rates_paired_rows_and_the_primary_line() {
        let out = render_tables(&tallies(false));
        assert!(
            out.contains("| voxdiagram | 1/1 (100.0%) | [20.7, 100.0] |"),
            "{out}"
        );
        assert!(
            out.contains("| first pass | A | 1 | 0 | 0 | 0 | +0.0 |"),
            "{out}"
        );
        assert!(
            out.contains("Completeness, model A: complete; 1 scored"),
            "{out}"
        );
        assert!(
            out.ends_with("Primary endpoint verdict (model A, first pass): inconclusive\n"),
            "{out}"
        );
        assert_eq!(out, render_tables(&tallies(false)));
    }

    #[test]
    fn helpers_format_shares_and_verdict_words() {
        assert_eq!(share(1, 4), "1/4 (25.0%)");
        assert_eq!(share(0, 0), "-");
        assert_eq!(primary_words("not_determined"), "not determined");
        assert_eq!(primary_words("inconclusive"), "inconclusive");
    }
}
