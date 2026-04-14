import re

with open('crates/vox-publisher/src/publisher/mod.rs', 'r', encoding='utf-8') as f:
    code = f.read()

# Fix imports
code = code.replace(
    'use crate::types::{HackerNewsConfig, HackerNewsMode, TwitterConfig, UnifiedNewsItem};',
    'use crate::types::{HackerNewsMode, UnifiedNewsItem};'
)
code = code.replace(
    'use crate::types::{BlueskyConfig, ChannelOutcome, ChannelsOutcome, CratesIoConfig, DiscordConfig, DistributionPolicyConfig, ForgeConfig, ForgePostType, HackerNewsConfig, LinkedInConfig, MastodonConfig, OpenCollectiveConfig, RedditConfig, RedditPostKind, SyndicationConfig, TwitterConfig, UnifiedNewsItem, YouTubeConfig};',
    'use crate::types::{ChannelOutcome, ChannelsOutcome, CratesIoConfig, DistributionPolicyConfig, ForgeConfig, ForgePostType, OpenCollectiveConfig, RedditConfig, RedditPostKind, SyndicationConfig, UnifiedNewsItem, YouTubeConfig, SocialChannel};'
)

# Replace derived_twitter block
code = re.sub(
    r'let derived_twitter: Option<TwitterConfig>.*?\n        \};\n',
    '',
    code,
    flags=re.DOTALL
)

# Replace derived_hn block
code = re.sub(
    r'let derived_hn: Option<HackerNewsConfig> =\n\s*item.syndication.hacker_news.clone\(\).map\(\|mut cfg\|\s*\{.*?\n\s*\}\);\n',
    '',
    code,
    flags=re.DOTALL
)

# Fix open collective slug error
code = code.replace('oc.collective_slug', 'self.config.open_collective_slug.as_deref().unwrap_or("vox-foundation")')

# Replace social channel blocks
code = code.replace(
    'if let Some(twitter) = &derived_twitter {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Twitter) {'
)
code = re.sub(
    r'adapters::twitter::post\(&self\.config, token\.as_str\(\), item, twitter, is_dry_run\)',
    'adapters::twitter::post(&self.config, token.as_str(), item, is_dry_run)',
    code
)
code = code.replace('"{:?}", twitter', '"{:?}", "enabled"')

code = code.replace(
    'if let Some(bluesky) = &item.syndication.bluesky {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Bluesky) {'
)
code = re.sub(
    r'adapters::bluesky::post\(&self\.config, handle, password, &bluesky\.pds_url, item, bluesky, is_dry_run\)',
    'adapters::bluesky::post(&self.config, handle, password, item, is_dry_run)',
    code
)

code = code.replace(
    'if let Some(mastodon) = &item.syndication.mastodon {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Mastodon) {'
)
code = re.sub(
    r'adapters::mastodon::post\(&self\.config, item, mastodon, is_dry_run\)',
    'adapters::mastodon::post(&self.config, item, item.syndication.mastodon_override.as_ref(), "", is_dry_run)',
    code
)

code = code.replace(
    'if let Some(linkedin) = &item.syndication.linkedin {',
    'if item.syndication.linkedin {'
)
code = re.sub(
    r'adapters::linkedin::post\(&self\.config, item, linkedin, is_dry_run\)',
    'adapters::linkedin::post(&self.config, item, is_dry_run)',
    code
)

code = code.replace(
    'if let Some(discord) = &item.syndication.discord {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Discord) {'
)
code = re.sub(
    r'adapters::discord::post\(&self\.config, item, discord, is_dry_run\)',
    'adapters::discord::post(&self.config, item, item.syndication.discord_override.as_ref(), is_dry_run)',
    code
)

code = code.replace(
    'if let Some(rg) = &item.syndication.researchgate {',
    'if item.syndication.researchgate {'
)
code = re.sub(
    r'adapters::researchgate::post\(item, rg, is_dry_run\)',
    'adapters::researchgate::post(item, is_dry_run)',
    code
)

