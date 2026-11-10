//! Paired collateral comparison for the three held-out reasoning slices.
//!
//! Each slice (`gsm8k_think`, `math500_think`, `vox_think`) is scored on two
//! per-item metrics, answer accuracy (`<slice>`) and think-initiation
//! (`<slice>_init`). Base and adapter answer the same items, so each metric is a
//! paired comparison: `losses` are items the base got and the adapter missed,
//! `gains` the reverse. A metric regresses when the exact one-sided McNemar test
//! on those discordant pairs rejects "the adapter is no worse" at the policy
//! `alpha`. The three slices' accuracy counts are also pooled into one test.
//! Derivation, power and alpha: `.planning/phases/335-*/335-RESEARCH.md`.

use crate::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};

/// Suffix that turns a slice name into its think-initiation key.
pub const THINK_INIT_SUFFIX: &str = "_init";

/// Report key and gate-message name of the pooled accuracy test.
pub const POOLED_KEY: &str = "pooled_accuracy";

/// Names the decision rule in every report, so a report written under another
/// rule is refused rather than reinterpreted.
pub const PAIRED_RULE: &str = "exact one-sided McNemar on paired items, fail when p < alpha";

/// Report text that keeps a pass from being read as more than it is.
pub const REASONING_SLICE_SCOPE_NOTE: &str = "The reasoning benches gsm8k_think, math500_think and vox_think are fixed 128-item slices, not full GSM8K or MATH-500 results. Each slice's accuracy and think-initiation, and accuracy pooled over the three slices, is tested for regression with an exact one-sided McNemar test on paired items; at these sizes only large regressions are detectable (see power), so a pass means no detectable regression.";

/// Which of a slice's two scores a baseline key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningMetric {
    Accuracy,
    ThinkInitiation,
}

/// Classify a baseline key by name: `gsm8k_think` is accuracy for that slice,
/// `gsm8k_think_init` its think-initiation. Anything else is not a reasoning key.
/// Never consults the filesystem, so the result does not depend on the cwd.
pub fn reasoning_slice_of(key: &str) -> Option<(&'static str, ReasoningMetric)> {
    let (base, metric) = match key.strip_suffix(THINK_INIT_SUFFIX) {
        Some(base) => (base, ReasoningMetric::ThinkInitiation),
        None => (key, ReasoningMetric::Accuracy),
    };
    SLICE_NAMES
        .iter()
        .find(|name| **name == base)
        .map(|name| (*name, metric))
}

/// Require `<slice>` and `<slice>_init` for every slice. The error lists every
/// missing key in `SLICE_NAMES` order, accuracy key first.
pub fn require_reasoning_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let have: Vec<&str> = keys.into_iter().collect();
    let mut missing: Vec<String> = Vec::new();
    for name in SLICE_NAMES {
        let init = format!("{name}{THINK_INIT_SUFFIX}");
        for key in [name.to_string(), init] {
            if !have.contains(&key.as_str()) {
                missing.push(key);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "baseline is missing {}; it must name accuracy and think-initiation for all three \
         {SLICE_SIZE}-item slices ({})",
        missing.join(", "),
        SLICE_NAMES.join(", ")
    ))
}

/// One item's result on one metric: `(item id, success)`.
pub type ItemFlag = (String, bool);

/// Exact one-sided McNemar p-value: `P(X >= losses)` for
/// `X ~ Binomial(losses + gains, 1/2)`, the chance of at least this many losses
/// among the discordant pairs if the adapter were no worse. `1.0` with no
/// discordant pairs. Summed in log space so it does not underflow at large `d`.
#[must_use]
pub fn mcnemar_one_sided_p(losses: usize, gains: usize) -> f64 {
    let d = losses + gains;
    if d == 0 {
        return 1.0;
    }
    // ln pmf(0) = d ln(1/2); ln pmf(k+1) = ln pmf(k) + ln(d-k) - ln(k+1).
    let mut ln_pmf = d as f64 * 0.5f64.ln();
    let mut p = 0.0;
    for k in 0..=d {
        if k >= losses {
            p += ln_pmf.exp();
        }
        if k < d {
            ln_pmf += ((d - k) as f64).ln() - ((k + 1) as f64).ln();
        }
    }
    p.min(1.0)
}

/// `x ln y`, with `0 ln 0 = 0`.
fn xlny(x: f64, y: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x * y.ln() }
}

