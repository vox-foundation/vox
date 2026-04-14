import re

print("Patching switching.rs")
with open('crates/vox-publisher/src/switching.rs', 'r', encoding='utf-8') as f:
    code = f.read()

code = code.replace("item.syndication.twitter = None;", "item.syndication.social.retain(|c| c != &crate::types::SocialChannel::Twitter);")
code = code.replace("item.syndication.hacker_news = None;", "item.syndication.hacker_news = false;")
code = code.replace("item.syndication.bluesky = None;", "item.syndication.social.retain(|c| c != &crate::types::SocialChannel::Bluesky);")
code = code.replace("item.syndication.mastodon = None;", "item.syndication.social.retain(|c| c != &crate::types::SocialChannel::Mastodon);")
code = code.replace("item.syndication.linkedin = None;", "item.syndication.linkedin = false;")
code = code.replace("item.syndication.discord = None;", "item.syndication.social.retain(|c| c != &crate::types::SocialChannel::Discord);")

with open('crates/vox-publisher/src/switching.rs', 'w', encoding='utf-8') as f:
    f.write(code)


print("Patching topic_packs.rs")
with open('crates/vox-publisher/src/topic_packs.rs', 'r', encoding='utf-8') as f:
    code = f.read()

code = code.replace("syn.twitter = None;", "syn.social.retain(|c| c != &crate::types::SocialChannel::Twitter);")
code = code.replace("syn.hacker_news = None;", "syn.hacker_news = false;")
code = code.replace("syn.bluesky = None;", "syn.social.retain(|c| c != &crate::types::SocialChannel::Bluesky);")
code = code.replace("syn.mastodon = None;", "syn.social.retain(|c| c != &crate::types::SocialChannel::Mastodon);")
code = code.replace("syn.linkedin = None;", "syn.linkedin = false;")
code = code.replace("syn.discord = None;", "syn.social.retain(|c| c != &crate::types::SocialChannel::Discord);")

with open('crates/vox-publisher/src/topic_packs.rs', 'w', encoding='utf-8') as f:
    f.write(code)


print("Patching distribution_compile.rs")
with open('crates/vox-publisher/src/distribution_compile.rs', 'r', encoding='utf-8') as f:
    code = f.read()

code = code.replace("item.syndication.twitter.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Twitter)")
code = code.replace("item.syndication.hacker_news.is_some()", "item.syndication.hacker_news")
code = code.replace("item.syndication.discord.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Discord)")
code = code.replace("item.syndication.mastodon.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Mastodon)")
code = code.replace("item.syndication.linkedin.is_some()", "item.syndication.linkedin")
code = code.replace("item.syndication.bluesky.is_some()", "item.syndication.social.contains(&crate::types::SocialChannel::Bluesky)")

with open('crates/vox-publisher/src/distribution_compile.rs', 'w', encoding='utf-8') as f:
    f.write(code)

