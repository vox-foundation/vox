use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocSuppression {
    pub id: String,
    pub page_path: String,
    pub target: String,
    pub owner: String,
    pub reason: String,
    pub expires_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionsFile {
    pub schema_version: u32,
    pub suppressions: Vec<DocSuppression>,
}

impl SuppressionsFile {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self {
                schema_version: 1,
                suppressions: Vec::new(),
            });
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read suppressions file: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse suppressions JSON: {}", e))
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, json)
    }

    /// Validate all suppressions against rules:
    /// 1. page_path and target must not be empty.
    /// 2. owner must not be empty.
    /// 3. reason must not be empty.
    /// 4. expires_after must be a valid date in YYYY-MM-DD format.
    /// 5. expires_after must be >= today.
    pub fn validate(&self, today: NaiveDate) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for s in &self.suppressions {
            if s.page_path.trim().is_empty() {
                errors.push(format!(
                    "Suppression '{}' is missing required 'page_path'",
                    s.id
                ));
            }
            if s.target.trim().is_empty() {
                errors.push(format!(
                    "Suppression '{}' is missing required 'target'",
                    s.id
                ));
            }
            if s.owner.trim().is_empty() {
                errors.push(format!(
                    "Suppression '{}' is missing required 'owner'",
                    s.id
                ));
            }
            if s.reason.trim().is_empty() {
                errors.push(format!(
                    "Suppression '{}' is missing required 'reason'",
                    s.id
                ));
            }
            match NaiveDate::parse_from_str(s.expires_after.trim(), "%Y-%m-%d") {
                Ok(exp) => {
                    if exp < today {
                        errors.push(format!(
                            "Suppression '{}' expired on {} (today is {})",
                            s.id, s.expires_after, today
                        ));
                    }
                }
                Err(_) => {
                    errors.push(format!(
                        "Suppression '{}' has invalid date format '{}' (must be YYYY-MM-DD)",
                        s.id, s.expires_after
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check whether a given page path and target mention are covered by an active suppression.
    pub fn is_suppressed(&self, page_path: &str, target: &str, today: NaiveDate) -> bool {
        if page_path.trim().is_empty() || target.trim().is_empty() {
            return false;
        }
        let norm_page = page_path.replace('\\', "/");
        self.suppressions.iter().any(|s| {
            let supp_page = s.page_path.trim().replace('\\', "/");
            let supp_target = s.target.trim();
            if supp_page.is_empty() || supp_target.is_empty() {
                return false;
            }
            if let Ok(exp) = NaiveDate::parse_from_str(s.expires_after.trim(), "%Y-%m-%d")
                && exp >= today
            {
                let page_match = supp_page == norm_page
                    || norm_page.ends_with(&format!("/{}", supp_page.trim_start_matches('/')));
                let target_match = supp_target == target;
                return page_match && target_match;
            }
            false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppression_validation() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();

        let valid = DocSuppression {
            id: "sup-001".to_string(),
            page_path: "docs/src/ref.md".to_string(),
            target: "vox-foo".to_string(),
            owner: "team-infra".to_string(),
            reason: "Migration in progress".to_string(),
            expires_after: "2026-11-01".to_string(),
        };

        let expired = DocSuppression {
            id: "sup-002".to_string(),
            page_path: "docs/src/ref.md".to_string(),
            target: "vox-bar".to_string(),
            owner: "team-infra".to_string(),
            reason: "Old migration".to_string(),
            expires_after: "2026-10-01".to_string(),
        };

        let file = SuppressionsFile {
            schema_version: 1,
            suppressions: vec![valid.clone(), expired],
        };

        let res = file.validate(today);
        assert!(res.is_err());
        let errs = res.unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("expired on 2026-10-01"));

        assert!(file.is_suppressed("docs/src/ref.md", "vox-foo", today));
        assert!(!file.is_suppressed("docs/src/ref.md", "vox-bar", today));
    }
}