/// Probability that the paired test fails `n` independent items at `alpha` when
/// each item is a loss with probability `p_loss` and a gain with `p_gain`. With
/// `p_loss == p_gain` this is the false-fail rate (the test's size).
///
/// Exact: the discordant count is Bin(n, p_loss + p_gain), the losses among `d`
/// discordant items are Bin(d, p_loss / (p_loss + p_gain)), and for each `d` the
/// test fails from the smallest loss count whose p-value is below `alpha` upward.
#[must_use]
pub fn mcnemar_power(n: usize, p_loss: f64, p_gain: f64, alpha: f64) -> f64 {
    let pd = p_loss + p_gain;
    if pd.is_nan() || pd <= 0.0 {
        return 0.0;
    }
    let mut ln_fact = vec![0.0f64; n + 1];
    for k in 1..=n {
        ln_fact[k] = ln_fact[k - 1] + (k as f64).ln();
    }
    let ln_choose = |a: usize, b: usize| ln_fact[a] - ln_fact[b] - ln_fact[a - b];
    let q = p_loss / pd;
    let mut power = 0.0;
    for d in 1..=n {
        // Smallest b with P(X >= b | X ~ Bin(d, 1/2)) < alpha, scanning down from d.
        let mut tail = 0.0;
        let mut crit = None;
        for b in (0..=d).rev() {
            tail += (ln_choose(d, b) + d as f64 * 0.5f64.ln()).exp();
            if tail >= alpha {
                break;
            }
            crit = Some(b);
        }
        let Some(crit) = crit else { continue };
        let ln_pd = ln_choose(n, d) + xlny(d as f64, pd) + xlny((n - d) as f64, 1.0 - pd);
        let fail: f64 = (crit..=d)
            .map(|b| (ln_choose(d, b) + xlny(b as f64, q) + xlny((d - b) as f64, 1.0 - q)).exp())
            .sum();
        power += ln_pd.exp() * fail;
    }
    power.min(1.0)
}

/// Design assumptions of the power table every report carries (phase 335): base
/// accuracy, discordant-item rates, and relative accuracy drops.
pub const POWER_BASE_ACCURACY: f64 = 0.5;
pub const POWER_DISCORDANT: [f64; 3] = [0.1, 0.2, 0.3];
pub const POWER_RELATIVE_DROPS: [f64; 3] = [0.05, 0.10, 0.20];

/// Power and false-fail rate of the paired test at `n` items, over the design grid.
fn power_rows(n: usize, alpha: f64) -> Vec<serde_json::Value> {
    POWER_DISCORDANT
        .iter()
        .map(|&pd| {
            let mut power = serde_json::Map::new();
            for rel in POWER_RELATIVE_DROPS {
                let delta = rel * POWER_BASE_ACCURACY;
                let p = mcnemar_power(n, (pd + delta) / 2.0, (pd - delta) / 2.0, alpha);
                power.insert(format!("{:.0}%", rel * 100.0), serde_json::json!(round4(p)));
            }
            serde_json::json!({
                "discordant": pd,
                "false_fail": round4(mcnemar_power(n, pd / 2.0, pd / 2.0, alpha)),
                "power_by_relative_drop": power,
            })
        })
        .collect()
}

fn round4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

/// What the paired test can and cannot detect at the slice size and pooled, so a
/// pass is read with its power next to it.
#[must_use]
pub fn power_json(alpha: f64) -> serde_json::Value {
    let pooled_n = SLICE_SIZE * SLICE_NAMES.len();
    let min_losses = (1..=SLICE_SIZE)
        .find(|&b| mcnemar_one_sided_p(b, 0) < alpha)
        .unwrap_or(usize::MAX);
    // Seven tests (accuracy and think-initiation per slice, pooled accuracy); the
    // union bound on any false fail, at the worst discordant rate of the grid.
    let family_bound: f64 = POWER_DISCORDANT
        .iter()
        .map(|&pd| {
            let slice = mcnemar_power(SLICE_SIZE, pd / 2.0, pd / 2.0, alpha);
            let pooled = mcnemar_power(pooled_n, pd / 2.0, pd / 2.0, alpha);
            2.0 * SLICE_NAMES.len() as f64 * slice + pooled
        })
        .fold(0.0, f64::max);
    serde_json::json!({
        "model": "items independent; loss rate (discordant + drop) / 2, gain rate (discordant - drop) / 2, drop = relative drop x base accuracy",
        "base_accuracy": POWER_BASE_ACCURACY,
        "min_losses_with_no_gains": min_losses,
        "slice": { "n": SLICE_SIZE, "rows": power_rows(SLICE_SIZE, alpha) },
        "pooled": { "n": pooled_n, "rows": power_rows(pooled_n, alpha) },
        "family_false_fail_bound": round4(family_bound.min(1.0)),
    })
}

/// One paired comparison, as stored in the report.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedMetric {
    /// `<slice>`, `<slice>_init` or [`POOLED_KEY`].
    pub key: String,
    pub n: usize,
    /// Base success rate.
    pub pre: f64,
    /// Adapter success rate.
    pub post: f64,
    /// Base right, adapter wrong.
    pub losses: usize,
    /// Base wrong, adapter right.
    pub gains: usize,
    pub p_value: f64,
    /// `p_value < alpha`.
    pub regressed: bool,
}

