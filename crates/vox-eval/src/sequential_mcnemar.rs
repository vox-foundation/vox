//! Exact sequential McNemar stopping rule and worst-case false-fail size (THRU-10).
//!
//! # Stopping rule and boundaries
//!
//! Per slice, items are evaluated in the pinned order. At each pre-registered look
//! $k$, the test evaluates the cumulative discordant counts $(b_k, c_k)$ over the
//! first $n_k$ items and rejects (fails the slice) if:
//!
//! ```text
//! mcnemar_one_sided_p(b_k, c_k) < look_alpha[k]
//! ```
//!
//! The rule stops early only for failure (efficacy boundary). Per D-23, futility
//! (early pass) and deterministic curtailment are not implemented. Five unreplied
//! losses already reject at $\alpha = 0.05$, so deterministic curtailment cannot
//! fire until the last few items; and futility changes the nominal level of the
//! pooled test across slices, so it is deferred to user decision.
//!
//! # Statistical model and exact size
//!
//! Under $H_0$ (the adapter is no worse than the base model, $p_l \le p_g$), the
//! directions of discordant items are fair coin flips independent of which items
//! are discordant (the standard premise of the paired McNemar test). Conditioned
//! on any discordant-count path $(d_1 \le d_2 \le \dots \le d_K)$, the conditional
//! rejection probability is determined by partial sums of a fair random walk.
//!
//! A schedule whose conditional rejection probability is at most $\alpha$ for
//! every possible discordant-count path has unconditional size (false-fail rate)
//! at most $\alpha$ for every discordance rate and difficulty profile.
//!
//! Per D-16, looks are derived from fractions times the slice size; this module
//! defines no look counts or alpha values outside its tests.

use crate::reasoning_slice::mcnemar_one_sided_p;

/// Report rule identifier for sequential McNemar, distinct from `PAIRED_RULE`.
pub const SEQUENTIAL_RULE: &str =
    "exact sequential McNemar on paired items, fail when p < look_alpha";

/// Rule ID string used in ladder policy YAML (`collateral_damage.rule`).
pub const SEQUENTIAL_RULE_ID: &str = "sequential_mcnemar_v1";

/// Direction of a paired item comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PairOutcome {
    /// Base model answered correctly, adapter answered incorrectly.
    Loss,
    /// Adapter answered correctly, base model answered incorrectly.
    Gain,
    /// Both models produced equivalent correctness (both correct or both incorrect).
    Tie,
}

impl PairOutcome {
    /// Pair two boolean flags (true = correct / initiated think).
    pub fn from_flags(base_ok: bool, adapter_ok: bool) -> Self {
        match (base_ok, adapter_ok) {
            (true, false) => Self::Loss,
            (false, true) => Self::Gain,
            _ => Self::Tie,
        }
    }
}

/// A pre-registered sequential McNemar stopping schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct SequentialRule {
    looks: Vec<usize>,
    look_alpha: Vec<f64>,
    alpha: f64,
}

impl SequentialRule {
    /// Construct and structurally validate a sequential schedule.
    pub fn new(looks: Vec<usize>, look_alpha: Vec<f64>, alpha: f64) -> Result<Self, String> {
        if alpha <= 0.0 || alpha >= 1.0 || alpha.is_nan() {
            return Err(format!("alpha must be in (0, 1), got {alpha}"));
        }
        if looks.is_empty() {
            return Err("looks must not be empty".to_string());
        }
        if looks.len() != look_alpha.len() {
            return Err(format!(
                "looks length ({}) does not match look_alpha length ({})",
                looks.len(),
                look_alpha.len()
            ));
        }
        if looks[0] == 0 {
            return Err("first look must be greater than 0".to_string());
        }
        for w in looks.windows(2) {
            if w[0] >= w[1] {
                return Err(format!(
                    "looks must be strictly increasing, got {} followed by {}",
                    w[0], w[1]
                ));
            }
        }
        for (i, &a) in look_alpha.iter().enumerate() {
            if a <= 0.0 || a > alpha || a.is_nan() {
                return Err(format!(
                    "look_alpha[{i}] ({a}) must be in (0, alpha ({alpha})]"
                ));
            }
        }
        Ok(Self {
            looks,
            look_alpha,
            alpha,
        })
    }

