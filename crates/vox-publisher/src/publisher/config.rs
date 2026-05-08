use crate::contract::NewsSiteConfig;
use tracing::warn;

#[derive(Clone)]
pub struct PublisherConfig {
    pub twitter_bearer_token: Option<String>,
    pub mastodon_domain: Option<String>,
    pub mastodon_access_token: Option<String>,
    pub bluesky_handle: Option<String>,
    pub bluesky_password: Option<String>,
    pub bluesky_pds_url: Option<String>,
    pub linkedin_access_token: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub open_collective_slug: Option<String>,
    pub linkedin_author_urn: Option<String>,
    pub forge_token: Option<String>,
    pub open_collective_token: Option<String>,
    pub dry_run: bool,
    pub site: NewsSiteConfig,
    pub twitter_api_base: Option<String>,
    pub forge_rest_base: Option<String>,
    pub forge_graphql_url: Option<String>,
    pub opencollective_graphql_url: Option<String>,
    pub twitter_text_chunk_max: Option<usize>,
    pub twitter_truncation_suffix: Option<String>,
    pub twitter_summary_margin_chars: Option<usize>,
    pub reddit_selfpost_summary_max: Option<usize>,
    pub reddit_client_id: Option<String>,
    pub reddit_client_secret: Option<String>,
    pub reddit_refresh_token: Option<String>,
    pub reddit_user_agent: Option<String>,
    pub youtube_client_id: Option<String>,
    pub youtube_client_secret: Option<String>,
    pub youtube_refresh_token: Option<String>,
    pub youtube_repo_root: Option<std::path::PathBuf>,
    pub hacker_news_mode: Option<String>,
    pub youtube_default_category_id: Option<String>,
    pub worthiness_score: Option<f64>,
    pub linkedin_api_base: Option<String>,
    pub reddit_api_base: Option<String>,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            twitter_bearer_token: None,
            mastodon_domain: None,
            mastodon_access_token: None,
            bluesky_handle: None,
            bluesky_password: None,
            bluesky_pds_url: None,
            linkedin_access_token: None,
            discord_webhook_url: None,
            linkedin_author_urn: None,
            open_collective_slug: None,
            forge_token: None,
            open_collective_token: None,
            dry_run: true,
            site: NewsSiteConfig::default(),
            twitter_api_base: None,
            forge_rest_base: None,
            forge_graphql_url: None,
            opencollective_graphql_url: None,
            twitter_text_chunk_max: None,
            twitter_truncation_suffix: None,
            twitter_summary_margin_chars: None,
            reddit_selfpost_summary_max: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_refresh_token: None,
            reddit_user_agent: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            youtube_refresh_token: None,
            youtube_repo_root: None,
            hacker_news_mode: None,
            youtube_default_category_id: None,
            worthiness_score: None,
            linkedin_api_base: None,
            reddit_api_base: None,
        }
    }
}

pub const ROUTE_SIMULATION_ENV_KEYS: &[&str] = &[
    "VOX_NEWS_SITE_BASE_URL",
    "VOX_NEWS_RSS_FEED_PATH",
    "VOX_NEWS_TWITTER_TOKEN",
    "VOX_NEWS_FORGE_TOKEN",
    "VOX_NEWS_OPENCOLLECTIVE_TOKEN",
    "VOX_NEWS_TWITTER_TEXT_CHUNK_MAX",
    "VOX_NEWS_TWITTER_TRUNCATION_SUFFIX",
    "VOX_SOCIAL_REDDIT_CLIENT_ID",
    "VOX_SOCIAL_REDDIT_CLIENT_SECRET",
    "VOX_SOCIAL_REDDIT_REFRESH_TOKEN",
    "VOX_SOCIAL_REDDIT_USER_AGENT",
    "VOX_SOCIAL_YOUTUBE_CLIENT_ID",
    "VOX_SOCIAL_YOUTUBE_CLIENT_SECRET",
    "VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN",
    "VOX_SOCIAL_HN_MODE",
    "VOX_SOCIAL_MASTODON_DOMAIN",
    "VOX_SOCIAL_MASTODON_ACCESS_TOKEN",
    "VOX_SOCIAL_BLUESKY_HANDLE",
    "VOX_SOCIAL_BLUESKY_PASSWORD",
    "VOX_SOCIAL_LINKEDIN_ACCESS_TOKEN",
    "VOX_SOCIAL_DISCORD_WEBHOOK",
    "VOX_SOCIAL_TWITTER_SUMMARY_MARGIN_CHARS",
    "VOX_SOCIAL_REDDIT_SELFPOST_SUMMARY_MAX",
    "VOX_SOCIAL_YOUTUBE_DEFAULT_CATEGORY_ID",
    "VOX_SCHOLARLY_ADAPTER",
    "VOX_SCHOLARLY_DISABLE",
    "VOX_SCHOLARLY_DISABLE_LIVE",
    "VOX_SOCIAL_LINKEDIN_API_BASE",
    "VOX_SOCIAL_REDDIT_API_BASE",
    "VOX_SCHOLARLY_DISABLE_ZENODO",
    "VOX_SCHOLARLY_DISABLE_OPENREVIEW",
    "VOX_OPENREVIEW_API_BASE",
    "VOX_OPENREVIEW_INVITATION",
    "VOX_OPENREVIEW_SIGNATURE",
    "VOX_OPENREVIEW_ACCESS_TOKEN",
    "OPENREVIEW_API_BASE",
    "OPENREVIEW_INVITATION",
    "OPENREVIEW_SIGNATURE",
    "OPENREVIEW_ACCESS_TOKEN",
    "OPENREVIEW_EMAIL",
    "OPENREVIEW_PASSWORD",
    "VOX_ZENODO_SANDBOX",
    "VOX_ZENODO_API_BASE",
    "VOX_ZENODO_HTTP_MAX_ATTEMPTS",
    "VOX_ZENODO_ATTACH_MANIFEST_BODY",
    "VOX_ZENODO_PUBLISH_DEPOSITION",
    "VOX_ZENODO_DRAFT_ONLY",
    "VOX_ZENODO_PUBLISH_NOW",
    "VOX_ZENODO_STAGING_DIR",
    "VOX_ZENODO_UPLOAD_ALLOWLIST",
    "VOX_ZENODO_VERIFY_STAGING_CHECKSUMS",
    "VOX_ZENODO_REQUIRE_METADATA_PARITY",
    "VOX_OPENREVIEW_HTTP_MAX_ATTEMPTS",
    "VOX_SOCIAL_WORTHINESS_ENFORCE",
    "VOX_SOCIAL_WORTHINESS_SCORE_MIN",
    "VOX_SOCIAL_LINKEDIN_API_BASE",
    "VOX_SOCIAL_REDDIT_API_BASE",
    "VOX_SOCIAL_TWITTER_API_BASE",
    "VOX_SOCIAL_LINKEDIN_AUTHOR_URN",
    "VOX_SOCIAL_BLUESKY_PDS_URL",
    "VOX_NEWS_OPENCOLLECTIVE_SLUG",
];

