use std::collections::HashMap;
use vox_code_audit::rules::{Finding, FindingConfidence, Severity};
use crate::features::{ExtractedFeatures, UnitHint};
use super::SweepRule;

pub struct NumericDedupRule {
    pub threshold: usize,
}

impl Default for NumericDedupRule {
    fn default() -> Self { Self { threshold: 3 } }
}

impl SweepRule for NumericDedupRule {
    fn id(&self) -> &'static str { "sweep/duplicate-numeric-literal" }
    fn severity(&self) -> Severity { Severity::Warning }

    fn sweep(&self, files: &[ExtractedFeatures]) -> Vec<Finding> {
        let mut index: HashMap<(u64, u8), Vec<(std::path::PathBuf, usize)>> = HashMap::new();
        for f in files {
            for n in &f.numeric_literals {
                let unit_disc = match &n.unit {
                    Some(UnitHint::Seconds) => 1u8,
                    Some(UnitHint::Millis) => 2,
                    Some(UnitHint::Bytes) => 3,
                    _ => continue,
                };
                let key = (n.value.to_bits(), unit_disc);
                index.entry(key).or_default().push((f.file.clone(), n.loc.line));
            }
        }
        index.into_iter()
            .filter(|(_, locs)| locs.len() >= self.threshold)
            .map(|((bits, unit_disc), locs)| {
                let val = f64::from_bits(bits);
                let unit_str = match unit_disc { 1 => "s", 2 => "ms", _ => "bytes" };
                Finding {
                    rule_id: self.id().to_string(),
                    rule_name: "Duplicate Numeric Literal".into(),
                    severity: self.severity(),
                    file: locs[0].0.clone(),
                    line: locs[0].1,
                    column: 0,
                    message: format!(
                        "{}{} appears {} times — define a named constant",
                        val, unit_str, locs.len()
                    ),
                    suggestion: Some(
                        "Add a const to vox-config::timeouts or the appropriate SSOT module".into(),
                    ),
                    context: String::new(),
                    confidence: Some(FindingConfidence::High),
                    evidence: Some(serde_json::json!({
                        "occurrences": locs.iter().map(|(p, l)| format!("{}:{}", p.display(), l)).collect::<Vec<_>>()
                    })),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::*;
    use std::path::PathBuf;
    use vox_code_audit::rules::Language;

    #[test]
    fn finds_repeated_duration_constant() {
        let make = |line: usize, val: f64| ExtractedFeatures {
            numeric_literals: vec![NumericLoc {
                value: val,
                unit: Some(UnitHint::Seconds),
                loc: Loc { line, col: 0 },
            }],
            ..ExtractedFeatures::new(PathBuf::from(format!("{}.rs", line)), Language::Rust)
        };
        let files = vec![make(1, 30.0), make(2, 30.0), make(3, 30.0)];
        let rule = NumericDedupRule::default();
        let findings = rule.sweep(&files);
        assert!(!findings.is_empty());
        assert!(findings[0].message.contains("30"));
    }
}
