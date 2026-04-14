import re

with open('crates/vox-publisher/src/publisher/mod.rs', 'r', encoding='utf-8') as f:
    code = f.read()

# Replace twitter logic entirely
code = re.sub(
    r'let derived_twitter: Option<TwitterConfig>.*?\n        \};\n',
    '',
    code,
    flags=re.DOTALL
)

code = code.replace(
    'if let Some(twitter) = &derived_twitter {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Twitter) {'
)
code = code.replace(
    'adapters::twitter::post(&self.config, credential_token, item, twitter, is_dry_run)',
    'adapters::twitter::post(&self.config, credential_token, item, item.syndication.twitter_override.as_ref(), "summary", is_dry_run)'
)
code = code.replace(
    'adapters::twitter::post(&self.config, token, item, twitter, is_dry_run)',
    'adapters::twitter::post(&self.config, token, item, item.syndication.twitter_override.as_ref(), "summary", is_dry_run)'
)


code = code.replace(
    'if let Some(bluesky) = &item.syndication.bluesky {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Bluesky) {'
)
code = code.replace(
    'adapters::bluesky::post(&self.config, handle, password, &bluesky.pds_url, item, bluesky, is_dry_run)',
    'adapters::bluesky::post(&self.config, handle, password, item, None, "summary", is_dry_run)'
)

code = code.replace(
    'if let Some(mastodon) = &item.syndication.mastodon {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Mastodon) {'
)
code = code.replace(
    'adapters::mastodon::post(&self.config, item, mastodon, is_dry_run)',
    'adapters::mastodon::post(&self.config, item, item.syndication.mastodon_override.as_ref(), "summary", is_dry_run)'
)

code = code.replace(
    'if let Some(linkedin) = &item.syndication.linkedin {',
    'if item.syndication.linkedin {'
)
code = code.replace(
    'adapters::linkedin::post(&self.config, item, linkedin, is_dry_run)',
    'adapters::linkedin::post(&self.config, item, is_dry_run)'
)

code = code.replace(
    'if let Some(discord) = &item.syndication.discord {',
    'if item.syndication.social.contains(&crate::types::SocialChannel::Discord) {'
)
code = code.replace(
    'adapters::discord::post(&self.config, item, discord, is_dry_run)',
    'adapters::discord::post(&self.config, item, item.syndication.discord_override.as_ref(), is_dry_run)'
)

code = code.replace(
    'if let Some(rg) = &item.syndication.researchgate {',
    'if item.syndication.researchgate {'
)
code = code.replace(
    'adapters::researchgate::post(item, rg, is_dry_run)',
    'adapters::researchgate::post(item, is_dry_run)'
)

code = code.replace(
    'let derived_hn: Option<HackerNewsConfig> =\n            item.syndication.hacker_news.clone().map(|mut cfg| {',
    'let derived_hn: Option<crate::types::HackerNewsConfig> = if item.syndication.hacker_news { Some(crate::types::HackerNewsConfig { mode: crate::types::HackerNewsMode::ManualAssist, comment_draft: None, title_override: None, url_override: None }) } else { None }.map(|mut cfg| {'
)

with open('crates/vox-publisher/src/publisher/mod.rs', 'w', encoding='utf-8') as f:
    f.write(code)

with open('crates/vox-publisher/src/adapters/linkedin.rs', 'r', encoding='utf-8') as f:
    lnk = f.read()

lnk = lnk.replace('cfg: &LinkedInConfig,', '')
lnk = lnk.replace('use crate::types::{LinkedInConfig, UnifiedNewsItem};', 'use crate::types::UnifiedNewsItem;')

with open('crates/vox-publisher/src/adapters/linkedin.rs', 'w', encoding='utf-8') as f:
    f.write(lnk)

with open('crates/vox-publisher/src/adapters/researchgate.rs', 'r', encoding='utf-8') as f:
    rg = f.read()

rg = rg.replace('cfg: &ResearchGateConfig,', '')
rg = rg.replace('use crate::types::{ResearchGateConfig, UnifiedNewsItem};', 'use crate::types::UnifiedNewsItem;')

with open('crates/vox-publisher/src/adapters/researchgate.rs', 'w', encoding='utf-8') as f:
    f.write(rg)

