use super::findings::RealityAuditFinding;
use super::suppressions::SuppressionsFile;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrandfatheredDeficit {
    pub fingerprint: String,
    pub page_path: String,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFile {
    pub schema_version: u32,
    pub generated_at: String,
    pub deficits: Vec<GrandfatheredDeficit>,
}

impl BaselineFile {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self {
                schema_version: 1,
                generated_at: chrono::Utc::now().to_rfc3339(),
                deficits: Vec::new(),
            });
        }
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read baseline file: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse baseline JSON: {}", e))
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, json)
    }

    pub fn contains_fingerprint(&self, fp: &str) -> bool {
        self.deficits.iter().any(|d| d.fingerprint == fp)
    }

    /// Match either exact fingerprint or `(page_path, raw_text)` so line-number shifts
    /// caused by frontmatter or header edits do not invalidate grandfathered entries.
    pub fn contains_deficit(&self, fp: &str, doc_paths: &[String], claim_ids: &[String]) -> bool {
        if self.contains_fingerprint(fp) {
            return true;
        }
        let Some(raw) = claim_ids.first() else {
            return false;
        };
        doc_paths.iter().any(|p| {
            self.deficits
                .iter()
                .any(|d| d.page_path == *p && d.raw_text == *raw)
        })
    }

    /// Tighten baseline by removing deficits that have been resolved.
    /// Baseline can ONLY shrink, never grow.
    pub fn tighten(&mut self, active_fingerprints: &HashSet<String>) -> usize {
        let before = self.deficits.len();
        self.deficits
            .retain(|d| active_fingerprints.contains(&d.fingerprint));
        before - self.deficits.len()
    }
}

/// Check if a page path is reader-facing (tutorials, reference, how-to, explanation, concepts).
pub fn is_reader_facing_page(page_path: &str) -> bool {
    let norm = page_path.replace('\\', "/");
    norm.contains("/tutorials/")
        || norm.contains("/reference/")
        || norm.contains("/how-to/")
        || norm.contains("/explanation/")
        || norm.contains("/concepts/")
}

#[derive(Debug, Clone)]
pub struct DriftGateResult {
    pub passed: bool,
    pub new_deficits: Vec<RealityAuditFinding>,
    pub expired_suppression_errors: Vec<String>,
    pub active_baselined_count: usize,
    pub resolved_baselined_count: usize,
}