impl PairedMetric {
    /// Build from counts: `base_ok` items the base got, `losses`/`gains` as above.
    /// Refuses counts that cannot come from `n` paired items.
    pub fn from_counts(
        key: &str,
        n: usize,
        base_ok: usize,
        losses: usize,
        gains: usize,
        alpha: f64,
    ) -> Result<Self, String> {
        check_alpha(alpha)?;
        if n == 0 {
            return Err(format!("{key}: no items"));
        }
        if base_ok > n || losses > base_ok || gains > n - base_ok {
            return Err(format!(
                "{key}: counts are inconsistent (n={n}, base right={base_ok}, losses={losses}, \
                 gains={gains})"
            ));
        }
        let post_ok = base_ok - losses + gains;
        let p_value = mcnemar_one_sided_p(losses, gains);
        Ok(Self {
            key: key.to_string(),
            n,
            pre: base_ok as f64 / n as f64,
            post: post_ok as f64 / n as f64,
            losses,
            gains,
            p_value,
            regressed: p_value < alpha,
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "n": self.n,
            "pre": self.pre,
            "post": self.post,
            "losses": self.losses,
            "gains": self.gains,
            "p_value": self.p_value,
            "regressed": self.regressed,
        })
    }
}

pub(crate) fn check_alpha(alpha: f64) -> Result<(), String> {
    if alpha.is_finite() && alpha > 0.0 && alpha < 1.0 {
        Ok(())
    } else {
        Err(format!("alpha {alpha} is not in (0, 1)"))
    }
}

