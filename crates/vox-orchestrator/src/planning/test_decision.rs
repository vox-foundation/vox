use crate::socrates::OrientReport;
use crate::types::{AgentTask, TaskCategory};
use serde::{Deserialize, Serialize};
use vox_socrates_policy::RiskBand;

/// Formal testing requirement for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestDecision {
    /// Test must be authored alongside or verified before completion.
    Required,
    /// Test is recommended but not blocking dispatch.
    Recommended,
    /// Test blocked by evidence gap; defer until task is clearer.
    Deferred,
    /// Test not applicable or explicitly waived (e.g. docs, configs).
    Skip,
}

/// Dynamic thresholds for deciding testing requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDecisionPolicy {
    /// The number of files touched that makes tests recommended.
    pub file_count_threshold: usize,
    /// Planning complexity out of 10.0 that demands testing.
    pub complexity_threshold: f64,
}

impl Default for TestDecisionPolicy {
    fn default() -> Self {
        Self {
            file_count_threshold: 4,
            complexity_threshold: 7.0,
        }
    }
}

impl TestDecisionPolicy {
    /// Evaluates dynamic rules against Task characteristics and Orient metrics.
    pub fn evaluate(&self, task: &AgentTask, orient: Option<&OrientReport>) -> TestDecision {
        // Rule: docs/config only -> Skip
        // We assume tasks with category Documentation are strictly docs.
        if let Some(o) = orient {
            if o.category == Some(TaskCategory::General) {
                // Used to be Documentation
                return TestDecision::Skip;
            }
        }

        let write_manifest: Vec<&std::path::PathBuf> = task.write_files();

        // Rule: Security keywords -> Required
        let low = task.description.to_ascii_lowercase();
        if low.contains("security")
            || low.contains("auth")
            || low.contains("crypto")
            || low.contains("permission")
        {
            return TestDecision::Required;
        }

        // Rule: .vox in manifest -> Required
        for path in &write_manifest {
            if path.extension().is_some_and(|e| e == "vox") {
                return TestDecision::Required;
            }
        }

        if let Some(o) = orient {
            // Rule: evidence_gap > 0.4 -> Deferred
            if o.evidence_gap > 0.4 {
                return TestDecision::Deferred;
            }

            // Rule: risk_band Red (Low) -> Required
            if o.risk_band == RiskBand::Low {
                return TestDecision::Required;
            }

            // Rule: complexity >= threshold -> Required
            if o.planning_complexity >= self.complexity_threshold {
                return TestDecision::Required;
            }
        }

        // Rule: file_count > threshold -> Recommended
        if write_manifest.len() > self.file_count_threshold {
            return TestDecision::Recommended;
        }

        TestDecision::Skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileAffinity, TaskId, TaskPriority};

    fn dummy_task(desc: &str, files: Vec<FileAffinity>) -> AgentTask {
        AgentTask::new(TaskId(1), desc, TaskPriority::Normal, files)
    }

    #[test]
    fn rule_security_keywords_trigger_required() {
        let policy = TestDecisionPolicy::default();
        let task = dummy_task("Implement auth permissions", vec![]);
        assert_eq!(policy.evaluate(&task, None), TestDecision::Required);
    }

    #[test]
    fn rule_vox_extension_trigger_required() {
        let policy = TestDecisionPolicy::default();
        let task = dummy_task("add syntax", vec![FileAffinity::write("abc.vox")]);
        assert_eq!(policy.evaluate(&task, None), TestDecision::Required);
    }

    #[test]
    fn rule_docs_category_trigger_skip() {
        let policy = TestDecisionPolicy::default();
        let task = dummy_task("add some logs", vec![]);
        let orient = OrientReport {
            category: Some(TaskCategory::General),
            ..Default::default()
        };
        assert_eq!(policy.evaluate(&task, Some(&orient)), TestDecision::Skip);
    }
}
