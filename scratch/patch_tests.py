import re

print("Fixing twitter tests")
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("twitter::post(&publisher_cfg, token, &item, &config, false)", "twitter::post(&publisher_cfg, token, &item, false)")
text = text.replace("twitter::post(&publisher_cfg, token, &item, &config, true)", "twitter::post(&publisher_cfg, token, &item, true)")
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing mastodon tests")
with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("mastodon::post(&publisher_cfg, &item, &cfg, false)", 'mastodon::post(&publisher_cfg, &item, Some(&cfg), "summary", false)')
text = text.replace("mastodon::post(&publisher_cfg, &item, &cfg, true)", 'mastodon::post(&publisher_cfg, &item, Some(&cfg), "summary", true)')
text = text.replace("let cfg = MastodonConfig {", "let cfg = MastodonOverride {")
with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing bluesky tests")
with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("bluesky::post(&publisher_cfg, handle, password, &pds_base, &item, &config, false)", "bluesky::post(&publisher_cfg, handle, password, &item, false)")
with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing linkedin tests")
with open("crates/vox-publisher/src/adapters/tests/linkedin.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("linkedin::post(&publisher_cfg, &item, &config, false)", "linkedin::post(&publisher_cfg, &item, false)")
with open("crates/vox-publisher/src/adapters/tests/linkedin.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing opencollective tests")
with open("crates/vox-publisher/src/adapters/tests/opencollective.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace('collective_slug: "test-collective".to_string(),\n', '')
with open("crates/vox-publisher/src/adapters/tests/opencollective.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing switching tests")
with open("crates/vox-publisher/src/switching.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("let tw = item.syndication.twitter.expect(\"twitter\");", 'assert!(item.syndication.social.contains(&crate::types::SocialChannel::Twitter));\nlet tw = item.syndication.twitter_override.as_ref().unwrap();')
with open("crates/vox-publisher/src/switching.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing topic_packs tests")
with open("crates/vox-publisher/src/topic_packs.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = re.sub(r'twitter:\s*Some\(crate::types::TwitterConfig \{[^\}]*\}\),', 'social: vec![crate::types::SocialChannel::Twitter],', text)
text = text.replace('assert!(syn.twitter.is_none());', 'assert!(!syn.social.contains(&crate::types::SocialChannel::Twitter));')
with open("crates/vox-publisher/src/topic_packs.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing distribution_compile tests")
with open("crates/vox-publisher/src/distribution_compile.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = re.sub(r'syn\.twitter\s*=\s*Some\(TwitterConfig \{[^\}]*?\}\);', 'syn.social.push(crate::types::SocialChannel::Twitter);', text)
with open("crates/vox-publisher/src/distribution_compile.rs", "w", encoding="utf-8") as f:
    f.write(text)

print("Fixing types.rs tests")
with open("crates/vox-publisher/src/types.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace('assert!(item.syndication.twitter.is_none());', 'assert!(!item.syndication.social.contains(&SocialChannel::Twitter));')
with open("crates/vox-publisher/src/types.rs", "w", encoding="utf-8") as f:
    f.write(text)