    /// Construct a rule from fractions of a slice size `n` (`looks[k] = ceil(fraction[k] * n)`).
    pub fn from_fractions(
        looks_fraction: &[f64],
        look_alpha: Vec<f64>,
        alpha: f64,
        n: usize,
    ) -> Result<Self, String> {
        if n == 0 {
            return Err("n must be greater than 0".to_string());
        }
        if looks_fraction.is_empty() {
            return Err("looks_fraction must not be empty".to_string());
        }
        for (i, &f) in looks_fraction.iter().enumerate() {
            if f <= 0.0 || f > 1.0 || f.is_nan() {
                return Err(format!("fraction[{i}] ({f}) must be in (0, 1]"));
            }
        }
        for w in looks_fraction.windows(2) {
            if w[0] >= w[1] {
                return Err(format!(
                    "fractions must be strictly increasing, got {} followed by {}",
                    w[0], w[1]
                ));
            }
        }
        let last = *looks_fraction.last().unwrap();
        if (last - 1.0).abs() > 1e-9 {
            return Err(format!("last fraction must be 1.0, got {last}"));
        }
        let looks = looks_fraction
            .iter()
            .map(|&f| (f * n as f64).ceil() as usize)
            .collect::<Vec<_>>();
        Self::new(looks, look_alpha, alpha)
    }

    /// Pinned item counts at which looks are evaluated.
    pub fn looks(&self) -> &[usize] {
        &self.looks
    }

    /// Per-look nominal rejection levels.
    pub fn look_alpha(&self) -> &[f64] {
        &self.look_alpha
    }

    /// Overall targeted significance level.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// Check whether `losses` and `gains` at look index `look` reject the slice.
    pub fn rejects_at(&self, look: usize, losses: usize, gains: usize) -> bool {
        if look >= self.look_alpha.len() {
            return false;
        }
        mcnemar_one_sided_p(losses, gains) < self.look_alpha[look]
    }

    /// Judge a sequence of pinned-order pair outcomes.
    ///
    /// Evaluates each look whose item count $n_k$ is covered by `outcomes.len()`,
    /// stopping at the first look that rejects. Items beyond a rejecting look
    /// do not alter the verdict.
    pub fn judge(&self, outcomes: &[PairOutcome]) -> SequentialVerdict {
        let mut records = Vec::new();
        let mut stopped_at_look = None;
        let mut items_used = outcomes.len();

        for (idx, &n_k) in self.looks.iter().enumerate() {
            if n_k > outcomes.len() {
                break;
            }
            let prefix = &outcomes[..n_k];
            let mut losses = 0;
            let mut gains = 0;
            for outcome in prefix {
                match outcome {
                    PairOutcome::Loss => losses += 1,
                    PairOutcome::Gain => gains += 1,
                    PairOutcome::Tie => {}
                }
            }
            let p = mcnemar_one_sided_p(losses, gains);
            let look_alpha = self.look_alpha[idx];
            let rejected = p < look_alpha;
            records.push(LookRecord {
                n: n_k,
                losses,
                gains,
                p,
                look_alpha,
                rejected,
            });
            if rejected {
                stopped_at_look = Some(idx + 1); // 1-based index
                items_used = n_k;
                break;
            }
        }

        let complete = if stopped_at_look.is_some() {
            true
        } else {
            self.looks
                .last()
                .is_none_or(|&last_n| outcomes.len() >= last_n)
        };

        SequentialVerdict {
            looks: records,
            stopped_at_look,
            items_used,
            complete,
        }
    }

