/// Venue-sensitive strictness (`double_blind` anonymization; `metadata_complete` errors on thin metadata).
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProfile {
    #[default]
    Default,
    DoubleBlind,
    /// Errors when structured scholarly metadata is missing or insufficient for repository metadata exports.
    MetadataComplete,
    /// arXiv-oriented packaging checks (submission bundle layout).
    ArxivAssist,
    /// Inbound scraped news: demands source URL, abstract text, title, and initial classification.
    NewsInbound,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreflightSeverity {
    Error,
    Warning,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreflightFinding {
    pub code: &'static str,
    pub severity: PreflightSeverity,
    pub message: String,
}
/// One human checkpoint surfaced outside scattered docs (live gates, legacy keys, manual venues).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ManualRequiredEntry {
    pub code: &'static str,
    pub reason: String,
    pub severity: PreflightSeverity,
    pub next_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_hint: Option<String>,
}
/// Ordered operator actions derived from preflight, gate, and configured channels.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NextActionEntry {
    pub code: &'static str,
    pub summary: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_hint: Option<String>,
}
/// Coarse automation posture for this preflight pass.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightConfidence {
    AutoSafe,
    AutoWithReview,
    ManualRequired,
}
/// Credential / venue readiness (presence-only; never exposes secret values).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DestinationReadinessEntry {
    pub destination: &'static str,
    pub ready: bool,
    pub remediation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_present: Option<bool>,
}
/// Optional gate / environment context so preflight can list live-publish blockers.
#[derive(Debug, Clone)]
pub struct PreflightAttentionInputs {
    pub gate: Option<crate::gate::PublishGateDecision>,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreflightReport {
    pub ok: bool,
    pub readiness_score: u8,
    pub findings: Vec<PreflightFinding>,
    /// Consolidated operator checklist (non-secret; actionable next steps).
    #[serde(default)]
    pub manual_required: Vec<ManualRequiredEntry>,
    #[serde(default)]
    pub next_actions: Vec<NextActionEntry>,
    pub confidence: PreflightConfidence,
    /// Destination / credential presence checks (no secret values).
    #[serde(default)]
    pub destination_readiness: Vec<DestinationReadinessEntry>,
    /// Conservative worthiness rubric output when requested (heuristic metrics; `meaningful_advance` is always false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worthiness: Option<crate::publication_worthiness::WorthinessEvaluation>,
}
