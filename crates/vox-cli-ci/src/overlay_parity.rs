use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use vox_bounded_fs::read_utf8_path_capped;

#[derive(Deserialize, Debug)]
pub struct ClaimsPolicy {
    #[serde(rename = "schema_version", default)]
    pub _schema_version: u32,
    pub overlays: Vec<String>,
    pub claims: Vec<Claim>,
}

#[derive(Deserialize, Debug)]
pub struct Claim {
    pub id: String,
    pub anchor: String,
    pub forbid: Vec<String>,
    pub allow_if: Option<String>,
    pub rationale: String,
}

struct CompiledClaim {
    id: String,
    anchor: String,
    forbid_regexes: Vec<Regex>,
    allow_if_regex: Option<Regex>,
    rationale: String,
}

fn contains_anchor(haystack: &str, anchor: &str) -> bool {
    if haystack.contains(anchor) {
        return true;
    }
    let h_norm: String = haystack.split_whitespace().collect::<Vec<_>>().join(" ");
    let a_norm: String = anchor.split_whitespace().collect::<Vec<_>>().join(" ");
    h_norm.contains(&a_norm)
}

pub fn check(
    agents_md: &str,
    overlays: &[(String, String)],
    claims_yaml: &str,
) -> Result<Vec<String>> {
    let policy: ClaimsPolicy =
        serde_yaml::from_str(claims_yaml).context("Failed to parse overlay claims YAML")?;

    let mut compiled_claims = Vec::with_capacity(policy.claims.len());
    for claim in &policy.claims {
        let mut forbid_regexes = Vec::with_capacity(claim.forbid.len());
        for pattern in &claim.forbid {
            let re = Regex::new(pattern).with_context(|| {
                format!("Invalid forbid regex for claim {}: {}", claim.id, pattern)
            })?;
            forbid_regexes.push(re);
        }

        let allow_if_regex = match &claim.allow_if {
            Some(pattern) => Some(Regex::new(pattern).with_context(|| {
                format!("Invalid allow_if regex for claim {}: {}", claim.id, pattern)
            })?),
            None => None,
        };

        compiled_claims.push(CompiledClaim {
            id: claim.id.clone(),
            anchor: claim.anchor.clone(),
            forbid_regexes,
            allow_if_regex,
            rationale: claim.rationale.clone(),
        });
    }

    let mut failures = Vec::new();

    // Check that every claim's anchor appears in AGENTS.md
    for claim in &compiled_claims {
        if !contains_anchor(agents_md, &claim.anchor) {
            failures.push(format!(
                "AGENTS.md anchor for claim {} no longer occurs in AGENTS.md: {:?}",
                claim.id, claim.anchor
            ));
        }
    }

    // Check each overlay line against each claim
    for (rel_path, content) in overlays {
        for (line_idx, line) in content.lines().enumerate() {
            let line_no = line_idx + 1;
            for claim in &compiled_claims {
                let is_allowed = claim
                    .allow_if_regex
                    .as_ref()
                    .is_some_and(|re| re.is_match(line));
                if is_allowed {
                    continue;
                }
                for forbid_re in &claim.forbid_regexes {
                    if forbid_re.is_match(line) {
                        failures.push(format!(
                            "{rel_path}:{line_no}: contradicts AGENTS.md claim {}: {}",
                            claim.id, claim.rationale
                        ));
                        break;
                    }
                }
            }
        }
    }

    Ok(failures)
}

pub fn run(root: &Path) -> Result<()> {
    let agents_path = root.join("AGENTS.md");
    if !agents_path.is_file() {
        bail!("Missing AGENTS.md at {}", agents_path.display());
    }
    let agents_md = read_utf8_path_capped(&agents_path)
        .with_context(|| format!("read {}", agents_path.display()))?;

    let claims_path = root.join("contracts/documentation/overlay-claims.v1.yaml");
    if !claims_path.is_file() {
        bail!("Missing claims file at {}", claims_path.display());
    }
    let claims_yaml = read_utf8_path_capped(&claims_path)
        .with_context(|| format!("read {}", claims_path.display()))?;

    let policy: ClaimsPolicy =
        serde_yaml::from_str(&claims_yaml).context("Failed to parse overlay claims YAML")?;

    let mut overlays: Vec<(String, String)> = Vec::new();
    for overlay_pattern in &policy.overlays {
        if overlay_pattern.contains('*') {
            // E.g. ".cursor/rules/*.mdc"
            let (parent_rel, file_pattern) = overlay_pattern
                .rsplit_once('/')
                .unwrap_or(("", overlay_pattern));
            let dir = root.join(parent_rel);
            if !dir.is_dir() {
                bail!("Missing overlay directory: {}", dir.display());
            }
            let ext = file_pattern.strip_prefix('*').unwrap_or(file_pattern);
            let mut entries = Vec::new();
            for entry in
                fs::read_dir(&dir).with_context(|| format!("read dir {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.to_string_lossy().ends_with(ext) {
                    entries.push(path);
                }
            }
            entries.sort();
            if entries.is_empty() {
                bail!("no {} files found in {}", ext, dir.display());
            }
            for path in entries {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = read_utf8_path_capped(&path)
                    .with_context(|| format!("read {}", path.display()))?;
                overlays.push((rel, content));
            }
        } else {
            let file_path = root.join(overlay_pattern);
            if !file_path.is_file() {
                bail!("Missing overlay file: {}", overlay_pattern);
            }
            let content = read_utf8_path_capped(&file_path)
                .with_context(|| format!("read {}", file_path.display()))?;
            overlays.push((overlay_pattern.clone(), content));
        }
    }

    let failures = check(&agents_md, &overlays, &claims_yaml)?;
    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{f}");
        }
        bail!(
            "overlay parity check failed with {} failure(s)",
            failures.len()
        );
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tempfile::tempdir;

    const FIXTURE_CLAIMS: &str = r#"schema_version: 1
