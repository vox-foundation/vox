//! Eval gate policy schema and validation.

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

use vox_bounded_fs::read_utf8_path_capped;

/// Eval gate policy schema (from eval-gates.yaml / eval-gates-ladder.yaml).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalGatePolicy {
    #[serde(default)]
    pub supervised_ratio: SupervisedRatioGate,
    #[serde(default)]
    pub pass_at_k: PassAtKGate,
    #[serde(default)]
    pub anti_stub: AntiStubGate,
    #[serde(default)]
    pub collateral_damage: CollateralDamageGate,
}

/// Stopping rule for collateral damage evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CollateralRule {
    #[default]
    FixedN,
    SequentialMcnemarV1,
}

/// Pre-registered sequential stopping rule configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SequentialBlock {
    pub looks_fraction: Vec<f64>,
    pub look_alpha: Vec<f64>,
}

/// Pinned slice entry under `collateral_damage.slices`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SlicePin {
    pub blake3: String,
    pub prefix_n: usize,
    pub prefix_blake3: String,
}

impl SlicePin {
    pub fn verify(&self, slice_bytes: &[u8]) -> Result<(), String> {
        let digest = vox_crypto::hex_encode(&vox_crypto::secure_hash(slice_bytes));
        if digest != self.blake3 {
            return Err(format!(
                "slice BLAKE3 mismatch: expected {}, got {}",
                self.blake3, digest
            ));
        }
        let prefix_bytes = slice_bytes
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .take(self.prefix_n)
            .fold(Vec::new(), |mut acc, line| {
                acc.extend_from_slice(line);
                acc.push(b'\n');
                acc
            });
        let prefix_digest = vox_crypto::hex_encode(&vox_crypto::secure_hash(&prefix_bytes));
        if prefix_digest != self.prefix_blake3 {
            return Err(format!(
                "prefix BLAKE3 mismatch: expected {}, got {}",
                self.prefix_blake3, prefix_digest
            ));
        }
        Ok(())
    }
}

/// Collateral damage gate configuration.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CollateralDamageGate {
    #[serde(default)]
    pub alpha: Option<f64>,
    #[serde(default)]
    pub rule: CollateralRule,
    #[serde(default)]
    pub sequential: Option<SequentialBlock>,
    #[serde(default)]
    pub slices: BTreeMap<String, SlicePin>,
    #[serde(default)]
    pub block: bool,
    #[serde(default)]
    pub require_report: Option<bool>,
}

