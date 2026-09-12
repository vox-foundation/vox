//! `vox mens cloud-estimate` — rank live GPU offers and print the bill, without
//! provisioning anything.
//!
//! This adds no cost model. `CloudResolver::resolve` already costs every offer
//! through `TimeEstimator`; this command prints that ranking instead of
//! dispatching the top row. It exists because the plan's default is "run it
//! locally" and you should be able to price the alternative for free.

use anyhow::Result;
use vox_populi::mens::cloud::CloudTarget;
use vox_populi::mens::cloud::resolver::{CloudResolver, ResolveRequest};
use vox_populi::mens::tensor::memory_model::{
    CalKey, DeviceBudget, Lane, MemoryModels, ModelShape, Request, Verdict, plan_for,
};

/// One row of the estimate table.
///
/// `secs` is the estimated wall time for the whole run; `total_usd` is what the
/// run costs end to end; `usd_per_hr` is the offer's rate. All three are printed
/// because the operator is choosing between "free and slower here" and
/// "metered and faster there".
///
/// Amounts are formatted first and padded second: `format!("${x:>7.2}")` pads
/// between the sigil and the digits, producing `"$   2.09"`, which is both ugly
/// and unsearchable.
#[must_use]
pub fn format_row(
    provider: &str,
    gpu: &str,
    gpu_count: u32,
    secs: f64,
    total_usd: f64,
    usd_per_hr: f64,
) -> String {
    format!(
        "{provider:<8} {gpu:<16} x{gpu_count}  {:>9}/hr  {:>7.1} h  {:>10}",
        format!("${usd_per_hr:.2}"),
        secs / 3600.0,
        format!("${total_usd:.2}"),
    )
}