overlays:
  - CLAUDE.md
  - GEMINI.md
  - .github/copilot-instructions.md
  - docs/src/AGENTS.md
  - .cursor/rules/*.mdc
claims:
  - id: no-hand-last-updated
    anchor: "Do **not** hand-add `last_updated`"
    forbid:
      - '(?i)(?:must\s+include|must\s+have|must\s+set|include|required).*last_updated'
    allow_if: '(?i)(?:do\s+not|never|not|derives\s+it)'
    rationale: "Git history derives last_updated; hand-authored last_updated causes a doc lint failure."
"#;

    const FIXTURE_AGENTS_MD: &str = r#"# AGENTS policy
Do **not** hand-add `last_updated` — the pipeline derives it from Git history.
"#;

    #[test]
    fn check_fails_on_affirmative_last_updated_requirement() {
        let overlays = vec![(
            ".cursor/rules/documentation-policy.mdc".to_string(),
            "New doc pages must include `category`, `last_updated`".to_string(),
        )];
        let failures = check(FIXTURE_AGENTS_MD, &overlays, FIXTURE_CLAIMS).unwrap();
        assert_eq!(failures.len(), 1, "expected 1 failure, got {failures:?}");
        assert!(failures[0].contains(".cursor/rules/documentation-policy.mdc:1:"));
        assert!(failures[0].contains("no-hand-last-updated"));
    }

    #[test]
    fn check_passes_on_corrected_overlay_line() {
        let overlays = vec![(
            ".cursor/rules/documentation-policy.mdc".to_string(),
            "- New doc pages need frontmatter. Never hand-author `last_updated`: the lint rejects it.".to_string(),
        )];
        let failures = check(FIXTURE_AGENTS_MD, &overlays, FIXTURE_CLAIMS).unwrap();
        assert!(failures.is_empty(), "expected 0 failures, got {failures:?}");
    }

    #[test]
    fn check_passes_on_negation_line() {
        let overlays = vec![(
            "CLAUDE.md".to_string(),
            "Do not hand-add `last_updated` to markdown pages.".to_string(),
        )];
        let failures = check(FIXTURE_AGENTS_MD, &overlays, FIXTURE_CLAIMS).unwrap();
        assert!(failures.is_empty(), "expected 0 failures, got {failures:?}");
    }

    #[test]
    fn check_fails_when_anchor_missing_from_agents_md() {
        let empty_agents = "# Empty policy\n";
        let overlays = vec![];
        let failures = check(empty_agents, &overlays, FIXTURE_CLAIMS).unwrap();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("no longer occurs in AGENTS.md"));
        assert!(failures[0].contains("no-hand-last-updated"));
    }

    #[test]
    fn check_errors_on_invalid_forbid_regex() {
        let bad_claims = r#"schema_version: 1
overlays: []
claims:
  - id: bad-regex-claim
    anchor: "Do **not** hand-add `last_updated`"
    forbid:
      - '(?i)(unclosed'
    rationale: "Bad regex test"
"#;
        let err = check(FIXTURE_AGENTS_MD, &[], bad_claims).unwrap_err();
        assert!(err.to_string().contains("bad-regex-claim"));
    }

    #[test]
    fn run_errors_on_missing_gemini_md() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("AGENTS.md"), FIXTURE_AGENTS_MD).unwrap();
        fs::create_dir_all(root.join("contracts/documentation")).unwrap();
        fs::write(
            root.join("contracts/documentation/overlay-claims.v1.yaml"),
            FIXTURE_CLAIMS,
        )
        .unwrap();
        fs::write(root.join("CLAUDE.md"), "# Claude\n").unwrap();
        // GEMINI.md intentionally omitted

        let err = run(root).unwrap_err();
        assert!(
            err.to_string().contains("GEMINI.md"),
            "expected error mentioning GEMINI.md, got: {err}"
        );
    }

    #[test]
    fn run_errors_when_cursor_rules_has_no_mdc() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("AGENTS.md"), FIXTURE_AGENTS_MD).unwrap();
        fs::create_dir_all(root.join("contracts/documentation")).unwrap();
        fs::write(
            root.join("contracts/documentation/overlay-claims.v1.yaml"),
            FIXTURE_CLAIMS,
        )
        .unwrap();
        fs::write(root.join("CLAUDE.md"), "# Claude\n").unwrap();
        fs::write(root.join("GEMINI.md"), "# Gemini\n").unwrap();
        fs::create_dir_all(root.join(".github")).unwrap();
        fs::write(root.join(".github/copilot-instructions.md"), "# Copilot\n").unwrap();
        fs::create_dir_all(root.join("docs/src")).unwrap();
        fs::write(root.join("docs/src/AGENTS.md"), "# Docs AGENTS\n").unwrap();
        fs::create_dir_all(root.join(".cursor/rules")).unwrap();
        // .cursor/rules directory exists but has no .mdc files

        let err = run(root).unwrap_err();
        assert!(
            err.to_string().contains(".cursor/rules") || err.to_string().contains(".mdc"),
            "expected error mentioning .cursor/rules or .mdc, got: {err}"
        );
    }

    #[test]
    fn check_claim_inline_vox_not_include() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let claims =
            fs::read_to_string(root.join("contracts/documentation/overlay-claims.v1.yaml"))
                .unwrap();
        let agents_md = fs::read_to_string(root.join("AGENTS.md")).unwrap();

        let bad_overlay = vec![(
            "test.mdc".to_string(),
            "use `{{#include}}` from examples/golden".to_string(),
        )];
        let failures = check(&agents_md, &bad_overlay, &claims).unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("inline-vox-not-include")),
            "expected failure for inline-vox-not-include, got: {failures:?}"
        );

        let good_overlay = vec![(
            "test.mdc".to_string(),
            "Do not use `{{#include}}` for new code".to_string(),
        )];
        let failures = check(&agents_md, &good_overlay, &claims).unwrap();
        assert!(
            !failures
                .iter()
                .any(|f| f.contains("inline-vox-not-include"))
        );
    }

    #[test]
    fn check_claim_frontmatter_minimum_keys() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let claims =
            fs::read_to_string(root.join("contracts/documentation/overlay-claims.v1.yaml"))
                .unwrap();
        let agents_md = fs::read_to_string(root.join("AGENTS.md")).unwrap();

        let bad_overlay1 = vec![(
            "test.mdc".to_string(),
            "- `training_eligible` is required".to_string(),
        )];
        let failures = check(&agents_md, &bad_overlay1, &claims).unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("frontmatter-minimum-keys"))
        );

        let bad_overlay2 = vec![(
            "test.mdc".to_string(),
            "`last_updated` is optional metadata".to_string(),
        )];
        let failures = check(&agents_md, &bad_overlay2, &claims).unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("frontmatter-minimum-keys"))
        );

        let good_overlay = vec![(
            "test.mdc".to_string(),
            "Minimum required keys: title, description, category, topic, domain, authorship"
                .to_string(),
        )];
        let failures = check(&agents_md, &good_overlay, &claims).unwrap();
        assert!(
            !failures
                .iter()
                .any(|f| f.contains("frontmatter-minimum-keys"))
        );
    }

    #[test]
    fn check_claim_no_cargo_fmt_all() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let claims =
            fs::read_to_string(root.join("contracts/documentation/overlay-claims.v1.yaml"))
                .unwrap();
        let agents_md = fs::read_to_string(root.join("AGENTS.md")).unwrap();

        let bad_overlay = vec![(
            "test.mdc".to_string(),
            "run `cargo fmt --all` to format".to_string(),
        )];
        let failures = check(&agents_md, &bad_overlay, &claims).unwrap();
        assert!(failures.iter().any(|f| f.contains("no-cargo-fmt-all")));

        let good_overlay = vec![(
            "test.mdc".to_string(),
            "Never run `cargo fmt --all` on this workspace".to_string(),
        )];
        let failures = check(&agents_md, &good_overlay, &claims).unwrap();
        assert!(!failures.iter().any(|f| f.contains("no-cargo-fmt-all")));
    }

    #[test]
    fn check_claim_no_explicit_cargo_path() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let claims =
            fs::read_to_string(root.join("contracts/documentation/overlay-claims.v1.yaml"))
                .unwrap();
        let agents_md = fs::read_to_string(root.join("AGENTS.md")).unwrap();

        let bad_overlay = vec![(
            "test.mdc".to_string(),
            r#"& "$env:USERPROFILE\.cargo\bin\cargo.exe" <args>"#.to_string(),
        )];
        let failures = check(&agents_md, &bad_overlay, &claims).unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("no-explicit-cargo-path"))
        );

        let good_overlay = vec![(
            "test.mdc".to_string(),
            "never call the real cargo binary by an explicit resolved path (~/.cargo/bin/cargo)"
                .to_string(),
        )];
        let failures = check(&agents_md, &good_overlay, &claims).unwrap();
        assert!(
            !failures
                .iter()
                .any(|f| f.contains("no-explicit-cargo-path"))
        );
    }

    #[test]
    fn real_tree() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root).expect("real tree overlay parity must pass");
    }
}