with open('crates/vox-publisher/src/publisher/mod.rs', 'w', encoding='utf-8') as f:
    f.write(code)

# Fix configs in adapters
print("Fixing adapters")
# Twitter
with open('crates/vox-publisher/src/adapters/twitter.rs', 'r', encoding='utf-8') as f:
    tw = f.read()
tw = tw.replace('pub async fn post(\n    publisher_cfg: &PublisherConfig,\n    token: &str,\n    item: &UnifiedNewsItem,\n    config: &TwitterConfig,\n    dry_run: bool,\n)', 'pub async fn post(\n    publisher_cfg: &PublisherConfig,\n    token: &str,\n    item: &UnifiedNewsItem,\n    dry_run: bool,\n)')
tw = tw.replace('let primary_text = config\n        .short_text\n        .clone()\n        .unwrap_or_else(|| truncate_chars(&item.content_markdown, chunk_max, truncation_suffix));', 'let primary_text = truncate_chars(&item.content_markdown, chunk_max, truncation_suffix);')
tw = tw.replace('let mut texts = if config.thread {', 'let mut texts = if true {')
tw = tw.replace('let full = config\n            .short_text\n            .clone()\n            .unwrap_or_else(|| item.content_markdown.clone());', 'let full = item.content_markdown.clone();')
with open('crates/vox-publisher/src/adapters/twitter.rs', 'w', encoding='utf-8') as f:
    f.write(tw)

# Bluesky
with open('crates/vox-publisher/src/adapters/bluesky.rs', 'r', encoding='utf-8') as f:
    bsky = f.read()
bsky = bsky.replace('use crate::types::{BlueskyOverride, UnifiedNewsItem};', 'use crate::types::UnifiedNewsItem;')
bsky = bsky.replace('cfg: Option<&BlueskyOverride>,', '')
bsky = bsky.replace('short_summary: &str,', '')
bsky = bsky.replace('let text = short_summary.to_string();', 'let text = item.content_markdown.clone();')
bsky = bsky.replace('pds_base.', 'base.')
bsky = re.sub(r'pub async fn post\([\s\S]*?dry_run: bool,\n\) -> Result<String> \{', 'pub async fn post(\n    publisher_cfg: &PublisherConfig,\n    handle: &str,\n    password: &str,\n    item: &UnifiedNewsItem,\n    dry_run: bool,\n) -> Result<String> {', bsky)
with open('crates/vox-publisher/src/adapters/bluesky.rs', 'w', encoding='utf-8') as f:
    f.write(bsky)

# LinkedIn
with open('crates/vox-publisher/src/adapters/linkedin.rs', 'r', encoding='utf-8') as f:
    li = f.read()
li = li.replace('let text = cfg.text.clone().unwrap_or_else(|| item.content_markdown.clone());', 'let text = item.content_markdown.clone();')
li = li.replace('let visibility = cfg.visibility.clone().unwrap_or_else(|| "PUBLIC".to_string());', 'let visibility = "PUBLIC".to_string();')
li = li.replace('"author": cfg.author_urn,', '"author": publisher_cfg.linkedin_author_urn.as_deref().unwrap_or(""),')
with open('crates/vox-publisher/src/adapters/linkedin.rs', 'w', encoding='utf-8') as f:
    f.write(li)

# Publisher config initialization missing
with open('crates/vox-publisher/src/publisher/config.rs', 'r', encoding='utf-8') as f:
    pcfg = f.read()
if "linkedin_author_urn: None," not in pcfg:
    pcfg = pcfg.replace('discord_webhook_url: None,', 'discord_webhook_url: None,\n            linkedin_author_urn: None,\n            open_collective_slug: None,')
with open('crates/vox-publisher/src/publisher/config.rs', 'w', encoding='utf-8') as f:
    f.write(pcfg)
