//! Per-spoke behavioral and functional evaluation runners (`338-07`, `338-08`).
//!
//! Produces [`vox_eval::suite_report::SuiteReport`] artifacts consumed by `vox mens eval-gate`.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Strict JSON-Schema argument validator for agent episodes (`D-08`).
#[allow(dead_code)]
pub fn schema_validator(args: &Value, schema: &Value) -> std::result::Result<(), String> {
    vox_eval::episode::toy_validate_schema(args, schema)
}

/// Run the scripted `agent_episodes` suite (`episodes.v1.jsonl` against `tool-catalog.v1.json`)
/// for paired `base` and `candidate` policies and return a [`vox_eval::suite_report::SuiteReport`].
#[allow(dead_code)]
pub fn run_agent_suite(
    suite_name: &str,
    catalog_path: &Path,
    episodes_path: &Path,
    base_name: &str,
    candidate_name: &str,
    mut base_policy: impl FnMut(
        &vox_eval::episode::Episode,
        u64,
    ) -> vox_eval::episode_controls::BoxedPolicy<'static>,
    mut candidate_policy: impl FnMut(
        &vox_eval::episode::Episode,
        u64,
    ) -> vox_eval::episode_controls::BoxedPolicy<'static>,
    seed: u64,
) -> Result<(
    vox_eval::suite_report::SuiteReport,
    vox_eval::suite_report::SuiteReport,
)> {
    let cat_raw = std::fs::read_to_string(catalog_path)
        .with_context(|| format!("reading catalog {}", catalog_path.display()))?;
    let cat_val: Value = serde_json::from_str(&cat_raw)
        .with_context(|| format!("parsing catalog {}", catalog_path.display()))?;
    let catalog = vox_eval::episode::load_catalog(&cat_val)
        .map_err(|e| anyhow::anyhow!("loading catalog: {e}"))?;

    let ep_raw = std::fs::read_to_string(episodes_path)
        .with_context(|| format!("reading episodes {}", episodes_path.display()))?;
    let episodes = vox_eval::episode::load_episodes(&ep_raw, &catalog)
        .map_err(|e| anyhow::anyhow!("loading episodes: {e}"))?;

    let mut base_items = Vec::with_capacity(episodes.len());
    let mut cand_items = Vec::with_capacity(episodes.len());
    for ep in &episodes {
        let mut base_pol = base_policy(ep, seed);
        let base_out =
            vox_eval::episode::run_episode(ep, &catalog, &mut *base_pol, &schema_validator, seed);
        let base_sc = vox_eval::episode::score_episode(ep, &base_out);

        let mut cand_pol = candidate_policy(ep, seed);
        let cand_out =
            vox_eval::episode::run_episode(ep, &catalog, &mut *cand_pol, &schema_validator, seed);
        let cand_sc = vox_eval::episode::score_episode(ep, &cand_out);

        let group = ep.behavior.as_str().to_string();
        base_items.push(vox_eval::suite_report::SuiteItem {
            id: ep.id.clone(),
            group: group.clone(),
            passed: base_sc.passed,
            reason: base_sc.reason,
        });
        cand_items.push(vox_eval::suite_report::SuiteItem {
            id: ep.id.clone(),
            group,
            passed: cand_sc.passed,
            reason: cand_sc.reason,
        });
    }

    Ok((
        vox_eval::suite_report::SuiteReport::from_episode_scores(
            suite_name,
            base_name,
            base_items,
            Value::Null,
            BTreeMap::new(),
        ),
        vox_eval::suite_report::SuiteReport::from_episode_scores(
            suite_name,
            candidate_name,
            cand_items,
            Value::Null,
            BTreeMap::new(),
        ),
    ))
}

/// Evaluate `rust_bench` compile-and-test items for paired base and candidate solutions and return
/// paired `(base, candidate)` [`vox_eval::suite_report::SuiteReport`] artifacts. Refuses `training_eligible: true` items.
#[allow(dead_code)]
pub fn run_rust_bench_suite(
    suite_name: &str,
    items_path: &Path,
    base_name: &str,
    candidate_name: &str,
    base_solutions: &BTreeMap<String, String>,
    candidate_solutions: &BTreeMap<String, String>,
    opts: &vox_corpus::corpus::rust_bench::RunOpts,
) -> Result<(
    vox_eval::suite_report::SuiteReport,
    vox_eval::suite_report::SuiteReport,
)> {
    let items = vox_corpus::corpus::rust_bench::load_items(items_path)?;
    let mut base_items = Vec::with_capacity(items.len());
    let mut cand_items = Vec::with_capacity(items.len());
    for item in &items {
        let base_pass = match base_solutions.get(&item.id) {
            Some(code) => vox_corpus::corpus::rust_bench::run_item(item, code, opts)?.is_pass(),
            None => false,
        };
        let cand_pass = match candidate_solutions.get(&item.id) {
            Some(code) => vox_corpus::corpus::rust_bench::run_item(item, code, opts)?.is_pass(),
            None => false,
        };
        base_items.push(vox_eval::suite_report::SuiteItem {
            id: item.id.clone(),
            group: "rust_bench".to_string(),
            passed: base_pass,
            reason: String::new(),
        });
        cand_items.push(vox_eval::suite_report::SuiteItem {
            id: item.id.clone(),
            group: "rust_bench".to_string(),
            passed: cand_pass,
            reason: String::new(),
        });
    }
    Ok((
        vox_eval::suite_report::SuiteReport::from_episode_scores(
            suite_name,
            base_name,
            base_items,
            Value::Null,
            BTreeMap::new(),
        ),
        vox_eval::suite_report::SuiteReport::from_episode_scores(
            suite_name,
            candidate_name,
            cand_items,
            Value::Null,
            BTreeMap::new(),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn agent_suite_reports_oracle_gain_over_never_call() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cat = root.join("mens/data/agent_episodes/tool-catalog.v1.json");
        let eps = root.join("mens/data/agent_episodes/episodes.v1.jsonl");
        let (base, cand) = run_agent_suite(
            "agent_episodes",
            &cat,
            &eps,
            "never_call",
            "oracle",
            |ep, seed| {
                let _ = (ep, seed);
                vox_eval::episode_controls::never_call(ep, seed)
            },
            |ep, seed| {
                let reference = ep.reference.clone();
                let id = ep.id.clone();
                let mut turn = 0usize;
                Box::new(move |_msgs, _tools| {
                    let reply = reference
                        .get(turn)
                        .map(|r| vox_eval::episode::resolve_reference_turn(r, seed, &id))
                        .unwrap_or_else(|| "Done.".to_string());
                    turn += 1;
                    Ok(reply)
                })
            },
            42,
        )
        .expect("run_agent_suite");
        assert_eq!(cand.n, 72);
        assert!((cand.pass_rate - 1.0).abs() < f64::EPSILON);
        assert!(cand.pass_rate > base.pass_rate);
        let verdict = vox_eval::suite_report::suite_verdict(
            &cand,
            Some(&base),
            &vox_eval::suite_report::SuiteFloors {
                min_pass_rate: 0.70,
                min_group_pass_rate: Some(0.50),
                alpha: 0.05,
            },
        );
        assert!(verdict.passed, "verdict failed: {:?}", verdict.reasons);
    }
}