/// Evaluate the tighten-only drift gate.
/// Rules:
/// 1. Only blocks merges for defects on reader-facing `status: current` documents.
/// 2. Deficits already present in `baseline.v1.json` are permitted (grandfathered).
/// 3. Deficits covered by active unexpired suppressions are permitted.
/// 4. Any NEW unsuppressed deficit fails the gate.
/// 5. Any expired suppression fails the gate.
pub fn evaluate_drift_gate(
    findings: &[RealityAuditFinding],
    baseline: &BaselineFile,
    suppressions: &SuppressionsFile,
    today: NaiveDate,
) -> DriftGateResult {
    let mut new_deficits = Vec::new();
    let mut active_baselined_count = 0;

    let suppression_errors = suppressions.validate(today).err().unwrap_or_default();

    for f in findings {
        let is_reader_facing = f.doc_paths.iter().any(|p| is_reader_facing_page(p));
        if !is_reader_facing {
            // Internals, architecture, archive pages are report-only and do not block
            continue;
        }

        let fp = f.fingerprint.as_deref().unwrap_or(&f.id);
        // Check if grandfathered in baseline
        if baseline.contains_deficit(fp, &f.doc_paths, &f.claim_ids) {
            active_baselined_count += 1;
            continue;
        }

        // Check if covered by active suppression
        let is_suppressed = f.doc_paths.iter().any(|doc_path| {
            suppressions.is_suppressed(doc_path, &f.claim_type, today)
                || f.code_paths
                    .iter()
                    .any(|cp| suppressions.is_suppressed(doc_path, cp, today))
                || f.contract_paths
                    .iter()
                    .any(|cp| suppressions.is_suppressed(doc_path, cp, today))
                || f.claim_ids
                    .iter()
                    .any(|cid| suppressions.is_suppressed(doc_path, cid, today))
        });

        if is_suppressed {
            continue;
        }

        new_deficits.push(f.clone());
    }

    let active_fps: HashSet<String> = findings
        .iter()
        .map(|f| f.fingerprint.clone().unwrap_or_else(|| f.id.clone()))
        .collect();
    let resolved_baselined_count = baseline
        .deficits
        .iter()
        .filter(|d| !active_fps.contains(&d.fingerprint))
        .count();

    let passed = new_deficits.is_empty() && suppression_errors.is_empty();

    DriftGateResult {
        passed,
        new_deficits,
        expired_suppression_errors: suppression_errors,
        active_baselined_count,
        resolved_baselined_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::findings::{FindingScores, RealityAuditFinding};
    use crate::claims::suppressions::DocSuppression;

    fn sample_finding(id: &str, fp: &str, doc_path: &str, target: &str) -> RealityAuditFinding {
        RealityAuditFinding {
            id: id.to_string(),
            claim_ids: vec![target.to_string()],
            doc_paths: vec![doc_path.to_string()],
            code_paths: vec![target.to_string()],
            contract_paths: Vec::new(),
            claim_type: "cli_command".to_string(),
            classification: "missing".to_string(),
            secondary_tags: Vec::new(),
            scores: FindingScores {
                impact: 3,
                blast_radius: 2,
                staleness: 1,
                enforcement_gap: 3,
                tractability: 3,
            },
            priority_score: 54,
            priority_band: "P1".to_string(),
            recommended_action: format!("Update mention {}", target),
            owner_candidate: None,
            status: "missing".to_string(),
            notes: None,
            source: Some("machine_audit".to_string()),
            fingerprint: Some(fp.to_string()),
            detector: Some("claims-extractor".to_string()),
        }
    }

    #[test]
    fn test_drift_gate_ratchet() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();

        let baselined_finding = sample_finding(
            "doc-deficit-0001",
            "fp-old",
            "docs/src/reference/cli.md",
            "vox old-cmd",
        );

        let new_finding = sample_finding(
            "doc-deficit-0002",
            "fp-new",
            "docs/src/tutorials/first-app.md",
            "vox broken-cmd",
        );

        let mut baseline = BaselineFile {
            schema_version: 1,
            generated_at: "2026-10-13T00:00:00Z".to_string(),
            deficits: vec![GrandfatheredDeficit {
                fingerprint: "fp-old".to_string(),
                page_path: "docs/src/reference/cli.md".to_string(),
                raw_text: "vox old-cmd".to_string(),
            }],
        };

        let suppressions = SuppressionsFile {
            schema_version: 1,
            suppressions: Vec::new(),
        };

        // 1. With only baselined finding -> pass
        let res1 = evaluate_drift_gate(
            std::slice::from_ref(&baselined_finding),
            &baseline,
            &suppressions,
            today,
        );
        assert!(res1.passed);
        assert_eq!(res1.new_deficits.len(), 0);
        assert_eq!(res1.active_baselined_count, 1);

        // 2. With new unsuppressed finding -> fail
        let res2 = evaluate_drift_gate(
            &[baselined_finding.clone(), new_finding.clone()],
            &baseline,
            &suppressions,
            today,
        );
        assert!(!res2.passed);
        assert_eq!(res2.new_deficits.len(), 1);
        assert_eq!(res2.new_deficits[0].fingerprint.as_deref(), Some("fp-new"));

        // 3. With suppression for new finding -> pass
        let mut suppressions_with_new = suppressions.clone();
        suppressions_with_new.suppressions.push(DocSuppression {
            id: "sup-01".to_string(),
            page_path: "docs/src/tutorials/first-app.md".to_string(),
            target: "vox broken-cmd".to_string(),
            owner: "team-docs".to_string(),
            reason: "Temporarily suppressing".to_string(),
            expires_after: "2026-11-01".to_string(),
        });
        let res3 = evaluate_drift_gate(
            &[baselined_finding.clone(), new_finding.clone()],
            &baseline,
            &suppressions_with_new,
            today,
        );
        assert!(res3.passed);

        // 4. Tighten removes resolved deficits from baseline
        let mut active_fps = HashSet::new();
        active_fps.insert("fp-new".to_string());
        let tightened_count = baseline.tighten(&active_fps);
        assert_eq!(tightened_count, 1);
        assert_eq!(baseline.deficits.len(), 0);
    }

    #[test]
    fn test_drift_gate_tolerates_line_number_shift() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let shifted_finding = sample_finding(
            "doc-deficit-0001",
            "fp-shifted-line",
            "docs/src/reference/cli.md",
            "vox old-cmd",
        );
        let baseline = BaselineFile {
            schema_version: 1,
            generated_at: "2026-10-13T00:00:00Z".to_string(),
            deficits: vec![GrandfatheredDeficit {
                fingerprint: "fp-original-line".to_string(),
                page_path: "docs/src/reference/cli.md".to_string(),
                raw_text: "vox old-cmd".to_string(),
            }],
        };
        let suppressions = SuppressionsFile {
            schema_version: 1,
            suppressions: Vec::new(),
        };
        let res = evaluate_drift_gate(&[shifted_finding], &baseline, &suppressions, today);
        assert!(res.passed);
        assert_eq!(res.new_deficits.len(), 0);
        assert_eq!(res.active_baselined_count, 1);
    }
}