/// Pair `base` and `adapter` item by item. Both must list the same ids in the
/// same order with no duplicates; anything else is refused, never re-aligned.
pub fn paired_metric(
    key: &str,
    base: &[ItemFlag],
    adapter: &[ItemFlag],
    alpha: f64,
) -> Result<PairedMetric, String> {
    if base.len() != adapter.len() {
        return Err(format!(
            "{key}: base has {} items, adapter {}",
            base.len(),
            adapter.len()
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let (mut base_ok, mut losses, mut gains) = (0, 0, 0);
    for ((bid, b), (aid, a)) in base.iter().zip(adapter) {
        if bid != aid {
            return Err(format!(
                "{key}: item order differs (base {bid}, adapter {aid}); refusing to pair"
            ));
        }
        if !seen.insert(bid.as_str()) {
            return Err(format!("{key}: duplicate item id {bid}"));
        }
        base_ok += usize::from(*b);
        losses += usize::from(*b && !*a);
        gains += usize::from(!*b && *a);
    }
    PairedMetric::from_counts(key, base.len(), base_ok, losses, gains, alpha)
}

/// Both paired comparisons for one slice.
#[derive(Debug, Clone)]
pub struct ReasoningSliceVerdict {
    pub slice: &'static str,
    pub n: usize,
    pub accuracy: PairedMetric,
    pub think_initiation: PairedMetric,
}

impl ReasoningSliceVerdict {
    /// True when accuracy or think-initiation regressed.
    pub fn exceeds_threshold(&self) -> bool {
        self.accuracy.regressed || self.think_initiation.regressed
    }

    /// `<slice>` if accuracy regressed, else `<slice>_init` if think-initiation did.
    pub fn failed_key(&self) -> Option<String> {
        self.failing_metric().map(|m| m.key.clone())
    }

    /// The regressed metric, accuracy first.
    pub fn failing_metric(&self) -> Option<&PairedMetric> {
        [&self.accuracy, &self.think_initiation]
            .into_iter()
            .find(|m| m.regressed)
    }
}

/// Compare one slice. `accuracy` and `think_initiation` are `(base, adapter)`
/// per-item flags; each side must have exactly [`SLICE_SIZE`] items.
pub fn eval_reasoning_slice(
    slice: &str,
    accuracy: (&[ItemFlag], &[ItemFlag]),
    think_initiation: (&[ItemFlag], &[ItemFlag]),
    alpha: f64,
) -> Result<ReasoningSliceVerdict, String> {
    let slice = known_slice(slice)?;
    for (side, len) in [
        ("accuracy base", accuracy.0.len()),
        ("accuracy adapter", accuracy.1.len()),
        ("think_initiation base", think_initiation.0.len()),
        ("think_initiation adapter", think_initiation.1.len()),
    ] {
        if len != SLICE_SIZE {
            return Err(format!("{slice} {side}: n={len}, expected {SLICE_SIZE}"));
        }
    }
    let acc = paired_metric(slice, accuracy.0, accuracy.1, alpha)?;
    let init_key = format!("{slice}{THINK_INIT_SUFFIX}");
    let think = paired_metric(&init_key, think_initiation.0, think_initiation.1, alpha)?;
    if accuracy
        .0
        .iter()
        .map(|(id, _)| id)
        .ne(think_initiation.0.iter().map(|(id, _)| id))
    {
        return Err(format!(
            "{slice}: accuracy and think-initiation name different items"
        ));
    }
    Ok(ReasoningSliceVerdict {
        slice,
        n: SLICE_SIZE,
        accuracy: acc,
        think_initiation: think,
    })
}

fn known_slice(slice: &str) -> Result<&'static str, String> {
    SLICE_NAMES
        .iter()
        .find(|name| **name == slice)
        .copied()
        .ok_or_else(|| {
            format!(
                "{slice}: not a reasoning slice ({})",
                SLICE_NAMES.join(", ")
            )
        })
}

/// Accuracy of every verdict pooled into one test: losses and gains summed
/// (under H0 a sum of independent Binomial(d_i, 1/2) is Binomial(Σd_i, 1/2)).
pub fn pooled_accuracy(
    verdicts: &[ReasoningSliceVerdict],
    alpha: f64,
) -> Result<PairedMetric, String> {
    let sum = |f: fn(&PairedMetric) -> usize| verdicts.iter().map(|v| f(&v.accuracy)).sum();
    let n: usize = sum(|m| m.n);
    let base_ok: usize = sum(|m| (m.pre * m.n as f64).round() as usize);
    PairedMetric::from_counts(
        POOLED_KEY,
        n,
        base_ok,
        sum(|m| m.losses),
        sum(|m| m.gains),
        alpha,
    )
}

/// The `reasoning_slices` block of the collateral report.
pub fn reasoning_slices_json(
    verdicts: &[ReasoningSliceVerdict],
    pooled: Option<&PairedMetric>,
    alpha: f64,
) -> serde_json::Value {
    let mut benches = serde_json::Map::new();
    for v in verdicts {
        benches.insert(
            v.slice.to_string(),
            serde_json::json!({
                "n": v.n,
                "accuracy": v.accuracy.to_json(),
                "think_initiation": v.think_initiation.to_json(),
            }),
        );
    }
    serde_json::json!({
        "scope_note": REASONING_SLICE_SCOPE_NOTE,
        "rule": PAIRED_RULE,
        "alpha": alpha,
        "slice_size": SLICE_SIZE,
        "benches": benches,
        "pooled": pooled.map(PairedMetric::to_json),
        "power": power_json(alpha),
    })
}

fn stored_count(m: &serde_json::Value, key: &str, field: &str) -> Result<usize, String> {
    m.get(field)
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("{key}: stored {field} is missing or not a count"))
}

/// Rebuild one stored metric from its counts. `pre` must be a whole number of
/// items; `post`, `p_value` and `regressed` are recomputed, never trusted.
fn stored_metric(m: &serde_json::Value, key: &str, alpha: f64) -> Result<PairedMetric, String> {
    let n = stored_count(m, key, "n")?;
    let pre = m
        .get("pre")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("{key}: stored pre is missing or not a finite number"))?;
    let base_ok = pre * n as f64;
    if (base_ok - base_ok.round()).abs() > 1e-9 || base_ok < 0.0 {
        return Err(format!(
            "{key}: stored pre {pre} is not a whole number of {n} items"
        ));
    }
    PairedMetric::from_counts(
        key,
        n,
        base_ok.round() as usize,
        stored_count(m, key, "losses")?,
        stored_count(m, key, "gains")?,
        alpha,
    )
}

