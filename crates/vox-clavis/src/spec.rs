use std::collections::BTreeSet;

use crate::policy::SecretPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Workflow {
    Chat,
    Mcp,
    Publish,
    Review,
    DbRemote,
    MensMesh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    Dev,
    Ci,
    Mobile,
    Prod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequirementMode {
    Auto,
    Local,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretBundle {
    MinimalLocalDev,
    MinimalCloudDev,
    GpuCloud,
    PublishReview,
    /// Role-scoped populi mesh tokens (split worker/submitter/admin); see `mesh_roles` in clavis SSOT.
    MeshRoles,
}

impl SecretBundle {
    #[must_use]
    pub const fn doc_name(self) -> &'static str {
        match self {
            Self::MinimalLocalDev => "minimal_local_dev",
            Self::MinimalCloudDev => "minimal_cloud_dev",
            Self::GpuCloud => "gpu_cloud",
            Self::PublishReview => "publish_review",
            Self::MeshRoles => "mesh_roles",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    ChatCloudPrimary,
    ChatCloudAlt,
    GpuCloud,
    PublishReview,
    DbRemote,
    Mesh,
    RuntimeIngress,
    AuxTools,
    /// Optional tokens for Scientia / news syndication adapters (`VOX_NEWS_*`, `VOX_SOCIAL_*`).
    ScientiaSyndication,
    /// Optional tokens for live scholarly repository adapters (Zenodo, OpenReview, Crossref, arXiv assist).
    ScholarlyPublication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecretId {
    GeminiApiKey,
    OpenRouterApiKey,
    OpenAiApiKey,
    AnthropicApiKey,
    HuggingFaceToken,
    GitHubToken,
    GroqApiKey,
    CerebrasApiKey,
    MistralApiKey,
    DeepSeekApiKey,
    SambaNovaApiKey,
    CustomOpenAiApiKey,
    V0ApiKey,
    OpenClawToken,
    TogetherApiKey,
    VoxRunpodApiKey,
    VoxVastApiKey,
    VoxApiKey,
    VoxBearerToken,
    VoxDbUrl,
    VoxDbToken,
    VoxMeshToken,
    /// Opaque bearer for populi workers (join/heartbeat/inbox/ack on role-scoped meshes).
    VoxMeshWorkerToken,
    /// Opaque bearer for job submitters (`/v1/populi/a2a/deliver` with workload submissions).
    VoxMeshSubmitterToken,
    /// Admin bearer for moderation operations on the populi control plane.
    VoxMeshAdminToken,
    /// Shared secret for HS256 `Authorization: Bearer <jwt>` mesh control-plane auth (claim `role` + `jti` replay cache).
    VoxMeshJwtHmacSecret,
    /// Ed25519 verifying key (raw 32 bytes as hex or Standard base64) for optional `job_result` / `job_fail` payload attestations.
    VoxMeshWorkerResultVerifyKey,
    VoxNewsTwitterBearer,
    VoxNewsOpenCollectiveToken,
    VoxSocialRedditClientId,
    VoxSocialRedditClientSecret,
    VoxSocialRedditRefreshToken,
    VoxSocialRedditUserAgent,
    VoxSocialYoutubeClientId,
    VoxSocialYoutubeClientSecret,
    VoxSocialYoutubeRefreshToken,
    /// Zenodo REST API personal access token (depositions / uploads).
    VoxZenodoAccessToken,
    /// OpenReview login identifier (typically the registered email).
    VoxOpenReviewEmail,
    /// OpenReview account password for API/session flows.
    VoxOpenReviewPassword,
    /// Crossref Metadata Plus / Plus API key for metadata deposits (optional).
    VoxCrossrefPlusApiKey,
    /// Shared operator secret acknowledging an arXiv assist / handoff step (optional guardrail).
    VoxArxivAssistHandoffSecret,
}

#[derive(Debug, Clone, Copy)]
pub struct SecretSpec {
    pub id: SecretId,
    pub canonical_env: &'static str,
    pub aliases: &'static [&'static str],
    pub deprecated_aliases: &'static [&'static str],
    pub backend_key: Option<&'static str>,
    pub auth_registry: Option<&'static str>,
    pub policy: SecretPolicy,
    pub remediation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementSet {
    AnyOf(&'static [SecretId]),
    AllOf(&'static [SecretId]),
}

#[derive(Debug, Clone)]
pub struct WorkflowRequirements {
    pub blocking: Vec<RequirementSet>,
    pub optional: Vec<SecretId>,
}

const ALL_CHAT_OPTIONALS: &[SecretId] = &[
    SecretId::GeminiApiKey,
    SecretId::OpenAiApiKey,
    SecretId::AnthropicApiKey,
    SecretId::HuggingFaceToken,
    SecretId::GroqApiKey,
    SecretId::CerebrasApiKey,
    SecretId::MistralApiKey,
    SecretId::DeepSeekApiKey,
    SecretId::SambaNovaApiKey,
    SecretId::CustomOpenAiApiKey,
];

const CHAT_CLOUD_PRIMARY: &[SecretId] = &[SecretId::OpenRouterApiKey];
const BUNDLE_DOC_NAMES: &[&str] = &[
    "minimal_local_dev",
    "minimal_cloud_dev",
    "gpu_cloud",
    "publish_review",
    "mesh_roles",
];

const SPECS: &[SecretSpec] = &[
    SecretSpec {
        id: SecretId::GeminiApiKey,
        canonical_env: "GEMINI_API_KEY",
        aliases: &["VOX_GEMINI_API_KEY"],
        deprecated_aliases: &["GOOGLE_AI_STUDIO_KEY"],
        backend_key: None,
        auth_registry: Some("google"),
        policy: SecretPolicy::required_fail(),
        remediation: "Run `vox clavis set google <token>` or set GEMINI_API_KEY.",
    },
    SecretSpec {
        id: SecretId::OpenRouterApiKey,
        canonical_env: "OPENROUTER_API_KEY",
        aliases: &["VOX_OPENROUTER_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: Some("openrouter"),
        policy: SecretPolicy::required_fail(),
        remediation: "Run `vox clavis set openrouter <token>` or set OPENROUTER_API_KEY.",
    },
    SecretSpec {
        id: SecretId::OpenAiApiKey,
        canonical_env: "OPENAI_API_KEY",
        aliases: &["VOX_OPENAI_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set OPENAI_API_KEY when OpenAI routes are enabled.",
    },
    SecretSpec {
        id: SecretId::AnthropicApiKey,
        canonical_env: "ANTHROPIC_API_KEY",
        aliases: &["VOX_ANTHROPIC_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set ANTHROPIC_API_KEY when Anthropic routes are enabled.",
    },
    SecretSpec {
        id: SecretId::HuggingFaceToken,
        canonical_env: "HF_TOKEN",
        aliases: &["VOX_HF_TOKEN"],
        deprecated_aliases: &["HUGGING_FACE_HUB_TOKEN"],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set HF_TOKEN if Hugging Face routes are needed.",
    },
    SecretSpec {
        id: SecretId::GitHubToken,
        canonical_env: "GITHUB_TOKEN",
        aliases: &["VOX_GITHUB_TOKEN"],
        deprecated_aliases: &["GH_TOKEN"],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::required_fail(),
        remediation: "Set GITHUB_TOKEN (or GH_TOKEN) for GitHub API flows.",
    },
    SecretSpec {
        id: SecretId::GroqApiKey,
        canonical_env: "GROQ_API_KEY",
        aliases: &["VOX_GROQ_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set GROQ_API_KEY when Groq routes are enabled.",
    },
    SecretSpec {
        id: SecretId::CerebrasApiKey,
        canonical_env: "CEREBRAS_API_KEY",
        aliases: &["VOX_CEREBRAS_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set CEREBRAS_API_KEY when Cerebras routes are enabled.",
    },
    SecretSpec {
        id: SecretId::MistralApiKey,
        canonical_env: "MISTRAL_API_KEY",
        aliases: &["VOX_MISTRAL_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set MISTRAL_API_KEY when Mistral routes are enabled.",
    },
    SecretSpec {
        id: SecretId::DeepSeekApiKey,
        canonical_env: "DEEPSEEK_API_KEY",
        aliases: &["VOX_DEEPSEEK_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set DEEPSEEK_API_KEY when DeepSeek routes are enabled.",
    },
    SecretSpec {
        id: SecretId::SambaNovaApiKey,
        canonical_env: "SAMBANOVA_API_KEY",
        aliases: &["VOX_SAMBANOVA_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set SAMBANOVA_API_KEY when SambaNova routes are enabled.",
    },
    SecretSpec {
        id: SecretId::CustomOpenAiApiKey,
        canonical_env: "CUSTOM_OPENAI_API_KEY",
        aliases: &["VOX_CUSTOM_OPENAI_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set CUSTOM_OPENAI_API_KEY when custom OpenAI-compatible routes are enabled.",
    },
    SecretSpec {
        id: SecretId::V0ApiKey,
        canonical_env: "V0_API_KEY",
        aliases: &["VOX_V0_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set V0_API_KEY for island generation against v0.dev.",
    },
    SecretSpec {
        id: SecretId::OpenClawToken,
        canonical_env: "VOX_OPENCLAW_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_OPENCLAW_TOKEN for protected OpenClaw gateway operations.",
    },
    SecretSpec {
        id: SecretId::TogetherApiKey,
        canonical_env: "TOGETHER_API_KEY",
        aliases: &["VOX_TOGETHER_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set TOGETHER_API_KEY for Together training flows.",
    },
    SecretSpec {
        id: SecretId::VoxRunpodApiKey,
        canonical_env: "VOX_RUNPOD_API_KEY",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_RUNPOD_API_KEY for RunPod cloud training/serve jobs.",
    },
    SecretSpec {
        id: SecretId::VoxVastApiKey,
        canonical_env: "VOX_VAST_API_KEY",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_VAST_API_KEY for Vast.ai cloud training/serve jobs.",
    },
    SecretSpec {
        id: SecretId::VoxApiKey,
        canonical_env: "VOX_API_KEY",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_API_KEY to require API key auth on runtime servers.",
    },
    SecretSpec {
        id: SecretId::VoxBearerToken,
        canonical_env: "VOX_BEARER_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_BEARER_TOKEN to require bearer auth on runtime servers.",
    },
    SecretSpec {
        id: SecretId::VoxDbUrl,
        canonical_env: "VOX_DB_URL",
        aliases: &[],
        deprecated_aliases: &["VOX_TURSO_URL", "TURSO_URL"],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_DB_URL for remote DB use.",
    },
    SecretSpec {
        id: SecretId::VoxDbToken,
        canonical_env: "VOX_DB_TOKEN",
        aliases: &[],
        deprecated_aliases: &["VOX_TURSO_TOKEN", "TURSO_AUTH_TOKEN"],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_DB_TOKEN for remote DB use.",
    },
    SecretSpec {
        id: SecretId::VoxMeshToken,
        canonical_env: "VOX_MESH_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Run `vox populi up` or set VOX_MESH_TOKEN for mesh auth.",
    },
    SecretSpec {
        id: SecretId::VoxMeshWorkerToken,
        canonical_env: "VOX_MESH_WORKER_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_MESH_WORKER_TOKEN for populi worker-scoped control-plane auth.",
    },
    SecretSpec {
        id: SecretId::VoxMeshSubmitterToken,
        canonical_env: "VOX_MESH_SUBMITTER_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_MESH_SUBMITTER_TOKEN for populi job-submitter auth (A2A deliver).",
    },
    SecretSpec {
        id: SecretId::VoxMeshAdminToken,
        canonical_env: "VOX_MESH_ADMIN_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_MESH_ADMIN_TOKEN for populi admin-only control-plane operations.",
    },
    SecretSpec {
        id: SecretId::VoxMeshJwtHmacSecret,
        canonical_env: "VOX_MESH_JWT_HMAC_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Optional HS256 secret for JWT mesh bearer tokens (set on server + issuer).",
    },
    SecretSpec {
        id: SecretId::VoxMeshWorkerResultVerifyKey,
        canonical_env: "VOX_MESH_WORKER_RESULT_VERIFY_KEY",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Optional Ed25519 public key to verify signed `job_result` / `job_fail` deliveries (worker signs BLAKE3 digest).",
    },
    SecretSpec {
        id: SecretId::VoxNewsTwitterBearer,
        canonical_env: "VOX_NEWS_TWITTER_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_NEWS_TWITTER_TOKEN (or Clavis env/backend resolution) for Twitter syndication.",
    },
    SecretSpec {
        id: SecretId::VoxNewsOpenCollectiveToken,
        canonical_env: "VOX_NEWS_OPENCOLLECTIVE_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_NEWS_OPENCOLLECTIVE_TOKEN for Open Collective updates.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditClientId,
        canonical_env: "VOX_SOCIAL_REDDIT_CLIENT_ID",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_CLIENT_ID when Reddit syndication is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditClientSecret,
        canonical_env: "VOX_SOCIAL_REDDIT_CLIENT_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_CLIENT_SECRET when Reddit syndication is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditRefreshToken,
        canonical_env: "VOX_SOCIAL_REDDIT_REFRESH_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_REFRESH_TOKEN when Reddit syndication is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialRedditUserAgent,
        canonical_env: "VOX_SOCIAL_REDDIT_USER_AGENT",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_REDDIT_USER_AGENT when Reddit syndication is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeClientId,
        canonical_env: "VOX_SOCIAL_YOUTUBE_CLIENT_ID",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_CLIENT_ID when YouTube upload is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeClientSecret,
        canonical_env: "VOX_SOCIAL_YOUTUBE_CLIENT_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_CLIENT_SECRET when YouTube upload is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxSocialYoutubeRefreshToken,
        canonical_env: "VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN when YouTube upload is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxZenodoAccessToken,
        canonical_env: "ZENODO_ACCESS_TOKEN",
        aliases: &["VOX_ZENODO_ACCESS_TOKEN"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set ZENODO_ACCESS_TOKEN when Zenodo deposition submission is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxOpenReviewEmail,
        canonical_env: "OPENREVIEW_EMAIL",
        aliases: &["VOX_OPENREVIEW_EMAIL"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set OPENREVIEW_EMAIL when OpenReview submission flows are enabled.",
    },
    SecretSpec {
        id: SecretId::VoxOpenReviewPassword,
        canonical_env: "OPENREVIEW_PASSWORD",
        aliases: &["VOX_OPENREVIEW_PASSWORD"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set OPENREVIEW_PASSWORD when OpenReview submission flows are enabled.",
    },
    SecretSpec {
        id: SecretId::VoxCrossrefPlusApiKey,
        canonical_env: "CROSSREF_PLUS_API_KEY",
        aliases: &["VOX_CROSSREF_PLUS_API_KEY"],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set CROSSREF_PLUS_API_KEY when Crossref metadata deposit is enabled.",
    },
    SecretSpec {
        id: SecretId::VoxArxivAssistHandoffSecret,
        canonical_env: "VOX_ARXIV_ASSIST_HANDOFF_SECRET",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_ARXIV_ASSIST_HANDOFF_SECRET to gate operator arXiv handoff acks.",
    },
];

impl SecretId {
    #[must_use]
    pub fn spec(self) -> SecretSpec {
        SPECS
            .iter()
            .copied()
            .find(|s| s.id == self)
            .expect("SecretId must exist in SPECS")
    }
}

/// Secrets resolved from `.vox/populi/mesh.env` when `ResolveOptions.include_populi_env` is set.
#[must_use]
pub const fn secret_reads_populi_env_file(id: SecretId) -> bool {
    matches!(
        id,
        SecretId::VoxMeshToken
            | SecretId::VoxMeshWorkerToken
            | SecretId::VoxMeshSubmitterToken
            | SecretId::VoxMeshAdminToken
    )
}

#[must_use]
pub fn all_specs() -> &'static [SecretSpec] {
    SPECS
}

#[must_use]
pub fn managed_secret_env_names() -> Vec<&'static str> {
    let mut names = BTreeSet::new();
    for spec in SPECS {
        names.insert(spec.canonical_env);
        for alias in spec.aliases {
            names.insert(*alias);
        }
        for alias in spec.deprecated_aliases {
            names.insert(*alias);
        }
    }
    names.into_iter().collect()
}

#[must_use]
pub fn requirements_for_profile(workflow: Workflow, profile: Profile) -> WorkflowRequirements {
    requirements_for_profile_mode(workflow, profile, RequirementMode::Auto)
}

#[must_use]
pub fn requirements_for_profile_mode(
    workflow: Workflow,
    profile: Profile,
    mode: RequirementMode,
) -> WorkflowRequirements {
    let effective_mode = match mode {
        RequirementMode::Auto => match profile {
            Profile::Dev | Profile::Mobile => RequirementMode::Local,
            Profile::Ci | Profile::Prod => RequirementMode::Cloud,
        },
        RequirementMode::Local => RequirementMode::Local,
        RequirementMode::Cloud => RequirementMode::Cloud,
    };

    let mut blocking = match workflow {
        Workflow::Chat | Workflow::Mcp => match effective_mode {
            RequirementMode::Local | RequirementMode::Auto => vec![],
            RequirementMode::Cloud => vec![RequirementSet::AllOf(CHAT_CLOUD_PRIMARY)],
        },
        Workflow::Publish | Workflow::Review => {
            vec![RequirementSet::AllOf(&[SecretId::GitHubToken])]
        }
        Workflow::DbRemote => vec![RequirementSet::AllOf(&[
            SecretId::VoxDbUrl,
            SecretId::VoxDbToken,
        ])],
        Workflow::MensMesh => vec![RequirementSet::AnyOf(&[
            SecretId::VoxMeshToken,
            SecretId::VoxMeshWorkerToken,
            SecretId::VoxMeshSubmitterToken,
            SecretId::VoxMeshAdminToken,
        ])],
    };

    if matches!(profile, Profile::Ci) && matches!(workflow, Workflow::Chat | Workflow::Mcp) {
        blocking.push(RequirementSet::AllOf(&[SecretId::GitHubToken]));
    }

    let optional = match workflow {
        Workflow::Chat | Workflow::Mcp => ALL_CHAT_OPTIONALS.to_vec(),
        Workflow::Publish | Workflow::Review => vec![
            SecretId::VoxZenodoAccessToken,
            SecretId::VoxOpenReviewEmail,
            SecretId::VoxOpenReviewPassword,
            SecretId::VoxCrossrefPlusApiKey,
            SecretId::VoxArxivAssistHandoffSecret,
        ],
        Workflow::DbRemote => vec![],
        Workflow::MensMesh => vec![],
    };
    WorkflowRequirements { blocking, optional }
}

#[must_use]
pub fn requirements_for_bundle(bundle: SecretBundle) -> WorkflowRequirements {
    match bundle {
        SecretBundle::MinimalLocalDev => WorkflowRequirements {
            blocking: vec![],
            optional: ALL_CHAT_OPTIONALS.to_vec(),
        },
        SecretBundle::MinimalCloudDev => WorkflowRequirements {
            blocking: vec![RequirementSet::AllOf(CHAT_CLOUD_PRIMARY)],
            optional: ALL_CHAT_OPTIONALS.to_vec(),
        },
        SecretBundle::GpuCloud => WorkflowRequirements {
            blocking: vec![RequirementSet::AnyOf(&[
                SecretId::VoxRunpodApiKey,
                SecretId::VoxVastApiKey,
            ])],
            optional: vec![SecretId::TogetherApiKey],
        },
        SecretBundle::PublishReview => WorkflowRequirements {
            blocking: vec![RequirementSet::AllOf(&[SecretId::GitHubToken])],
            optional: vec![],
        },
        SecretBundle::MeshRoles => WorkflowRequirements {
            blocking: vec![RequirementSet::AnyOf(&[
                SecretId::VoxMeshWorkerToken,
                SecretId::VoxMeshSubmitterToken,
            ])],
            optional: vec![
                SecretId::VoxMeshToken,
                SecretId::VoxMeshAdminToken,
            ],
        },
    }
}

#[must_use]
pub fn all_bundle_doc_names() -> &'static [&'static str] {
    BUNDLE_DOC_NAMES
}

#[must_use]
pub fn capabilities_for_secret(id: SecretId) -> &'static [Capability] {
    match id {
        SecretId::OpenRouterApiKey => &[Capability::ChatCloudPrimary],
        SecretId::GeminiApiKey
        | SecretId::OpenAiApiKey
        | SecretId::AnthropicApiKey
        | SecretId::GroqApiKey
        | SecretId::CerebrasApiKey
        | SecretId::MistralApiKey
        | SecretId::DeepSeekApiKey
        | SecretId::SambaNovaApiKey
        | SecretId::CustomOpenAiApiKey
        | SecretId::HuggingFaceToken => &[Capability::ChatCloudAlt],
        SecretId::VoxRunpodApiKey | SecretId::VoxVastApiKey | SecretId::TogetherApiKey => {
            &[Capability::GpuCloud]
        }
        SecretId::GitHubToken => &[Capability::PublishReview],
        SecretId::VoxDbUrl | SecretId::VoxDbToken => &[Capability::DbRemote],
        SecretId::VoxMeshToken
        | SecretId::VoxMeshWorkerToken
        | SecretId::VoxMeshSubmitterToken
        | SecretId::VoxMeshAdminToken
        | SecretId::VoxMeshJwtHmacSecret
        | SecretId::VoxMeshWorkerResultVerifyKey => &[Capability::Mesh],
        SecretId::VoxApiKey | SecretId::VoxBearerToken => &[Capability::RuntimeIngress],
        SecretId::V0ApiKey | SecretId::OpenClawToken => &[Capability::AuxTools],
        SecretId::VoxNewsTwitterBearer
        | SecretId::VoxNewsOpenCollectiveToken
        | SecretId::VoxSocialRedditClientId
        | SecretId::VoxSocialRedditClientSecret
        | SecretId::VoxSocialRedditRefreshToken
        | SecretId::VoxSocialRedditUserAgent
        | SecretId::VoxSocialYoutubeClientId
        | SecretId::VoxSocialYoutubeClientSecret
        | SecretId::VoxSocialYoutubeRefreshToken => &[Capability::ScientiaSyndication],
        SecretId::VoxZenodoAccessToken
        | SecretId::VoxOpenReviewEmail
        | SecretId::VoxOpenReviewPassword
        | SecretId::VoxCrossrefPlusApiKey
        | SecretId::VoxArxivAssistHandoffSecret => &[Capability::ScholarlyPublication],
    }
}

#[must_use]
pub fn required_for(workflow: Workflow) -> Vec<SecretId> {
    required_for_profile(workflow, Profile::Dev)
}

#[must_use]
pub fn required_for_profile(workflow: Workflow, profile: Profile) -> Vec<SecretId> {
    let mut out = BTreeSet::new();
    for req in requirements_for_profile(workflow, profile).blocking {
        match req {
            RequirementSet::AnyOf(ids) | RequirementSet::AllOf(ids) => {
                for id in ids {
                    out.insert(*id);
                }
            }
        }
    }
    out.into_iter().collect()
}
