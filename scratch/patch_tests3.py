import re

print("Patching tests 3")
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("let config = TwitterConfig::default();\n", "")
text = text.replace("twitter::post(&publisher_cfg, token, &item, &config, false)", "twitter::post(&publisher_cfg, token, &item, false)")
with open("crates/vox-publisher/src/adapters/tests/twitter.rs", "w", encoding="utf-8") as f:
    f.write(text)

with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("status: None,\n", "")
text = text.replace('visibility: "public".to_string(),', 'visibility: Some("public".to_string()),')
text = text.replace('MastodonConfig::default()', 'MastodonOverride::default()')
with open("crates/vox-publisher/src/adapters/tests/mastodon.rs", "w", encoding="utf-8") as f:
    f.write(text)

with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("let config = BlueskyConfig::default();\n", "")
text = text.replace("&config, ", "")
with open("crates/vox-publisher/src/adapters/tests/bluesky.rs", "w", encoding="utf-8") as f:
    f.write(text)

with open("crates/vox-publisher/src/adapters/tests/discord.rs", "r", encoding="utf-8") as f:
    text = f.read()
text = text.replace("&cfg, false)", "Some(&cfg), false)")
text = text.replace("&cfg, true)", "Some(&cfg), true)")
with open("crates/vox-publisher/src/adapters/tests/discord.rs", "w", encoding="utf-8") as f:
    f.write(text)