impl PublisherConfig {
    #[inline]
    fn syndication_secret(id: vox_secrets::SecretId) -> Option<String> {
        vox_secrets::resolve_secret(id)
            .expose()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn clear_route_simulation_env_overrides() {
        for &key in ROUTE_SIMULATION_ENV_KEYS {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    #[must_use]
    pub fn from_operator_environment(
        dry_run: bool,
        youtube_repo_root: Option<std::path::PathBuf>,
        site: NewsSiteConfig,
    ) -> Self {
        let env_opt = |k: vox_secrets::SecretId| {
            vox_secrets::resolve_secret(k)
                .expose()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let env_usize = |k: vox_secrets::SecretId| {
            env_opt(k).and_then(|v| match v.parse::<usize>() {
                Ok(n) => Some(n),
                Err(_) => {
                    warn!(
                        target: "vox.publisher.config",
                        key = ?k,
                        value = v,
                        "invalid usize env override; ignoring"
                    );
                    None
                }
            })
        };
        Self {
            twitter_bearer_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxNewsTwitterBearer,
            ),
            mastodon_domain: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialMastodonDomain,
            ),
            mastodon_access_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialMastodonToken,
            ),
            bluesky_handle: Self::syndication_secret(vox_secrets::SecretId::VoxSocialBlueskyHandle),
            bluesky_password: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialBlueskyPassword,
            ),
            linkedin_access_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialLinkedinAccessToken,
            ),
            forge_token: Self::syndication_secret(vox_secrets::SecretId::ForgeToken),
            open_collective_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxNewsOpenCollectiveToken,
            ),
            twitter_summary_margin_chars: env_usize(
                vox_secrets::SecretId::VoxSocialTwitterSummaryMarginChars,
            ),
            reddit_selfpost_summary_max: env_usize(
                vox_secrets::SecretId::VoxSocialRedditSelfpostSummaryMax,
            ),
            reddit_client_id: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialRedditClientId,
            ),
            reddit_client_secret: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialRedditClientSecret,
            ),
            reddit_refresh_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialRedditRefreshToken,
            ),
            reddit_user_agent: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialRedditUserAgent,
            ),
            youtube_client_id: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialYoutubeClientId,
            ),
            youtube_client_secret: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialYoutubeClientSecret,
            ),
            youtube_refresh_token: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialYoutubeRefreshToken,
            ),
            hacker_news_mode: env_opt(vox_secrets::SecretId::VoxSocialHnMode),
            youtube_default_category_id: env_opt(
                vox_secrets::SecretId::VoxSocialYoutubeDefaultCategoryId,
            ),
            twitter_text_chunk_max: env_usize(vox_secrets::SecretId::VoxNewsTwitterTextChunkMax),
            twitter_truncation_suffix: env_opt(
                vox_secrets::SecretId::VoxNewsTwitterTruncationSuffix,
            ),
            twitter_api_base: env_opt(vox_secrets::SecretId::VoxSocialTwitterApiBase),
            linkedin_api_base: env_opt(vox_secrets::SecretId::VoxSocialLinkedinApiBase),
            reddit_api_base: env_opt(vox_secrets::SecretId::VoxSocialRedditApiBase),
            discord_webhook_url: env_opt(vox_secrets::SecretId::VoxSocialDiscordWebhook),
            linkedin_author_urn: Self::syndication_secret(
                vox_secrets::SecretId::VoxSocialLinkedinAuthorUrn,
            ),
            bluesky_pds_url: Self::syndication_secret(vox_secrets::SecretId::VoxSocialBlueskyPdsUrl),
            open_collective_slug: Self::syndication_secret(
                vox_secrets::SecretId::VoxNewsOpenCollectiveSlug,
            ),
            youtube_repo_root,
            dry_run,
            site,
            ..Default::default()
        }
    }
}
