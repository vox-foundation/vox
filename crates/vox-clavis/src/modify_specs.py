import sys

with open(r'c:\Users\Owner\vox\crates\vox-clavis\src\spec.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

new_specs = '''
    SecretSpec {
        id: SecretId::VoxSocialRedditClientId,
        canonical_env: "VOX_SOCIAL_REDDIT_CLIENT_ID",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_CLIENT_ID.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditClientSecret,
        canonical_env: "VOX_SOCIAL_REDDIT_CLIENT_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_CLIENT_SECRET.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditRefreshToken,
        canonical_env: "VOX_SOCIAL_REDDIT_REFRESH_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_REFRESH_TOKEN.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditUserAgent,
        canonical_env: "VOX_SOCIAL_REDDIT_USER_AGENT",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_USER_AGENT.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeClientId,
        canonical_env: "VOX_SOCIAL_YOUTUBE_CLIENT_ID",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_CLIENT_ID.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeClientSecret,
        canonical_env: "VOX_SOCIAL_YOUTUBE_CLIENT_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_CLIENT_SECRET.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeRefreshToken,
        canonical_env: "VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN.",
    },
    SecretSpec {
        id: SecretId::VoxSocialMastodonToken,
        canonical_env: "VOX_SOCIAL_MASTODON_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_MASTODON_TOKEN.",
    },
    SecretSpec {
        id: SecretId::VoxSocialMastodonDomain,
        canonical_env: "VOX_SOCIAL_MASTODON_DOMAIN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_MASTODON_DOMAIN.",
    },
    SecretSpec {
        id: SecretId::VoxSocialLinkedinAccessToken,
        canonical_env: "VOX_SOCIAL_LINKEDIN_ACCESS_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_LINKEDIN_ACCESS_TOKEN.",
    },
    SecretSpec {
        id: SecretId::VoxSocialDiscordWebhook,
        canonical_env: "VOX_SOCIAL_DISCORD_WEBHOOK_URL",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_DISCORD_WEBHOOK_URL.",
    },
'''

for i in range(len(lines)):
    if 'pub fn secret_reads_populi_env_file' in lines[i]:
        # Backtrack to the nearest ]; line
        for j in range(i, 0, -1):
            if '];' in lines[j] and '}' in lines[j-1]:
                # Found the end of SPECS.
                lines.insert(j, new_specs)
                break
        break

with open(r'c:\Users\Owner\vox\crates\vox-clavis\src\spec.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)
print('Added specs.')