impl CollateralDamageGate {
    pub fn sequential_rule(&self) -> Result<Option<vox_eval::sequential_mcnemar::SequentialRule>> {
        match self.rule {
            CollateralRule::FixedN => Ok(None),
            CollateralRule::SequentialMcnemarV1 => {
                let block = self.sequential.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("rule sequential_mcnemar_v1 requires a 'sequential' block")
                })?;
                let alpha = self.alpha.unwrap_or(0.05);
                let rule = vox_eval::sequential_mcnemar::SequentialRule::from_fractions(
                    &block.looks_fraction,
                    block.look_alpha.clone(),
                    alpha,
                    vox_eval::reasoning_bench::SLICE_SIZE,
                )
                .map_err(|e| anyhow::anyhow!(e))?;
                Ok(Some(rule))
            }
        }
    }

    pub fn to_requirement(&self) -> Result<vox_eval::collateral_evidence::CollateralRequirement> {
        let alpha = self
            .alpha
            .ok_or_else(|| anyhow::anyhow!("collateral_damage gate requires alpha"))?;
        let rule = match self.rule {
            CollateralRule::FixedN => vox_eval::reasoning_slice::PAIRED_RULE.to_string(),
            CollateralRule::SequentialMcnemarV1 => {
                vox_eval::sequential_mcnemar::SEQUENTIAL_RULE_ID.to_string()
            }
        };
        let slices: std::collections::HashMap<String, String> = self
            .slices
            .iter()
            .map(|(k, v)| (k.clone(), v.blake3.clone()))
            .collect();
        Ok(vox_eval::collateral_evidence::CollateralRequirement {
            slices,
            rule,
            alpha,
            grader: vox_eval::qwen_reasoning::GRADER_ID.to_string(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassAtKGate {
    #[serde(default)]
    pub min_pass_rate_at_1: f64,
    #[serde(default)]
    pub min_pass_rate_at_k: f64,
    #[serde(default = "default_passatk_max_drop")]
    pub max_regression_drop: f64,
    #[serde(default)]
    pub block: bool,
    #[serde(default = "default_passatk_metrics_file")]
    pub metrics_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_file: Option<String>,
}

impl Default for PassAtKGate {
    fn default() -> Self {
        Self {
            min_pass_rate_at_1: 0.0,
            min_pass_rate_at_k: 0.0,
            max_regression_drop: default_passatk_max_drop(),
            block: false,
            metrics_file: default_passatk_metrics_file(),
            baseline_file: None,
        }
    }
}

fn default_passatk_metrics_file() -> String {
    "benchmark_passatk.json".to_string()
}

fn default_passatk_max_drop() -> f64 {
    0.03
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SupervisedRatioGate {
    #[serde(default = "default_min_supervised")]
    pub min_pct: f64,
    #[serde(default)]
    pub block: bool,
}

fn default_min_supervised() -> f64 {
    0.10
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AntiStubGate {
    #[serde(default)]
    pub min_pass_rate: f64,
    #[serde(default)]
    pub max_placeholder_event_rate: f64,
    #[serde(default)]
    pub max_trivial_placeholder_event_rate: f64,
    #[serde(default)]
    pub min_construct_richness_mean: f64,
    #[serde(default)]
    pub block: bool,
}

/// Validate collateral damage gate stopping rule and sequential block if present.
pub fn validate_collateral_rule(gate: &CollateralDamageGate, path: &Path) -> Result<()> {
    if let Some(block) = &gate.sequential {
        let alpha = gate.alpha.unwrap_or(0.05);
        vox_eval::sequential_mcnemar::SequentialRule::from_fractions(
            &block.looks_fraction,
            block.look_alpha.clone(),
            alpha,
            vox_eval::reasoning_bench::SLICE_SIZE,
        )
        .map_err(|e| anyhow::anyhow!("policy at {}: {e}", path.display()))?;
    }
    if gate.rule == CollateralRule::SequentialMcnemarV1 {
        if gate.sequential.is_none() {
            anyhow::bail!(
                "policy at {} specifies rule 'sequential_mcnemar_v1' but no 'sequential' block is present",
                path.display()
            );
        }
        let rule = gate
            .sequential_rule()
            .map_err(|e| anyhow::anyhow!("policy at {}: {e}", path.display()))?
            .ok_or_else(|| {
                anyhow::anyhow!("policy at {}: missing sequential rule", path.display())
            })?;
        rule.check_size()
            .map_err(|e| anyhow::anyhow!("policy at {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Load policy from path.
pub fn load_policy(path: &Path) -> Result<EvalGatePolicy> {
    let content = read_utf8_path_capped(path)?;
    let policy: EvalGatePolicy = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("policy at {}: {e}", path.display()))?;
    validate_collateral_rule(&policy.collateral_damage, path)?;
    Ok(policy)
}

/// Load policy file as untyped JSON value.
pub fn load_policy_value(path: &Path) -> Result<serde_json::Value> {
    let content = read_utf8_path_capped(path)?;
    let val: serde_json::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("policy at {}: {e}", path.display()))?;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLICE: &[u8] = b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n{\"id\":\"c\"}\n";

    fn blake3_hex(b: &[u8]) -> String {
        vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
    }

    fn pin_of(slice: &[u8], prefix_n: usize) -> SlicePin {
        let prefix: Vec<u8> = slice
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .take(prefix_n)
            .fold(Vec::new(), |mut acc, l| {
                acc.extend_from_slice(l);
                acc.push(b'\n');
                acc
            });
        SlicePin {
            blake3: blake3_hex(slice),
            prefix_n,
            prefix_blake3: blake3_hex(&prefix),
        }
    }

    #[test]
    fn pin_matches_exact_slice() {
        let pin = pin_of(SLICE, 2);
        assert!(pin.verify(SLICE).is_ok());
    }

    #[test]
    fn changed_prefix_item_is_refused() {
        let mut pin = pin_of(SLICE, 2);
        let edited = b"{\"id\":\"Z\"}\n{\"id\":\"b\"}\n{\"id\":\"c\"}\n";
        pin.blake3 = blake3_hex(edited);
        let e = pin.verify(edited).unwrap_err();
        assert!(e.contains("prefix BLAKE3 mismatch"), "{e}");
    }

    #[test]
    fn the_ladder_pins_every_slice_and_the_checked_in_files_match() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let policy = load_policy(&root.join("mens/config/eval-gates-ladder.yaml")).unwrap();
        for name in vox_eval::reasoning_bench::SLICE_NAMES {
            let pin = policy
                .collateral_damage
                .slices
                .get(name)
                .unwrap_or_else(|| panic!("{name}"));
            assert_eq!(
                pin.prefix_n, 32,
                "{name}: the prefix is the original 32 items"
            );
            let bytes =
                std::fs::read(root.join("mens/data").join(name).join("reasoning.jsonl")).unwrap();
            pin.verify(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn collateral_requirement_from_policy_json_agrees_with_typed_gate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root.join("mens/config/eval-gates-ladder.yaml");
        let typed = load_policy(&path).unwrap();
        let val = load_policy_value(&path).unwrap();
        let req =
            vox_eval::collateral_evidence::CollateralRequirement::from_policy_json(&val).unwrap();
        assert_eq!(Some(req.alpha), typed.collateral_damage.alpha);
        for (name, pin) in &typed.collateral_damage.slices {
            assert_eq!(req.slices.get(name), Some(&pin.blake3));
        }
    }

    #[test]
    fn load_policy_reads_collateral_damage_block_from_ladder_yaml() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let policy = load_policy(&root.join("mens/config/eval-gates-ladder.yaml")).unwrap();
        assert_eq!(policy.collateral_damage.alpha, Some(0.05));
        assert_eq!(policy.collateral_damage.rule, CollateralRule::FixedN);
        assert!(policy.collateral_damage.slices.len() >= 3);
        assert!(policy.collateral_damage.block);
    }

    #[test]
    fn load_policy_refuses_active_sequential_rule_without_sequential_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing_block.yaml");
        let yaml = r#"
collateral_damage:
  alpha: 0.05
  rule: sequential_mcnemar_v1
"#;
        std::fs::write(&path, yaml).unwrap();
        let err = load_policy(&path).unwrap_err().to_string();
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("sequential"), "{err}");
    }

    #[test]
    fn load_policy_refuses_unknown_rule_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unknown_rule.yaml");
        let yaml = r#"
collateral_damage:
  rule: invalid_rule
"#;
        std::fs::write(&path, yaml).unwrap();
        let err = load_policy(&path).unwrap_err().to_string();
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(
            err.contains("unknown variant") || err.contains("invalid_rule"),
            "{err}"
        );
    }

    #[test]
    fn oversized_active_schedule_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized.yaml");
        let yaml = r#"
collateral_damage:
  alpha: 0.05
  rule: sequential_mcnemar_v1
  sequential:
    looks_fraction: [0.25, 0.5, 0.75, 1.0]
    look_alpha: [0.05, 0.002, 0.005, 0.04795]
"#;
        std::fs::write(&path, yaml).unwrap();
        let err = load_policy(&path).unwrap_err().to_string();
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("worst-case size"), "{err}");
        assert!(err.contains("exceeds alpha"), "{err}");
    }

    #[test]
    fn oversized_inactive_schedule_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized_inactive.yaml");
        let yaml = r#"
collateral_damage:
  alpha: 0.05
  rule: fixed_n
  sequential:
    looks_fraction: [0.25, 0.5, 0.75, 1.0]
    look_alpha: [0.05, 0.002, 0.005, 0.04795]
"#;
        std::fs::write(&path, yaml).unwrap();
        let policy = load_policy(&path).expect("inactive oversized policy loads");
        assert_eq!(policy.collateral_damage.sequential_rule().unwrap(), None);
    }
}
