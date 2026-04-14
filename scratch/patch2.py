import sys
import re

print("Patching derivation.rs")
with open('crates/vox-publisher/src/publication_preflight/derivation.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = code.replace("item.syndication.hacker_news.is_some()", "item.syndication.hacker_news")
code = code.replace("item.syndication.twitter.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Twitter)")
code = code.replace("item.syndication.discord.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Discord)")
with open('crates/vox-publisher/src/publication_preflight/derivation.rs', 'w', encoding='utf-8') as f:
    f.write(code)


print("Patching types.rs")
with open('crates/vox-publisher/src/types.rs', 'r', encoding='utf-8') as f:
    code = f.read()
# Replace ALL CratesIoConfig structs and insert one instance at the end
code = re.sub(
    r'#\[derive\(Debug, Clone, Serialize, Deserialize, Default\)\]\s*pub struct CratesIoConfig \{.*?\}\n',
    '',
    code, flags=re.DOTALL
)
code += '\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct CratesIoConfig {\n    pub crates_to_update: Vec<String>,\n}\n'
with open('crates/vox-publisher/src/types.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching canary.rs")
with open('crates/vox-publisher/src/adapters/canary.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = re.sub(r'use crate::PublisherConfig;\nuse crate::adapter_health::HeartbeatStatus;\nuse reqwest::Client;\nuse std::time::Instant;\n', '', code)
with open('crates/vox-publisher/src/adapters/canary.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching adapter_health.rs")
with open('crates/vox-publisher/src/adapter_health.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = code.replace('use crate::adapters::canary;\n', '')
with open('crates/vox-publisher/src/adapter_health.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching publisher/config.rs")
with open('crates/vox-publisher/src/publisher/config.rs', 'r', encoding='utf-8') as f:
    code = f.read()
if "pub open_collective_slug: Option<String>," not in code:
    code = code.replace("pub discord_webhook_url: Option<String>,", "pub discord_webhook_url: Option<String>,\n    pub open_collective_slug: Option<String>,\n    pub linkedin_author_urn: Option<String>,")
with open('crates/vox-publisher/src/publisher/config.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching adapters/bluesky.rs missing arg")
with open('crates/vox-publisher/src/adapters/bluesky.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = code.replace('use crate::types::{BlueskyConfig, UnifiedNewsItem};', 'use crate::types::{BlueskyOverride, UnifiedNewsItem};')
code = code.replace(
    'pub async fn post(\n    _publisher_cfg: &PublisherConfig,\n    handle: &str,\n    password: &str,\n    pds_base: &str,\n    item: &UnifiedNewsItem,\n    config: &BlueskyConfig,\n    dry_run: bool,\n)',
    'pub async fn post(\n    publisher_cfg: &PublisherConfig,\n    handle: &str,\n    password: &str,\n    item: &UnifiedNewsItem,\n    cfg: Option<&BlueskyOverride>,\n    short_summary: &str,\n    dry_run: bool,\n)'
)
code = code.replace(
    'let text = config\n        .text\n        .clone()\n        .unwrap_or_else(|| item.content_markdown.clone());',
    'let text = short_summary.to_string();'
)
code = code.replace('let base = pds_base.trim_end_matches(\'/\');', 'let base = publisher_cfg.bluesky_pds_url.as_deref().unwrap_or(DEFAULT_PDS).trim_end_matches(\'/\');')
with open('crates/vox-publisher/src/adapters/bluesky.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching adapters/twitter.rs unused override")
with open('crates/vox-publisher/src/adapters/twitter.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = code.replace('use crate::types::{TwitterOverride, UnifiedNewsItem};', 'use crate::types::UnifiedNewsItem;')
code = code.replace('cfg: Option<&TwitterOverride>,', '')
with open('crates/vox-publisher/src/adapters/twitter.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching adapters/researchgate.rs")
with open('crates/vox-publisher/src/adapters/researchgate.rs', 'r', encoding='utf-8') as f:
    code = f.read()
code = code.replace('pub async fn post(\n    _item: &UnifiedNewsItem,\n    _config: &ResearchGateConfig,\n    dry_run: bool,\n)', 'pub async fn post(\n    _item: &UnifiedNewsItem,\n    dry_run: bool,\n)')
with open('crates/vox-publisher/src/adapters/researchgate.rs', 'w', encoding='utf-8') as f:
    f.write(code)