/// Re-validate a stored collateral report against the policy `alpha` without
/// trusting its verdict fields.
///
/// Requires the stored rule and alpha to equal [`PAIRED_RULE`] and `alpha`, the
/// slice label, and every name in [`SLICE_NAMES`] with its counts. Every metric
/// and the pooled test are recomputed from the stored counts; stored p-values,
/// `regressed` flags and `status` are ignored. A regression is an `Err`.
pub fn verify_reasoning_slices(
    report: &serde_json::Value,
    alpha: f64,
) -> Result<Vec<ReasoningSliceVerdict>, String> {
    check_alpha(alpha)?;
    let block = report
        .get("reasoning_slices")
        .filter(|b| b.is_object())
        .ok_or("report has no reasoning_slices block (written by an older or edited run)")?;
    let rule = block.get("rule").and_then(serde_json::Value::as_str);
    match rule {
        Some(PAIRED_RULE) => {}
        Some(crate::sequential_mcnemar::SEQUENTIAL_RULE) => {
            return crate::sequential_mcnemar::verify_sequential_block(block, alpha);
        }
        _ => {
            return Err(format!(
                "reasoning_slices.rule is missing or not a known rule: expected \"{PAIRED_RULE}\" or \"{}\"",
                crate::sequential_mcnemar::SEQUENTIAL_RULE
            ));
        }
    }
    let stored_alpha = block
        .get("alpha")
        .and_then(serde_json::Value::as_f64)
        .ok_or("reasoning_slices has no numeric alpha")?;
    if stored_alpha != alpha {
        return Err(format!(
            "stored alpha {stored_alpha} is not the policy alpha {alpha}"
        ));
    }
    if block.get("scope_note").and_then(serde_json::Value::as_str)
        != Some(REASONING_SLICE_SCOPE_NOTE)
    {
        return Err("reasoning_slices.scope_note is missing or not the expected label".into());
    }
    if block.get("slice_size").and_then(serde_json::Value::as_u64) != Some(SLICE_SIZE as u64) {
        return Err(format!(
            "reasoning_slices.slice_size is missing or not {SLICE_SIZE}"
        ));
    }
    let benches = block
        .get("benches")
        .filter(|b| b.is_object())
        .ok_or("reasoning_slices.benches is missing")?;
    let mut verdicts = Vec::with_capacity(SLICE_NAMES.len());
    for name in SLICE_NAMES {
        let bench = benches
            .get(name)
            .ok_or_else(|| format!("reasoning_slices.benches is missing {name}"))?;
        let init_key = format!("{name}{THINK_INIT_SUFFIX}");
        let accuracy = stored_metric(&bench["accuracy"], name, alpha)?;
        let think = stored_metric(&bench["think_initiation"], &init_key, alpha)?;
        for m in [&accuracy, &think] {
            if m.n != SLICE_SIZE {
                return Err(format!("{}: n={}, expected {SLICE_SIZE}", m.key, m.n));
            }
        }
        let verdict = ReasoningSliceVerdict {
            slice: name,
            n: SLICE_SIZE,
            accuracy,
            think_initiation: think,
        };
        if let Some(m) = verdict.failing_metric() {
            return Err(regression_message(m, alpha));
        }
        verdicts.push(verdict);
    }
    let pooled = pooled_accuracy(&verdicts, alpha)?;
    if pooled.regressed {
        return Err(regression_message(&pooled, alpha));
    }
    Ok(verdicts)
}