    /// Judge both accuracy and think-initiation metrics look by look on the same prefix.
    ///
    /// The slice stops at the first look where either metric rejects.
    /// `items_used` is that look's $n$, or the full length when complete.
    pub fn judge_slice(
        &self,
        slice: &str,
        accuracy: &[PairOutcome],
        think: &[PairOutcome],
    ) -> SequentialSliceVerdict {
        let mut acc_looks = Vec::new();
        let mut think_looks = Vec::new();
        let mut stopped_at_look = None;
        let min_len = accuracy.len().min(think.len());
        let mut items_used = min_len;

        for (idx, &n_k) in self.looks.iter().enumerate() {
            if n_k > min_len {
                break;
            }
            let mut acc_losses = 0;
            let mut acc_gains = 0;
            for outcome in &accuracy[..n_k] {
                match outcome {
                    PairOutcome::Loss => acc_losses += 1,
                    PairOutcome::Gain => acc_gains += 1,
                    PairOutcome::Tie => {}
                }
            }
            let acc_p = mcnemar_one_sided_p(acc_losses, acc_gains);
            let look_alpha = self.look_alpha[idx];
            let acc_rejected = acc_p < look_alpha;
            acc_looks.push(LookRecord {
                n: n_k,
                losses: acc_losses,
                gains: acc_gains,
                p: acc_p,
                look_alpha,
                rejected: acc_rejected,
            });

            let mut think_losses = 0;
            let mut think_gains = 0;
            for outcome in &think[..n_k] {
                match outcome {
                    PairOutcome::Loss => think_losses += 1,
                    PairOutcome::Gain => think_gains += 1,
                    PairOutcome::Tie => {}
                }
            }
            let think_p = mcnemar_one_sided_p(think_losses, think_gains);
            let think_rejected = think_p < look_alpha;
            think_looks.push(LookRecord {
                n: n_k,
                losses: think_losses,
                gains: think_gains,
                p: think_p,
                look_alpha,
                rejected: think_rejected,
            });

            if acc_rejected || think_rejected {
                stopped_at_look = Some(idx + 1); // 1-based index
                items_used = n_k;
                break;
            }
        }

        let acc_stopped = acc_looks.iter().position(|r| r.rejected).map(|i| i + 1);
        let think_stopped = think_looks.iter().position(|r| r.rejected).map(|i| i + 1);

        let acc_verdict = SequentialVerdict {
            looks: acc_looks,
            stopped_at_look: acc_stopped,
            items_used,
            complete: stopped_at_look.is_some() || self.looks.last().is_none_or(|&l| min_len >= l),
        };
        let think_verdict = SequentialVerdict {
            looks: think_looks,
            stopped_at_look: think_stopped,
            items_used,
            complete: stopped_at_look.is_some() || self.looks.last().is_none_or(|&l| min_len >= l),
        };

        SequentialSliceVerdict {
            slice: slice.to_string(),
            accuracy: acc_verdict,
            think_initiation: think_verdict,
            stopped_at_look,
            items_used,
        }
    }

    /// Exact worst-case false-fail probability (size) over every possible
    /// discordant-count path $(d_1 \le d_2 \le \dots \le d_K)$.
    #[allow(clippy::needless_range_loop)]
    pub fn worst_case_size(&self) -> f64 {
        if self.looks.is_empty() {
            return 0.0;
        }
        let k_looks = self.looks.len();
        let mut deltas = Vec::with_capacity(k_looks);
        let mut prev = 0;
        for &look in &self.looks {
            deltas.push(look - prev);
            prev = look;
        }

        let max_delta = deltas.iter().copied().max().unwrap_or(0);

        // Precompute Binomial(e, 0.5) PMF table
        let mut binom_pmf = vec![Vec::new(); max_delta + 1];
        for e in 0..=max_delta {
            let mut row = vec![0.0f64; e + 1];
            if e == 0 {
                row[0] = 1.0;
            } else {
                let p0 = 0.5f64.powi(e as i32);
                row[0] = p0;
                for j in 1..=e {
                    row[j] = row[j - 1] * (e - j + 1) as f64 / j as f64;
                }
            }
            binom_pmf[e] = row;
        }

        // Precompute Binomial(e, 0.5) tail table: P(Bin(e, 0.5) >= m)
        let mut binom_tail = vec![Vec::new(); max_delta + 1];
        for e in 0..=max_delta {
            let mut row = vec![0.0f64; e + 2];
            let mut cum = 0.0;
            row[e + 1] = 0.0;
            for j in (0..=e).rev() {
                cum += binom_pmf[e][j];
                row[j] = cum.min(1.0);
            }
            binom_tail[e] = row;
        }

        // Precompute critical losses table for each look and discordant count
        let mut crit_table = Vec::with_capacity(k_looks);
        for k in 0..k_looks {
            let n_k = self.looks[k];
            let alpha_k = self.look_alpha[k];
            let mut row = Vec::with_capacity(n_k + 1);
            for d in 0..=n_k {
                row.push(critical_losses(d, alpha_k));
            }
            crit_table.push(row);
        }

        fn dfs(
            look_idx: usize,
            d: usize,
            surv: &[f64],
            k_looks: usize,
            deltas: &[usize],
            crit_table: &[Vec<usize>],
            binom_pmf: &[Vec<f64>],
            binom_tail: &[Vec<f64>],
        ) -> f64 {
            let delta = deltas[look_idx];
            if look_idx == k_looks - 1 {
                let mut max_final_rej = 0.0f64;
                for e in 0..=delta {
                    let d_final = d + e;
                    let c = crit_table[look_idx][d_final];
                    let mut rej_e = 0.0f64;
                    for (i, &p_surv) in surv.iter().enumerate() {
                        if p_surv > 0.0 {
                            let needed = c.saturating_sub(i);
                            let tail_p = if needed <= e {
                                binom_tail[e][needed]
                            } else {
                                0.0
                            };
                            rej_e += p_surv * tail_p;
                        }
                    }
                    if rej_e > max_final_rej {
                        max_final_rej = rej_e;
                    }
                }
                return max_final_rej;
            }

            let mut max_rej = 0.0f64;
            let mut new_surv = vec![0.0f64; d + delta + 1];

            for e in 0..=delta {
                let d_next = d + e;
                let c = crit_table[look_idx][d_next];
                let pmf = &binom_pmf[e];

                new_surv[..=d_next].fill(0.0);
                for (s, &p_surv) in surv.iter().enumerate() {
                    if p_surv > 0.0 {
                        for (j, &p_j) in pmf.iter().enumerate() {
                            new_surv[s + j] += p_surv * p_j;
                        }
                    }
                }

                let c_bound = c.min(d_next + 1);
                let mut delta_rej = 0.0f64;
                for s in c_bound..=d_next {
                    delta_rej += new_surv[s];
                }

                let surv_slice = &new_surv[..c_bound];
                let surv_sum: f64 = surv_slice.iter().sum();

                let path_rej = if surv_sum <= 1e-15 {
                    delta_rej
                } else {
                    delta_rej
                        + dfs(
                            look_idx + 1,
                            d_next,
                            surv_slice,
                            k_looks,
                            deltas,
                            crit_table,
                            binom_pmf,
                            binom_tail,
                        )
                };

                if path_rej > max_rej {
                    max_rej = path_rej;
                }
            }

            max_rej
        }

        let initial_surv = [1.0f64];
        dfs(
            0,
            0,
            &initial_surv,
            k_looks,
            &deltas,
            &crit_table,
            &binom_pmf,
            &binom_tail,
        )
    }

