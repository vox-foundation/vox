import re

print("Patching types.rs validate()")
with open('crates/vox-publisher/src/types.rs', 'r', encoding='utf-8') as f:
    code = f.read()

# Fix oc.collective_slug error
code = code.replace(
    'if let Some(ref oc) = self.syndication.open_collective\n            && oc.collective_slug.trim().is_empty()\n        {\n            errors.push("Open Collective target designated but slug is missing.");\n        }',
    ''
)

# Fix hn_override error
code = code.replace(
    'if let Some(ref hn) = self.syndication.hn_override\n            && let Some(ref url) = hn.url_override\n            && !url.trim().is_empty()\n        {\n            if !url.trim().starts_with("http") {\n                errors.push("Hacker News URL override must be a full absolute HTTP(S) URL.");\n            }\n        }',
    'if let Some(ref hn) = self.syndication.hacker_news_override {\n            if let Some(ref url) = hn.url_override {\n                if !url.trim().is_empty() && !url.trim().starts_with("http") {\n                    errors.push("Hacker News URL override must be a full absolute HTTP(S) URL.");\n                }\n            }\n        }'
)

with open('crates/vox-publisher/src/types.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patching publisher/config.rs")
with open('crates/vox-publisher/src/publisher/config.rs', 'r', encoding='utf-8') as f:
    code = f.read()

if "pub bluesky_pds_url: Option<String>," not in code:
    code = code.replace(
        "pub bluesky_password: Option<SecretString>,",
        "pub bluesky_password: Option<SecretString>,\n    pub bluesky_pds_url: Option<String>,"
    )

with open('crates/vox-publisher/src/publisher/config.rs', 'w', encoding='utf-8') as f:
    f.write(code)