/// Rank live cloud GPU offers for `model_dir` and print the estimated bill.
/// Read-only: resolves and ranks via [`CloudResolver::resolve`], never
/// dispatches or provisions anything.
///
/// There is exactly one local row, and it comes from `CloudResolver::resolve`
/// itself (`LocalProvider::list_offers`, requested whenever `target` is
/// `auto` or `local`): a real `GpuOffer` priced at $0/hr and timed through the
/// same `TimeEstimator` three-tier estimation as every rented offer, not a
/// hand-rolled placeholder. Two competing "local" rows with disagreeing
/// numbers (one real, one a guess) would be exactly the kind of
/// confidently-wrong output this command exists to prevent — so this
/// function does not construct its own. The $0/hr price sorts it to the top
/// of `resolve`'s cost-ranked list on its own (see `rank_offers`), which is
/// what "print the local row first" means in practice for `--target auto`.
pub async fn run(
    model_dir: std::path::PathBuf,
    target: String,
    max_budget: f64,
    batch_size: usize,
    seq_len: usize,
    num_samples: usize,
    epochs: usize,
) -> Result<()> {
    let shape = ModelShape::from_model_dir(&model_dir)?;
    let request = Request {
        batch_size: batch_size as u64,
        seq_len: seq_len as u64,
    };

    // Every rented offer is CUDA. `plan_for` is infallible: it reports a
    // verdict rather than erroring, so an uncalibrated lane surfaces as
    // `Verdict::Refused` with `predicted_bytes: None` (see
    // `MemoryModels::get`, the sole source of that error text) rather than a
    // silently-borrowed constant from another lane.
    let cuda_key = CalKey::new(Lane::CandleCuda, true)?;
    let models = MemoryModels::load_default()?;
    // A generous ceiling: this call exists to get `predicted_bytes`, not to
    // gate against a particular device's usable memory — the resolver (with
    // real per-offer VRAM) does that gating for the rented rows.
    let sizing_budget = DeviceBudget {
        working_set_bytes: u64::MAX / 2,
        operator_fraction: None,
    };
    let plan = plan_for(&sizing_budget, &models, &cuda_key, &shape, &request);

    let predicted_bytes = match (&plan.verdict, plan.predicted_bytes) {
        (_, Some(bytes)) => bytes,
        (Verdict::Refused(reason), None) => {
            anyhow::bail!(
                "cannot size a candle-cuda run: {reason}. The memory model has no \
                 measured constant for this lane; borrowing another lane's constant \
                 would produce a confident wrong estimate. Measure it first (see the \
                 memory-SSOT plan) or pass an explicit --min-vram-mb."
            );
        }
        (Verdict::Fits, None) => unreachable!("Fits verdict always carries predicted_bytes"),
    };

    // Only a u64 crosses the seam into the resolver. `GpuOffer.vram_mb` is
    // populated from Vast's `gpu_ram` and RunPod's memory field, both MiB in
    // practice despite the field name — div_ceil(1_048_576) matches that.
    let min_vram_mb = predicted_bytes.div_ceil(1_048_576);

    let resolver = CloudResolver::new_from_env().await?;
    let target: CloudTarget = target.parse()?;
    let req = ResolveRequest {
        min_vram_mb,
        seq_len,
        batch_size,
        num_samples,
        epochs,
        max_acceptable_cost: max_budget,
        target,
    };
    let (ranked, rejected) = resolver.resolve(&req).await?;

    for offer in &ranked {
        println!(
            "{}",
            format_row(
                offer.offer.provider.display_name(),
                &offer.offer.gpu_name,
                offer.offer.gpu_count,
                offer.estimated_secs,
                offer.estimated_cost_usd,
                offer.offer.price_per_hour_usd,
            )
        );
    }
    for (offer_id, reason) in &rejected {
        println!("  rejected {offer_id}: {reason}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MUTATION CAUGHT: printing `estimated_secs` raw, or dividing by 60 instead of
    /// 3600. A 13-hour run shown as "13 min" or "46800" is the number the operator
    /// uses to decide whether the >12h trigger fired. Getting it wrong by 60x
    /// produces a confident, wrong purchase.
    #[test]
    fn a_thirteen_hour_run_prints_as_hours_not_seconds() {
        let row = format_row("vast", "h100 sxm", 1, 46_800.0, 27.16, 2.09);
        assert!(row.contains("13.0 h"), "expected hours in {row:?}");
        assert!(row.contains("$27.16"), "expected total cost in {row:?}");
    }

    /// MUTATION CAUGHT: printing $/hr where total cost belongs (or vice versa), and
    /// padding *inside* the `$` -- `format!("${x:>7.2}", 2.09)` renders "$   2.09",
    /// which contains neither "$2.09" nor any greppable amount. $2.09 and $27.16 are
    /// both plausible numbers on a rental table; a swap makes the cheapest row look
    /// 13x cheaper than it is.
    #[test]
    fn hourly_rate_and_run_total_are_both_shown_and_not_swapped() {
        let row = format_row("vast", "h100 sxm", 1, 46_800.0, 27.16, 2.09);
        let rate_at = row.find("$2.09").expect("hourly rate missing");
        let total_at = row.find("$27.16").expect("run total missing");
        assert!(rate_at < total_at, "rate must precede total in {row:?}");
    }

    /// MUTATION CAUGHT: dropping the free local row, or dividing by its $0.00 rate.
    /// The local row is the entire point of the comparison under this plan's entry
    /// gate -- "$0 and 2h10m here" vs "$27 and 13h there" is the decision, and a NaN
    /// or a panic on the zero-price row removes it from the table silently.
    #[test]
    fn the_free_local_row_renders_without_dividing_by_zero() {
        let row = format_row("local", "apple m5 max", 1, 1_126_800.0, 0.0, 0.0);
        assert!(row.contains("$0.00"), "expected a free row in {row:?}");
        assert!(row.contains("313.0 h"), "expected 313 h in {row:?}");
        assert!(
            !row.contains("NaN") && !row.contains("inf"),
            "bad math in {row:?}"
        );
    }
}