    /// Validate that the schedule's worst-case size does not exceed `alpha`.
    pub fn check_size(&self) -> Result<f64, String> {
        let size = self.worst_case_size();
        if size <= self.alpha {
            Ok(size)
        } else {
            Err(format!(
                "sequential schedule worst-case size ({size:.6}) exceeds alpha ({:.6})",
                self.alpha
            ))
        }
    }
}

/// Compute the smallest loss count $b \in [0, d]$ with `mcnemar_one_sided_p(b, d - b) < level`.
///
/// Returns $d + 1$ when no loss count rejects at `level`.
pub fn critical_losses(discordant: usize, level: f64) -> usize {
    if discordant == 0 {
        return 1;
    }
    for b in 0..=discordant {
        if mcnemar_one_sided_p(b, discordant - b) < level {
            return b;
        }
    }
    discordant + 1
}

/// Record of an evaluation at a single look.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LookRecord {
    pub n: usize,
    pub losses: usize,
    pub gains: usize,
    pub p: f64,
    pub look_alpha: f64,
    pub rejected: bool,
}

/// Outcome of judging paired items against a sequential stopping rule.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SequentialVerdict {
    pub looks: Vec<LookRecord>,
    pub stopped_at_look: Option<usize>, /* 1-based */
    pub items_used: usize,
    pub complete: bool,
}

/// Verdict for one reasoning slice judged under a sequential McNemar rule.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SequentialSliceVerdict {
    pub slice: String,
    pub accuracy: SequentialVerdict,
    pub think_initiation: SequentialVerdict,
    pub stopped_at_look: Option<usize>,
    pub items_used: usize,
}

impl SequentialSliceVerdict {
    pub fn rejected(&self) -> bool {
        self.stopped_at_look.is_some()
    }
}

