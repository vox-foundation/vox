use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::contract::validate_github_repo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNewsItem {
    pub id: String,
    pub title: String,
    pub author: String,
    pub published_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub content_markdown: String,
    #[serde(default)]
    pub syndication: SyndicationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyndicationConfig {
    pub twitter: Option<TwitterConfig>,
    pub github: Option<GitHubConfig>,
    pub open_collective: Option<OpenCollectiveConfig>,
    pub crates_io: Option<CratesIoConfig>,
    #[serde(default = "default_rss")]
    pub rss: bool,
    #[serde(default)]
    pub dry_run: bool,
}

fn default_rss() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    pub short_text: Option<String>,
    #[serde(default)]
    pub thread: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GitHubPostType {
    Release,
    Discussion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub repo: String,
    pub post_type: GitHubPostType,
    #[serde(default)]
    pub release_tag: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub discussion_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCollectiveConfig {
    #[serde(default)]
    pub is_private: bool,
    pub collective_slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CratesIoConfig {
    pub crates_to_update: Vec<String>,
}

impl UnifiedNewsItem {
    pub fn parse(content: &str, id: &str) -> anyhow::Result<Self> {
        crate::contract::validate_news_id(id)?;

        if !content.starts_with("---") {
            return Err(anyhow::anyhow!("Missing YAML frontmatter"));
        }

        let mut parts = content.splitn(3, "---");
        parts.next();
        let yaml = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Incomplete frontmatter"))?;
        let markdown = parts.next().unwrap_or_default().trim();

        #[derive(Deserialize)]
        struct Frontmatter {
            title: String,
            author: String,
            #[serde(default)]
            published_at: Option<String>,
            #[serde(default)]
            tags: Vec<String>,
            #[serde(default)]
            syndication: SyndicationConfig,
        }

        let front: Frontmatter = serde_yaml::from_str(yaml)?;

        let published_at = if let Some(ref s) = front.published_at {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                    s.parse::<DateTime<Utc>>()
                        .map_err(|e| anyhow::anyhow!("{}", e))
                })?
        } else {
            Utc::now()
        };

        if let Some(ref gh) = front.syndication.github {
            validate_github_repo(&gh.repo)?;
        }

        Ok(Self {
            id: id.to_string(),
            title: front.title,
            author: front.author,
            published_at,
            tags: front.tags,
            content_markdown: markdown.to_string(),
            syndication: front.syndication,
        })
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        crate::contract::validate_news_id(&self.id)?;
        if let Some(ref gh) = self.syndication.github {
            validate_github_repo(&gh.repo)?;
            match gh.post_type {
                GitHubPostType::Release => {
                    let tag = gh
                        .release_tag
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(self.id.as_str());
                    if tag.contains('/') {
                        anyhow::bail!("release_tag must not contain slashes: {:?}", tag);
                    }
                }
                GitHubPostType::Discussion => {
                    let cat = gh
                        .discussion_category
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("");
                    if cat.is_empty() {
                        anyhow::bail!(
                            "github.discussion_category is required when post_type is Discussion"
                        );
                    }
                }
            }
        }
        if let Some(ref oc) = self.syndication.open_collective
            && oc.collective_slug.trim().is_empty()
        {
            anyhow::bail!("open_collective.collective_slug must not be empty");
        }
        Ok(())
    }

    #[must_use]
    pub fn content_sha3_256(&self) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "title": self.title,
            "author": self.author,
            "published_at": self.published_at.to_rfc3339(),
            "tags": self.tags,
            "content_markdown": self.content_markdown,
            "syndication": self.syndication,
        });
        let mut hasher = Sha3_256::new();
        hasher.update(canonical.to_string().as_bytes());
        let digest = hasher.finalize();
        format!("{digest:x}")
    }
}
