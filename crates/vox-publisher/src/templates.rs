//! Standard news templates (placeholders `{{key}}`).

use crate::contract::{DEFAULT_GITHUB_REPO, DEFAULT_OPENCOLLECTIVE_SLUG, DEFAULT_SITE_BASE_URL};

/// Known template ids for agents and MCP tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsTemplateId {
    ResearchUpdate,
    Release,
    SecurityAdvisory,
    CommunityUpdate,
}

impl NewsTemplateId {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResearchUpdate => "research_update",
            Self::Release => "release",
            Self::SecurityAdvisory => "security_advisory",
            Self::CommunityUpdate => "community_update",
        }
    }
}

/// Replace `{{key}}` with values from `vars`. Unknown placeholders are left unchanged.
pub fn render_placeholders(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        let needle = format!("{{{{{}}}}}", k);
        out = out.replace(&needle, v);
    }
    out
}

pub fn ensure_no_unresolved_placeholders(rendered: &str) -> anyhow::Result<()> {
    if rendered.contains("{{") || rendered.contains("}}") {
        anyhow::bail!("Template rendering left unresolved placeholders.");
    }
    Ok(())
}

/// Embedded canonical templates (LF). Paths are relative to repo root for human editing; crate copies stay in sync via tests.
pub fn template_source(id: NewsTemplateId) -> &'static str {
    match id {
        NewsTemplateId::ResearchUpdate => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/news-templates/research_update.md"
            ))
        }
        NewsTemplateId::Release => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/news-templates/release.md"
            ))
        }
        NewsTemplateId::SecurityAdvisory => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/news-templates/security_advisory.md"
            ))
        }
        NewsTemplateId::CommunityUpdate => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/news-templates/community_update.md"
            ))
        }
    }
}

/// Render a research draft from standard placeholders.
pub fn render_research_update(
    id: &str,
    title: &str,
    author: &str,
    published_at_rfc3339: &str,
    abstract_text: &str,
) -> String {
    let base = template_source(NewsTemplateId::ResearchUpdate);
    let rendered = render_placeholders(
        base,
        &[
            ("id", id),
            ("title", title),
            ("author", author),
            ("published_at", published_at_rfc3339),
            ("abstract_text", abstract_text),
            ("site_base_url", DEFAULT_SITE_BASE_URL),
            ("default_github_repo", DEFAULT_GITHUB_REPO),
            ("default_collective_slug", DEFAULT_OPENCOLLECTIVE_SLUG),
        ],
    );
    // Keep behavior deterministic for callers: unresolved placeholders are an authoring error.
    if let Err(e) = ensure_no_unresolved_placeholders(&rendered) {
        tracing::warn!("research template unresolved placeholders: {}", e);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_template_renders_keys() {
        let s = render_research_update("r1", "T", "A", "2026-01-01T00:00:00Z", "Abstract here");
        assert!(s.contains("id: \"r1\""));
        assert!(s.contains("# T"));
        assert!(s.contains("Abstract here"));
        assert!(!s.contains("{{"));
    }

    #[test]
    fn docs_mirror_research_template_matches_crate_template() {
        let crate_src = template_source(NewsTemplateId::ResearchUpdate).replace("\r\n", "\n");
        let docs_src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/news/templates/research_update.md"
        ))
        .replace("\r\n", "\n");
        assert_eq!(
            crate_src, docs_src,
            "docs/news/templates/research_update.md must mirror crates/vox-publisher/news-templates/research_update.md"
        );
    }
}