/// The `reasoning_slices` block of a collateral report under a sequential McNemar rule.
pub fn sequential_slices_json(
    verdicts: &[SequentialSliceVerdict],
    pooled: Option<&crate::reasoning_slice::PairedMetric>,
    rule: &SequentialRule,
) -> serde_json::Value {
    let mut benches = serde_json::Map::new();
    for v in verdicts {
        benches.insert(
            v.slice.clone(),
            serde_json::json!({
                "stopped_at_look": v.stopped_at_look,
                "items_used": v.items_used,
                "accuracy": serde_json::to_value(&v.accuracy).unwrap(),
                "think_initiation": serde_json::to_value(&v.think_initiation).unwrap(),
            }),
        );
    }
    serde_json::json!({
        "scope_note": crate::reasoning_slice::REASONING_SLICE_SCOPE_NOTE,
        "rule": SEQUENTIAL_RULE,
        "alpha": rule.alpha(),
        "slice_size": crate::reasoning_bench::SLICE_SIZE,
        "sequential": {
            "looks": rule.looks(),
            "look_alpha": rule.look_alpha(),
        },
        "benches": benches,
        "pooled": pooled.map(crate::reasoning_slice::PairedMetric::to_json),
        "power": crate::reasoning_slice::power_json(rule.alpha()),
    })
}