/// `"<key> regressed: L losses vs G gains of n (pre → post), p=… < alpha"`.
pub fn regression_message(m: &PairedMetric, alpha: f64) -> String {
    format!(
        "{} regressed: {} losses vs {} gains of {} paired items (pre {:.4} post {:.4}), \
         one-sided McNemar p={:.4} < alpha {alpha}",
        m.key, m.losses, m.gains, m.n, m.pre, m.post, m.p_value
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHA: f64 = 0.05;

    /// `n` items `p-0000…`, the first `ok` of them successes.
    fn flags(n: usize, ok: impl Fn(usize) -> bool) -> Vec<ItemFlag> {
        (0..n).map(|i| (format!("p-{i:04}"), ok(i))).collect()
    }

    fn all(v: bool) -> Vec<ItemFlag> {
        flags(SLICE_SIZE, |_| v)
    }

    #[test]
    fn slice_of_classifies_by_name_only() {
        assert_eq!(
            reasoning_slice_of("gsm8k_think"),
            Some(("gsm8k_think", ReasoningMetric::Accuracy))
        );
        assert_eq!(
            reasoning_slice_of("vox_think_init"),
            Some(("vox_think", ReasoningMetric::ThinkInitiation))
        );
        assert_eq!(reasoning_slice_of("code_bench"), None);
        assert_eq!(reasoning_slice_of("foo_init"), None);
    }

    #[test]
    fn five_keys_are_refused_and_six_are_accepted() {
        let five = [
            "gsm8k_think",
            "math500_think",
            "math500_think_init",
            "vox_think",
            "vox_think_init",
        ];
        let err = require_reasoning_keys(five).unwrap_err();
        assert!(err.contains("gsm8k_think_init"), "{err}");
        let six = [
            "gsm8k_think",
            "gsm8k_think_init",
            "math500_think",
            "math500_think_init",
            "vox_think",
            "vox_think_init",
        ];
        assert!(require_reasoning_keys(six).is_ok());
    }

    /// Exact binomial tails, computed by hand: P(X>=k), X~Bin(d,1/2).
    #[test]
    fn mcnemar_p_matches_exact_binomial_tails() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert_eq!(mcnemar_one_sided_p(0, 0), 1.0);
        assert!(close(mcnemar_one_sided_p(0, 7), 1.0));
        assert!(close(mcnemar_one_sided_p(1, 0), 0.5));
        assert!(close(mcnemar_one_sided_p(4, 0), 1.0 / 16.0));
        assert!(close(mcnemar_one_sided_p(5, 0), 1.0 / 32.0));
        // d=8, k>=7: (C(8,7)+C(8,8))/256 = 9/256
        assert!(close(mcnemar_one_sided_p(7, 1), 9.0 / 256.0));
        // d=10, k>=7: (120+45+10+1)/1024 = 176/1024
        assert!(close(mcnemar_one_sided_p(7, 3), 176.0 / 1024.0));
        // symmetric middle: d=2, k>=1 = 3/4
        assert!(close(mcnemar_one_sided_p(1, 1), 0.75));
        // large d stays finite and in range
        let p = mcnemar_one_sided_p(700, 500);
        assert!(p > 0.0 && p < 1e-6, "{p}");
    }

    /// The smallest failing outcomes at alpha 0.05 (RESEARCH §4): 5-0, 7-1, 9-2, 10-3.
    #[test]
    fn the_failing_boundary_at_alpha_005_is_the_documented_one() {
        for (b, c) in [(5, 0), (7, 1), (9, 2), (10, 3)] {
            assert!(mcnemar_one_sided_p(b, c) < ALPHA, "{b}-{c} must fail");
            assert!(
                mcnemar_one_sided_p(b - 1, c) >= ALPHA,
                "{}-{c} must pass",
                b - 1
            );
        }
    }

    #[test]
    fn one_net_lost_item_is_not_a_regression() {
        // base 16/32, adapter 15/32 with one loss failed the old 5% rule; same
        // counts on the grown slice.
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| i < 15);
        let m = paired_metric("gsm8k_think", &base, &adapter, ALPHA).unwrap();
        assert_eq!((m.losses, m.gains), (1, 0));
        assert_eq!((m.pre, m.post), (16.0 / 128.0, 15.0 / 128.0));
        assert!(!m.regressed);
    }

    #[test]
    fn five_losses_and_no_gains_on_32_is_a_regression() {
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| i < 11);
        let m = paired_metric("gsm8k_think", &base, &adapter, ALPHA).unwrap();
        assert_eq!((m.losses, m.gains), (5, 0));
        assert!(m.regressed, "p={}", m.p_value);
    }

    #[test]
    fn gains_offset_losses() {
        // 6 lost, 4 gained: net -2, p = P(Bin(10,.5) >= 6) = 0.377 → no regression.
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| (6..20).contains(&i));
        let m = paired_metric("x", &base, &adapter, ALPHA).unwrap();
        assert_eq!((m.losses, m.gains), (6, 4));
        assert!(!m.regressed);
    }

    /// Every multinomial outcome of `n` items, tested directly.
    fn brute_force_power(n: usize, pl: f64, pg: f64, alpha: f64) -> f64 {
        let fact = |k: usize| (1..=k).map(|x| x as f64).product::<f64>();
        let mut total = 0.0;
        for b in 0..=n {
            for c in 0..=n - b {
                if mcnemar_one_sided_p(b, c) < alpha {
                    let r = n - b - c;
                    total += fact(n) / (fact(b) * fact(c) * fact(r))
                        * pl.powi(b as i32)
                        * pg.powi(c as i32)
                        * (1.0 - pl - pg).powi(r as i32);
                }
            }
        }
        total
    }

    #[test]
    fn power_matches_brute_force_enumeration() {
        for (pl, pg) in [(0.12, 0.08), (0.1, 0.1), (0.3, 0.0), (0.05, 0.2)] {
            let (fast, slow) = (
                mcnemar_power(14, pl, pg, ALPHA),
                brute_force_power(14, pl, pg, ALPHA),
            );
            assert!(
                (fast - slow).abs() < 1e-12,
                "pl={pl} pg={pg}: {fast} vs {slow}"
            );
        }
        assert_eq!(mcnemar_power(14, 0.0, 0.0, ALPHA), 0.0);
    }

    #[test]
    fn power_at_32_items_matches_the_recorded_derivation() {
        // 335-RESEARCH.md, n=32, discordant 0.2, 20% relative drop of 0.5: 0.193.
        let p = mcnemar_power(32, 0.15, 0.05, ALPHA);
        assert!((p - 0.193).abs() < 5e-4, "{p}");
    }

    #[test]
    fn the_false_fail_rate_never_exceeds_alpha() {
        for n in [SLICE_SIZE, 3 * SLICE_SIZE] {
            for pd in POWER_DISCORDANT {
                let size = mcnemar_power(n, pd / 2.0, pd / 2.0, ALPHA);
                assert!(size > 0.0 && size <= ALPHA, "n={n} pd={pd}: {size}");
            }
        }
    }

    #[test]
    fn misaligned_items_are_refused_not_realigned() {
        let base = all(true);
        let mut adapter = all(true);
        adapter.swap(0, 1);
        let e = paired_metric("gsm8k_think", &base, &adapter, ALPHA).unwrap_err();
        assert!(e.contains("order differs"), "{e}");
        let e = paired_metric("gsm8k_think", &base, &adapter[1..], ALPHA).unwrap_err();
        assert!(e.contains(&(SLICE_SIZE - 1).to_string()), "{e}");
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let mut both = all(true);
        both[1].0 = both[0].0.clone();
        let e = paired_metric("gsm8k_think", &both, &both, ALPHA).unwrap_err();
        assert!(e.contains("duplicate"), "{e}");
    }

    #[test]
    fn an_alpha_outside_the_unit_interval_is_refused() {
        for a in [0.0, 1.0, -0.1, f64::NAN, f64::INFINITY] {
            assert!(
                paired_metric("x", &all(true), &all(true), a).is_err(),
                "{a}"
            );
        }
    }

    #[test]
    fn a_think_initiation_regression_alone_fails_on_the_init_key() {
        let acc = all(true);
        let think_adapter = flags(SLICE_SIZE, |i| i >= 6);
        let v = eval_reasoning_slice(
            "vox_think",
            (&acc, &acc),
            (&all(true), &think_adapter),
            ALPHA,
        )
        .unwrap();
        assert!(v.exceeds_threshold());
        assert_eq!(v.failed_key().as_deref(), Some("vox_think_init"));
    }

    #[test]
    fn an_accuracy_regression_fails_on_the_slice_key() {
        let base = all(true);
        let adapter = flags(SLICE_SIZE, |i| i >= 5);
        let v =
            eval_reasoning_slice("gsm8k_think", (&base, &adapter), (&base, &base), ALPHA).unwrap();
        assert_eq!(v.failed_key().as_deref(), Some("gsm8k_think"));
    }

    #[test]
    fn an_item_count_other_than_the_slice_size_is_refused() {
        for n in [0, 32, SLICE_SIZE - 1, SLICE_SIZE + 1] {
            let f = flags(n, |_| true);
            let e = eval_reasoning_slice("gsm8k_think", (&f, &f), (&f, &f), ALPHA).unwrap_err();
            assert!(e.contains(&format!("expected {SLICE_SIZE}")), "n={n}: {e}");
        }
    }

    #[test]
    fn an_unknown_slice_name_is_refused() {
        let f = all(true);
        assert!(eval_reasoning_slice("gsm8k", (&f, &f), (&f, &f), ALPHA).is_err());
    }

    #[test]
    fn accuracy_and_think_must_name_the_same_items() {
        let f = all(true);
        let other: Vec<ItemFlag> = (0..SLICE_SIZE)
            .map(|i| (format!("q-{i:04}"), true))
            .collect();
        let e = eval_reasoning_slice("gsm8k_think", (&f, &f), (&other, &other), ALPHA).unwrap_err();
        assert!(e.contains("different items"), "{e}");
    }

    /// Three slices that each lose 2 of 16 right with no gains: no slice fails on its
    /// own (p = 0.25), the pooled 6-0 does (p = 1/64).
    #[test]
    fn a_small_loss_on_every_slice_fails_the_pooled_test_only() {
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| i < 14);
        let verdicts: Vec<_> = SLICE_NAMES
            .iter()
            .map(|s| {
                eval_reasoning_slice(s, (&base, &adapter), (&all(true), &all(true)), ALPHA).unwrap()
            })
            .collect();
        assert!(verdicts.iter().all(|v| !v.exceeds_threshold()));
        let pooled = pooled_accuracy(&verdicts, ALPHA).unwrap();
        assert_eq!(
            (pooled.n, pooled.losses, pooled.gains),
            (3 * SLICE_SIZE, 6, 0)
        );
        assert_eq!(pooled.pre, 16.0 / SLICE_SIZE as f64);
        assert!(pooled.regressed, "p={}", pooled.p_value);
        assert_eq!(pooled.key, POOLED_KEY);
    }

    fn passing_verdicts() -> Vec<ReasoningSliceVerdict> {
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| i < 15);
        SLICE_NAMES
            .iter()
            .map(|s| {
                eval_reasoning_slice(s, (&base, &adapter), (&all(true), &all(true)), ALPHA).unwrap()
            })
            .collect()
    }

    /// A report as the writer emits it.
    fn good_report() -> serde_json::Value {
        let verdicts = passing_verdicts();
        let pooled = pooled_accuracy(&verdicts, ALPHA).unwrap();
        serde_json::json!({
            "status": "pass",
            "failed_on": null,
            "reports": [],
            "reasoning_slices": reasoning_slices_json(&verdicts, Some(&pooled), ALPHA),
        })
    }

    #[test]
    fn the_report_block_labels_the_rule_alpha_and_slices() {
        let j = &good_report()["reasoning_slices"];
        let note = j["scope_note"].as_str().unwrap();
        assert!(
            note.contains("128-item") && note.contains("not full"),
            "{note}"
        );
        assert_eq!(j["rule"], PAIRED_RULE);
        assert_eq!(j["alpha"], ALPHA);
        assert_eq!(j["slice_size"], SLICE_SIZE);
        assert_eq!(j["pooled"]["n"], 3 * SLICE_SIZE);
        assert_eq!(j["power"]["slice"]["n"], SLICE_SIZE);
        assert_eq!(j["power"]["pooled"]["n"], 3 * SLICE_SIZE);
        assert_eq!(j["power"]["min_losses_with_no_gains"], 5);
        for name in SLICE_NAMES {
            assert_eq!(j["benches"][name]["accuracy"]["losses"], 3 - 2, "{name}");
        }
    }

    #[test]
    fn a_stored_report_in_the_writer_shape_is_accepted() {
        let verdicts = verify_reasoning_slices(&good_report(), ALPHA).unwrap();
        assert_eq!(verdicts.len(), 3);
    }

    fn edited(pointer: &str, value: serde_json::Value) -> String {
        let mut r = good_report();
        *r.pointer_mut(pointer).unwrap() = value;
        verify_reasoning_slices(&r, ALPHA).unwrap_err()
    }

    #[test]
    fn a_stored_pass_over_regressed_counts_is_recomputed_and_refused() {
        let e = edited(
            "/reasoning_slices/benches/gsm8k_think/accuracy/losses",
            serde_json::json!(5),
        );
        assert!(e.contains("gsm8k_think regressed"), "{e}");
    }

    #[test]
    fn a_stored_regressed_think_initiation_is_refused_on_the_init_key() {
        let mut r = good_report();
        let t = "/reasoning_slices/benches/vox_think/think_initiation";
        *r.pointer_mut(&format!("{t}/losses")).unwrap() = serde_json::json!(6);
        let e = verify_reasoning_slices(&r, ALPHA).unwrap_err();
        assert!(e.contains("vox_think_init regressed"), "{e}");
    }

    #[test]
    fn a_pooled_regression_is_recomputed_even_if_stored_as_passing() {
        let mut r = good_report();
        for name in SLICE_NAMES {
            let a = format!("/reasoning_slices/benches/{name}/accuracy/losses");
            *r.pointer_mut(&a).unwrap() = serde_json::json!(2);
        }
        let e = verify_reasoning_slices(&r, ALPHA).unwrap_err();
        assert!(e.contains(POOLED_KEY), "{e}");
    }

    #[test]
    fn a_report_for_another_alpha_or_rule_is_refused() {
        let e = edited("/reasoning_slices/alpha", serde_json::json!(0.01));
        assert!(e.contains("alpha"), "{e}");
        let e = edited("/reasoning_slices/rule", serde_json::json!("5% relative"));
        assert!(e.contains("rule"), "{e}");
        assert!(verify_reasoning_slices(&good_report(), 0.10).is_err());
    }

    #[test]
    fn an_old_relative_rule_report_is_refused() {
        let mut r = good_report();
        r["reasoning_slices"]
            .as_object_mut()
            .unwrap()
            .remove("rule");
        assert!(
            verify_reasoning_slices(&r, ALPHA)
                .unwrap_err()
                .contains("rule")
        );
    }

    #[test]
    fn a_missing_slice_is_refused_by_name() {
        let mut r = good_report();
        r.pointer_mut("/reasoning_slices/benches")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("math500_think");
        let e = verify_reasoning_slices(&r, ALPHA).unwrap_err();
        assert!(e.contains("math500_think"), "{e}");
    }

    #[test]
    fn inconsistent_or_fractional_stored_counts_are_refused() {
        let e = edited(
            "/reasoning_slices/benches/gsm8k_think/accuracy/pre",
            serde_json::json!(0.51),
        );
        assert!(e.contains("whole number"), "{e}");
        let e = edited(
            "/reasoning_slices/benches/gsm8k_think/accuracy/losses",
            serde_json::json!(17),
        );
        assert!(e.contains("inconsistent"), "{e}");
        let e = edited(
            "/reasoning_slices/benches/gsm8k_think/accuracy/n",
            serde_json::json!(31),
        );
        assert!(
            e.contains(&format!("expected {SLICE_SIZE}")) || e.contains("whole number"),
            "{e}"
        );
        let e = edited(
            "/reasoning_slices/benches/vox_think/think_initiation/gains",
            serde_json::Value::Null,
        );
        assert!(e.contains("vox_think_init") && e.contains("gains"), "{e}");
    }

    #[test]
    fn a_missing_scope_note_or_slice_size_is_refused() {
        let e = edited("/reasoning_slices/slice_size", serde_json::json!(31));
        assert!(e.contains("slice_size"), "{e}");
        let mut r = good_report();
        r["reasoning_slices"]
            .as_object_mut()
            .unwrap()
            .remove("scope_note");
        assert!(
            verify_reasoning_slices(&r, ALPHA)
                .unwrap_err()
                .contains("scope_note")
        );
    }
}
