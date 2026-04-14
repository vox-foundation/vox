import re

print("Fixing twitter tests")
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("use crate::types::{TwitterConfig, UnifiedNewsItem};", "use crate::types::UnifiedNewsItem;")
text = re.sub(r'let config = TwitterConfig \{[\s\S]*?\};\n', '', text)
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing mastodon tests")
with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("use crate::types::MastodonConfig;", "use crate::types::MastodonOverride;")
text = text.replace("status: Some(\"This is a status\".to_string()),\n", "")
text = text.replace("sensitive: false,\n", "")
with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing bluesky tests")
with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("use crate::types::{BlueskyConfig, UnifiedNewsItem};", "use crate::types::UnifiedNewsItem;")
text = re.sub(r'let config = BlueskyConfig \{[\s\S]*?\};\n', '', text)
with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing linkedin tests")
with open("crates/vox-publisher/src/adapters/tests/linkedin.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("use crate::types::{LinkedInConfig, UnifiedNewsItem};", "use crate::types::UnifiedNewsItem;")
text = re.sub(r'let config = LinkedInConfig \{[\s\S]*?\};\n', '', text)
with open("crates/vox-publisher/src/adapters/tests/linkedin.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing discord tests")
with open("crates/vox-publisher/src/adapters/tests/discord.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("use crate::types::DiscordConfig;", "use crate::types::DiscordOverride;")
text = text.replace("DiscordConfig", "DiscordOverride")
text = text.replace("embed_title: None,\n", "")
text = text.replace("embed_description: None,\n", "")
text = text.replace("embed_url: None,\n", "")
text = text.replace("embed_color: None,\n", "")
text = text.replace("tts: false,\n", "")
with open("crates/vox-publisher/src/adapters/tests/discord.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fix distribution compile")
with open("crates/vox-publisher/src/distribution_compile.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("ChannelPolicyConfig, SyndicationConfig, TwitterConfig, UnifiedNewsItem};", "ChannelPolicyConfig, SyndicationConfig, UnifiedNewsItem};")
with open("crates/vox-publisher/src/distribution_compile.rs", "w", encoding="utf-8") as f:
    f.write(text)