/// Re-validate a stored sequential reasoning report against policy `alpha` without
/// trusting its verdict fields.
///
/// Recomputes each look's `p` from counts and checks that `stopped_at_look` matches the
/// first rejecting look (1-based), rejecting any regression as `Err`. On a pass,
/// returns reconstructed `Vec<ReasoningSliceVerdict>`.
pub fn verify_sequential_block(
    block: &serde_json::Value,
    alpha: f64,
) -> Result<Vec<crate::reasoning_slice::ReasoningSliceVerdict>, String> {
    crate::reasoning_slice::check_alpha(alpha)?;
    let stored_alpha = block
        .get("alpha")
        .and_then(serde_json::Value::as_f64)
        .ok_or("reasoning_slices has no numeric alpha")?;
    if (stored_alpha - alpha).abs() > 1e-9 {
        return Err(format!(
            "stored alpha {stored_alpha} is not the policy alpha {alpha}"
        ));
    }
    if block.get("scope_note").and_then(serde_json::Value::as_str)
        != Some(crate::reasoning_slice::REASONING_SLICE_SCOPE_NOTE)
    {
        return Err("reasoning_slices.scope_note is missing or not the expected label".into());
    }
    if block.get("slice_size").and_then(serde_json::Value::as_u64)
        != Some(crate::reasoning_bench::SLICE_SIZE as u64)
    {
        return Err(format!(
            "reasoning_slices.slice_size is missing or not {}",
            crate::reasoning_bench::SLICE_SIZE
        ));
    }

    let seq_obj = block
        .get("sequential")
        .filter(|v| v.is_object())
        .ok_or("reasoning_slices.sequential is missing or not an object")?;
    let looks: Vec<usize> = seq_obj
        .get("looks")
        .and_then(serde_json::Value::as_array)
        .ok_or("reasoning_slices.sequential.looks is missing or not an array")?
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|u| usize::try_from(u).ok())
                .ok_or_else(|| "look item count is missing or not an integer".to_string())
        })
        .collect::<Result<_, _>>()?;
    let look_alpha: Vec<f64> = seq_obj
        .get("look_alpha")
        .and_then(serde_json::Value::as_array)
        .ok_or("reasoning_slices.sequential.look_alpha is missing or not an array")?
        .iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| "look_alpha element is missing or not a float".to_string())
        })
        .collect::<Result<_, _>>()?;

    let rule = SequentialRule::new(looks, look_alpha, alpha)?;
    rule.check_size()?;

    let benches = block
        .get("benches")
        .filter(|b| b.is_object())
        .ok_or("reasoning_slices.benches is missing")?;

    let mut reconstructed_verdicts = Vec::with_capacity(crate::reasoning_bench::SLICE_NAMES.len());

    for name in crate::reasoning_bench::SLICE_NAMES {
        let bench = benches
            .get(name)
            .ok_or_else(|| format!("reasoning_slices.benches is missing {name}"))?;

        let stored_stopped = bench.get("stopped_at_look").and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_u64().map(|u| u as usize)
            }
        });
        let stored_items = bench
            .get("items_used")
            .and_then(serde_json::Value::as_u64)
            .map(|u| u as usize)
            .ok_or_else(|| format!("{name}: missing or invalid items_used"))?;

        let parse_looks =
            |metric_name: &str| -> Result<Vec<(usize, usize, usize, f64, bool)>, String> {
                let m_obj = bench
                    .get(metric_name)
                    .ok_or_else(|| format!("{name}: missing metric {metric_name}"))?;
                let looks_arr = m_obj
                    .get("looks")
                    .and_then(serde_json::Value::as_array)
                    .ok_or_else(|| format!("{name}.{metric_name}: missing looks array"))?;
                let mut parsed = Vec::with_capacity(looks_arr.len());
                for (idx, look_val) in looks_arr.iter().enumerate() {
                    let n = look_val
                        .get("n")
                        .and_then(serde_json::Value::as_u64)
                        .map(|u| u as usize)
                        .ok_or_else(|| format!("{name}.{metric_name}[{idx}]: missing n"))?;
                    let losses = look_val
                        .get("losses")
                        .and_then(serde_json::Value::as_u64)
                        .map(|u| u as usize)
                        .ok_or_else(|| format!("{name}.{metric_name}[{idx}]: missing losses"))?;
                    let gains = look_val
                        .get("gains")
                        .and_then(serde_json::Value::as_u64)
                        .map(|u| u as usize)
                        .ok_or_else(|| format!("{name}.{metric_name}[{idx}]: missing gains"))?;
                    let recomputed_p = mcnemar_one_sided_p(losses, gains);
                    let look_a = rule.look_alpha.get(idx).copied().ok_or_else(|| {
                        format!("{name}.{metric_name}[{idx}]: look index out of range")
                    })?;
                    let rejected = recomputed_p < look_a;
                    parsed.push((n, losses, gains, recomputed_p, rejected));
                }
                Ok(parsed)
            };

        let acc_looks = parse_looks("accuracy")?;
        let think_looks = parse_looks("think_initiation")?;

        let mut first_rej_look = None;
        for idx in 0..acc_looks.len().min(think_looks.len()) {
            if acc_looks[idx].4 || think_looks[idx].4 {
                first_rej_look = Some(idx + 1); // 1-based index
                break;
            }
        }

        if stored_stopped != first_rej_look {
            return Err(format!(
                "{name}: stored stopped_at_look {:?} does not match recomputed {:?}",
                stored_stopped, first_rej_look
            ));
        }

        let expected_items = match first_rej_look {
            Some(idx) => rule.looks[idx - 1],
            None => *rule
                .looks
                .last()
                .unwrap_or(&crate::reasoning_bench::SLICE_SIZE),
        };
        if stored_items != expected_items {
            return Err(format!(
                "{name}: stored items_used {stored_items} does not match expected {expected_items}"
            ));
        }

        if let Some(rej_idx) = first_rej_look {
            return Err(format!("{name}: slice regressed at look {rej_idx}"));
        }

        // On a pass, rebuild ReasoningSliceVerdict from the last look counts
        let last_acc = acc_looks
            .last()
            .ok_or_else(|| format!("{name}: accuracy has no looks"))?;
        let last_think = think_looks
            .last()
            .ok_or_else(|| format!("{name}: think_initiation has no looks"))?;

        if last_acc.0 != crate::reasoning_bench::SLICE_SIZE {
            return Err(format!(
                "{name}: last look n={} is not expected {}",
                last_acc.0,
                crate::reasoning_bench::SLICE_SIZE
            ));
        }

        let acc_n = last_acc.0;
        let acc_losses = last_acc.1;
        let acc_gains = last_acc.2;
        let acc_base_ok = (acc_n - acc_gains).max(acc_losses);
        let last_look_alpha = *rule.look_alpha.last().unwrap_or(&alpha);

        let accuracy_metric = crate::reasoning_slice::PairedMetric::from_counts(
            name,
            acc_n,
            acc_base_ok,
            acc_losses,
            acc_gains,
            last_look_alpha,
        )?;

        let init_key = format!("{name}{}", crate::reasoning_slice::THINK_INIT_SUFFIX);
        let think_n = last_think.0;
        let think_losses = last_think.1;
        let think_gains = last_think.2;
        let think_base_ok = (think_n - think_gains).max(think_losses);

        let think_metric = crate::reasoning_slice::PairedMetric::from_counts(
            &init_key,
            think_n,
            think_base_ok,
            think_losses,
            think_gains,
            last_look_alpha,
        )?;

        reconstructed_verdicts.push(crate::reasoning_slice::ReasoningSliceVerdict {
            slice: name,
            n: crate::reasoning_bench::SLICE_SIZE,
            accuracy: accuracy_metric,
            think_initiation: think_metric,
        });
    }

    let pooled = crate::reasoning_slice::pooled_accuracy(&reconstructed_verdicts, alpha)?;
    if pooled.regressed {
        return Err(crate::reasoning_slice::regression_message(&pooled, alpha));
    }

    Ok(reconstructed_verdicts)
}

#[cfg(test)]
#[path = "sequential_mcnemar_tests.rs"]
mod tests;
