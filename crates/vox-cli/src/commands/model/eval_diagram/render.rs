//! Tallies -> markdown tables, deterministic (fixed number formats), so the report can be checked
//! to contain them verbatim instead of hand-transcribed numbers.

use std::fmt::Write as _;

use super::tally::{Rate, Tallies};

/// Percent with one decimal.
fn pct(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

/// `valid/n (p%)` and its Wilson interval.
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

/// Render the report tables.
#[must_use]
pub fn render_tables(t: &Tallies) -> String {
    let mut s = String::new();
    if t.synthetic {
        s.push_str("SYNTHETIC FIXTURE: not a measurement\n\n");
    }
    for (slot, st) in &t.slots {
        let _ = writeln!(s, "### Model {slot}\n");
        let _ = writeln!(s, "| Arm | First pass | 95% CI |");
        let _ = writeln!(s, "|-----|------------|--------|");
        for (arm, at) in &st.arms {
            let _ = writeln!(s, "| {arm} | {} |", rate_cells(&at.first_pass));
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::tally::{ArmTally, SlotTally};
    use std::collections::BTreeMap;
    use vox_eval::corpus_stats::{Z_95, wilson_interval};

    fn tallies(synthetic: bool) -> Tallies {
        let rate = Rate {
            valid: 2,
            n: 3,
            ci: wilson_interval(2, 3, Z_95),
        };
        let arms = BTreeMap::from([("mermaid".to_string(), ArmTally { first_pass: rate })]);
        Tallies {
            synthetic,
            corpus_size: 3,
            margin: 0.10,
            slots: BTreeMap::from([(
                "A".to_string(),
                SlotTally {
                    scored_tasks: 3,
                    arms,
                },
            )]),
        }
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
    fn rates_use_one_decimal_percent_and_bounds() {
        let out = render_tables(&tallies(false));
        assert!(
            out.contains("| mermaid | 2/3 (66.7%) | [20.8, 93.9] |"),
            "{out}"
        );
        assert_eq!(out, render_tables(&tallies(false)));
    }
}
