//! Source trust scoring: Crossref retraction lookup + OpenAlex venue/author
//! reputation, feeding `ResearchHit.trust_score`. Both APIs are free/keyless.
//! Ranking is fail-open: any network or parse error yields a neutral trust
//! score rather than blocking the research pipeline. Promotion eligibility
//! ([`promotion_eligibility`]) is the one fail-closed consumer: a failed lookup
//! is `Unavailable`, which is ineligible.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex as StdMutex, OnceLock};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};
use vox_secrets::{SecretId, resolve_secret};

const CROSSREF_BASE: &str = "https://api.crossref.org";
const OPENALEX_BASE: &str = "https://api.openalex.org";
const DEFAULT_POLITE_MAILTO: &str = "research@vox.computer";
const MAX_TRUST_CACHE_ENTRIES: usize = 256;

/// Strict 3.5-second HTTP timeout for scholarly metadata and trust lookups.
pub(crate) const SCHOLARLY_HTTP_TIMEOUT: Duration = Duration::from_millis(3500);

/// Generic Cell Rate Algorithm (GCRA) asynchronous rate limiter.
pub(crate) struct GcraLimiter {
    emission_interval: Duration,
    burst_tolerance: Duration,
    tat: Mutex<Option<Instant>>,
}

impl GcraLimiter {
    #[must_use]
    pub(crate) fn new(emission_interval: Duration, burst_tolerance: Duration) -> Self {
        Self {
            emission_interval,
            burst_tolerance,
            tat: Mutex::new(None),
        }
    }

    #[must_use]
    pub(crate) fn from_rps(rps: f64) -> Self {
        let clamped_rps = rps.max(0.001);
        let emission_interval = Duration::from_secs_f64(1.0 / clamped_rps);
        Self::new(emission_interval, Duration::ZERO)
    }

    /// Semantic Scholar preset: `1 RPS` unauthenticated, `10 RPS` when
    /// `SecretId::VoxSemanticScholarApiKey` resolves.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn semantic_scholar() -> Self {
        Self::semantic_scholar_for_key_present(resolve_semantic_scholar_api_key().is_some())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn semantic_scholar_for_key_present(has_api_key: bool) -> Self {
        if has_api_key {
            Self::from_rps(10.0)
        } else {
            Self::from_rps(1.0)
        }
    }

    /// Crossref polite pool preset (`5 RPS`).
    #[must_use]
    pub(crate) fn crossref_polite() -> Self {
        Self::from_rps(5.0)
    }

    /// OpenAlex polite pool preset (`10 RPS`).
    #[must_use]
    pub(crate) fn openalex() -> Self {
        Self::from_rps(10.0)
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn emission_interval(&self) -> Duration {
        self.emission_interval
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn burst_tolerance(&self) -> Duration {
        self.burst_tolerance
    }

    /// Computes wait duration for an arrival at `now` and advances the theoretical
    /// arrival time (`TAT`) atomically.
    pub(crate) async fn reserve_wait_at(&self, now: Instant) -> Duration {
        let mut guard = self.tat.lock().await;
        let current_tat = guard.unwrap_or(now);
        let allow_at = current_tat.checked_sub(self.burst_tolerance).unwrap_or(now);
        let (wait, base_tat) = if now < allow_at {
            (allow_at.duration_since(now), current_tat)
        } else {
            (Duration::ZERO, now.max(current_tat))
        };
        *guard = Some(base_tat + self.emission_interval);
        wait
    }

    /// Acquires a single cell from the GCRA limiter, sleeping until permitted.
    pub(crate) async fn acquire(&self) {
        let wait = self.reserve_wait_at(Instant::now()).await;
        if !wait.is_zero() {
            sleep(wait).await;
        }
    }
}

static OPENALEX_LIMITER: OnceLock<GcraLimiter> = OnceLock::new();
static CROSSREF_LIMITER: OnceLock<GcraLimiter> = OnceLock::new();
#[allow(dead_code)]
static SEMANTIC_SCHOLAR_LIMITER: OnceLock<GcraLimiter> = OnceLock::new();

pub(crate) fn openalex_limiter() -> &'static GcraLimiter {
    OPENALEX_LIMITER.get_or_init(GcraLimiter::openalex)
}

pub(crate) fn crossref_limiter() -> &'static GcraLimiter {
    CROSSREF_LIMITER.get_or_init(GcraLimiter::crossref_polite)
}

#[allow(dead_code)]
pub(crate) fn semantic_scholar_limiter() -> &'static GcraLimiter {
    SEMANTIC_SCHOLAR_LIMITER.get_or_init(GcraLimiter::semantic_scholar)
}

pub(crate) fn resolve_crossref_mailto() -> String {
    let crossref = resolve_secret(SecretId::VoxScientiaCrossrefMailto);
    if let Some(email) = crossref.expose().map(str::trim).filter(|s| !s.is_empty()) {
        return email.to_string();
    }
    let openalex = resolve_secret(SecretId::VoxOpenAlexEmail);
    if let Some(email) = openalex.expose().map(str::trim).filter(|s| !s.is_empty()) {
        return email.to_string();
    }
    DEFAULT_POLITE_MAILTO.to_string()
}

pub(crate) fn resolve_openalex_email() -> String {
    let openalex = resolve_secret(SecretId::VoxOpenAlexEmail);
    if let Some(email) = openalex.expose().map(str::trim).filter(|s| !s.is_empty()) {
        return email.to_string();
    }
    DEFAULT_POLITE_MAILTO.to_string()
}

pub(crate) fn resolve_crossref_plus_api_key() -> Option<String> {
    let resolved = resolve_secret(SecretId::VoxCrossrefPlusApiKey);
    resolved
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[allow(dead_code)]
pub(crate) fn resolve_semantic_scholar_api_key() -> Option<String> {
    let resolved = resolve_secret(SecretId::VoxSemanticScholarApiKey);
    resolved
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Parses Crossref `/works/{doi}` JSON payload and returns `Some(true)` if any
/// `update-to` or `updated-by` notice indicates a retraction, withdrawal, or
/// expression of concern; `Some(false)` if parsed cleanly with none; `None` if
/// JSON is malformed.
#[must_use]
pub fn parse_crossref_retraction_json(text: &str) -> Option<bool> {
    #[derive(Deserialize)]
    struct CrossrefWork {
        message: CrossrefMessage,
    }
    #[derive(Deserialize)]
    struct CrossrefMessage {
        #[serde(default, rename = "updated-by")]
        updated_by: Vec<CrossrefUpdate>,
        #[serde(default, rename = "update-to")]
        update_to: Vec<CrossrefUpdate>,
    }
    #[derive(Deserialize)]
    struct CrossrefUpdate {
        #[serde(rename = "type")]
        update_type: String,
    }

    let parsed: CrossrefWork = serde_json::from_str(text).ok()?;
    let is_retracted = parsed
        .message
        .updated_by
        .iter()
        .chain(parsed.message.update_to.iter())
        .any(|u| is_retraction_or_concern_type(&u.update_type));
    Some(is_retracted)
}

#[must_use]
fn is_retraction_or_concern_type(raw_type: &str) -> bool {
    let norm = raw_type.trim().to_ascii_lowercase().replace('_', "-");
    matches!(
        norm.as_str(),
        "retraction" | "retracted" | "withdrawal" | "withdrawn" | "expression-of-concern"
    )
}

/// Computes the final trust score given an optional retraction verdict and a
/// base venue reputation multiplier. Confirmed retraction (`Some(true)`) forces
/// a hard-zero trust score (`0.0`).
#[must_use]
pub fn trust_score_for_retraction(is_retracted: Option<bool>, reputation: f64) -> f64 {
    if is_retracted == Some(true) {
        0.0
    } else {
        reputation
    }
}

pub struct TrustScorer {
    http: reqwest::Client,
    crossref_base: String,
    openalex_base: String,
    retraction_cache: StdMutex<HashMap<String, bool>>,
    venue_cache: StdMutex<HashMap<String, Option<String>>>,
}

impl TrustScorer {
    pub fn new() -> Self {
        Self {
            http: vox_http_client::client_builder()
                .timeout(SCHOLARLY_HTTP_TIMEOUT)
                .user_agent("VoxResearchBot/1.0 (+https://vox.dev/research-bot)")
                .build()
                .expect("reqwest client"),
            crossref_base: CROSSREF_BASE.to_string(),
            openalex_base: OPENALEX_BASE.to_string(),
            retraction_cache: StdMutex::new(HashMap::new()),
            venue_cache: StdMutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    fn with_base_urls(crossref_base: impl Into<String>, openalex_base: impl Into<String>) -> Self {
        Self {
            http: vox_http_client::client_builder()
                .timeout(SCHOLARLY_HTTP_TIMEOUT)
                .user_agent("VoxResearchBot/1.0 (+https://vox.dev/research-bot)")
                .build()
                .expect("reqwest client"),
            crossref_base: crossref_base.into(),
            openalex_base: openalex_base.into(),
            retraction_cache: StdMutex::new(HashMap::new()),
            venue_cache: StdMutex::new(HashMap::new()),
        }
    }

    /// Returns `Some(true)` if the DOI is confirmed retracted/withdrawn/flagged,
    /// `Some(false)` if confirmed clean, `None` if the lookup failed
    /// (caller should treat `None` as "unknown, don't penalize").
    pub async fn check_retraction(&self, doi: &str) -> Option<bool> {
        self.check_retraction_with_db(doi, None).await
    }

    /// Retraction lookup with bounded in-memory caching and optional `VoxDb` cache hook.
    pub async fn check_retraction_with_db(
        &self,
        doi: &str,
        _db: Option<&vox_db::VoxDb>,
    ) -> Option<bool> {
        let normalized_doi = doi.trim();
        if normalized_doi.is_empty() {
            return None;
        }
        let cache_key = format!("{}::{}", self.crossref_base, normalized_doi);
        if let Ok(guard) = self.retraction_cache.lock()
            && let Some(&cached) = guard.get(&cache_key)
        {
            return Some(cached);
        }

        crossref_limiter().acquire().await;
        let mailto = resolve_crossref_mailto();
        let url = format!(
            "{}/works/{}?mailto={}",
            self.crossref_base.trim_end_matches('/'),
            urlencoding::encode(normalized_doi),
            urlencoding::encode(&mailto),
        );
        let mut req = self.http.get(&url).timeout(SCHOLARLY_HTTP_TIMEOUT);
        if let Some(plus_key) = resolve_crossref_plus_api_key() {
            req = req.header("Crossref-Plus-API-Token", format!("Bearer {plus_key}"));
        }
        let resp = req.send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let text = resp.text().await.ok()?;
        let is_retracted = parse_crossref_retraction_json(&text)?;
        if let Ok(mut guard) = self.retraction_cache.lock() {
            if guard.len() >= MAX_TRUST_CACHE_ENTRIES {
                guard.clear();
            }
            guard.insert(cache_key, is_retracted);
        }
        Some(is_retracted)
    }

    /// Promotion-time lookup: the one fail-closed consumer of the retraction
    /// check. The explicit `doi` wins over one extracted from `url`; no DOI means
    /// `NotApplicable` (no network call).
    pub async fn lookup_for_promotion(&self, url: &str, doi: Option<&str>) -> TrustLookup {
        self.lookup_for_promotion_with_attestation(url, doi, None)
            .await
    }

    /// Promotion-time lookup with an optional offline human attestation fallback
    /// so standalone / air-gapped publications never deadlock when scholarly APIs
    /// are unreachable. Confirmed retractions still fail closed even when attested.
    pub async fn lookup_for_promotion_with_attestation(
        &self,
        url: &str,
        doi: Option<&str>,
        attestation: Option<&OfflineAttestation>,
    ) -> TrustLookup {
        let Some(doi) = doi.map(str::to_owned).or_else(|| extract_doi_from_url(url)) else {
            return TrustLookup::NotApplicable;
        };
        match self.check_retraction(&doi).await {
            Some(true) => TrustLookup::Retracted,
            Some(false) => TrustLookup::Clean,
            None => {
                if let Some(att) = attestation
                    && att.is_valid()
                {
                    TrustLookup::OfflineAttested {
                        signer_id: att.signer_id.trim().to_string(),
                        reason: att.reason.trim().to_string(),
                        timestamp_unix_ms: att.timestamp_unix_ms,
                    }
                } else {
                    TrustLookup::Unavailable {
                        reason: format!("Crossref lookup failed for DOI {doi}"),
                    }
                }
            }
        }
    }

    /// Returns the OpenAlex venue `source_type` string (e.g. "journal",
    /// "repository", "conference", "preprint") for a work matching `title`,
    /// or `None` on any lookup failure or when no venue type is available.
    /// This is the raw signal `reputation_multiplier` derives its score
    /// from — exposed separately so callers building `WorthinessSignalItem`s
    /// (see `vox-scientia::producers::worthiness`) can classify venue type
    /// without duplicating the OpenAlex fetch/parse.
    pub async fn venue_type(&self, title: &str) -> Option<String> {
        #[derive(Deserialize)]
        struct OpenAlexSearch {
            results: Vec<OpenAlexWork>,
        }
        #[derive(Deserialize)]
        struct OpenAlexWork {
            #[serde(default)]
            primary_location: Option<OpenAlexLocation>,
        }
        #[derive(Deserialize)]
        struct OpenAlexLocation {
            source: Option<OpenAlexSource>,
        }
        #[derive(Deserialize)]
        struct OpenAlexSource {
            #[serde(rename = "type")]
            source_type: Option<String>,
        }

        let normalized_title = title.trim();
        if normalized_title.is_empty() {
            return None;
        }
        let cache_key = format!("{}::{}", self.openalex_base, normalized_title);
        if let Ok(guard) = self.venue_cache.lock()
            && let Some(cached) = guard.get(&cache_key)
        {
            return cached.clone();
        }

        openalex_limiter().acquire().await;
        let mailto = resolve_openalex_email();
        let url = format!(
            "{}/works?search={}&per-page=1&mailto={}",
            self.openalex_base.trim_end_matches('/'),
            urlencoding::encode(normalized_title),
            urlencoding::encode(&mailto),
        );
        let resp = self
            .http
            .get(&url)
            .timeout(SCHOLARLY_HTTP_TIMEOUT)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let text = resp.text().await.ok()?;
        let parsed = serde_json::from_str::<OpenAlexSearch>(&text).ok()?;

        let venue = parsed
            .results
            .into_iter()
            .next()
            .and_then(|w| w.primary_location)
            .and_then(|l| l.source)
            .and_then(|s| s.source_type);
        if let Ok(mut guard) = self.venue_cache.lock() {
            if guard.len() >= MAX_TRUST_CACHE_ENTRIES {
                guard.clear();
            }
            guard.insert(cache_key, venue.clone());
        }
        venue
    }

    /// Returns a soft reputation multiplier in [0.5, 1.5] based on the
    /// venue type and author citation history for a work matching `title`,
    /// via OpenAlex. Returns `1.0` (neutral) on any lookup failure.
    pub async fn reputation_multiplier(&self, title: &str) -> f64 {
        match self.venue_type(title).await.as_deref() {
            Some("journal") => 1.5,
            Some("repository") | Some("conference") => 1.2,
            Some("preprint") => 1.0,
            _ => 1.0,
        }
    }
}

impl Default for TrustScorer {
    fn default() -> Self {
        Self::new()
    }
}

static SHARED_SCORER: OnceLock<TrustScorer> = OnceLock::new();

/// Returns a process-wide shared `TrustScorer`, constructed once on first
/// use. Prefer this over `TrustScorer::new()` in hot paths that score many
/// hits, so the underlying HTTP client's connection pool is actually reused.
fn shared_scorer() -> &'static TrustScorer {
    SHARED_SCORER.get_or_init(TrustScorer::new)
}

/// Computes a combined trust score for a hit: 1.0 baseline, halved on
/// confirmed retraction, scaled by venue reputation otherwise. Never fails
/// — any lookup error yields the neutral 1.0 baseline. `doi` is optional
/// since most web hits won't resolve to one.
/// Extracts a DOI from a URL if it matches a common DOI-URL shape
/// (`https://doi.org/10.XXXX/...` or `https://dx.doi.org/10.XXXX/...`).
/// Returns `None` for URLs that aren't DOI links — this is intentionally
/// narrow (exact-prefix match, not a general DOI regex) since it only
/// needs to catch the common case of a search hit whose URL IS a DOI
/// resolver link; a hit citing a DOI in its body text without a doi.org
/// URL is out of scope for this simple extractor.
pub fn extract_doi_from_url(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let doi = rest.trim_end_matches('/').trim();
            if !doi.is_empty() && doi.starts_with("10.") {
                // Re-slice from the ORIGINAL (non-lowercased) url to preserve
                // DOI casing (DOIs can be case-sensitive in some registries).
                let start = url.len() - rest.len();
                return Some(url[start..].trim_end_matches('/').trim().to_string());
            }
        }
    }
    None
}

pub async fn score_hit_trust(title: &str, doi: Option<&str>) -> f64 {
    let scorer = shared_scorer();
    if let Some(doi) = doi
        && scorer.check_retraction(doi).await == Some(true)
    {
        return 0.0; // confirmed retraction/withdrawal forces hard-zero trust score (0.0)
    }
    scorer.reputation_multiplier(title).await
}

/// Domain substrings recognized as scholarly by *both* `is_plausibly_academic`
/// (below) and `web_dispatcher::source_authority_score`'s citation-tier
/// boost — kept as one list so the two independently-maintained checks can't
/// silently drift apart on the domains they agree matter (each function also
/// has its own additional domains beyond this shared core, since gating an
/// OpenAlex lookup and boosting search-result ranking are related but not
/// identical concerns).
pub(crate) const CORE_ACADEMIC_DOMAINS: &[&str] =
    &["doi.org/", "arxiv.org/", "pubmed.ncbi.nlm.nih.gov/"];

/// Cheap domain check gating the OpenAlex title-search call in
/// `score_hit_trust_for_url` — skips the network call and title-collision
/// misclassification risk for hits that are clearly not scholarly sources.
/// Fail-open: an unrecognized domain returns `false` (skip OpenAlex), which
/// is the same neutral 1.0 result a genuine no-match would have produced
/// anyway, so this never suppresses a real signal, only wasted calls.
pub fn is_plausibly_academic(url: &str) -> bool {
    let key = url.to_ascii_lowercase();
    CORE_ACADEMIC_DOMAINS.iter().any(|d| key.contains(d))
        || key.contains(".edu/")
        || key.contains("ncbi.nlm.nih.gov/")
        || key.contains("researchgate.net/")
        || key.contains("springer.com/")
        || key.contains("sciencedirect.com/")
        || key.contains("jstor.org/")
}

/// URL-aware wrapper around `score_hit_trust` that skips the OpenAlex
/// reputation lookup entirely for non-academic domains. This is the new
/// entry point `web_gather.rs` should call; `score_hit_trust` itself is
/// left unchanged for any other caller that doesn't yet have a URL handy.
pub async fn score_hit_trust_for_url(title: &str, doi: Option<&str>, url: &str) -> f64 {
    if !is_plausibly_academic(url) {
        return 1.0;
    }
    score_hit_trust(title, doi).await
}

/// Offline human attestation record for standalone / air-gapped promotion when
/// live scholarly APIs are unreachable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineAttestation {
    pub signer_id: String,
    pub reason: String,
    pub timestamp_unix_ms: i64,
}

impl OfflineAttestation {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.signer_id.trim().is_empty()
            && !self.reason.trim().is_empty()
            && self.timestamp_unix_ms > 0
    }
}

/// Outcome of a trust lookup made for a promotion decision (TRUST-06 / VERI-03).
/// `NotApplicable` (no DOI, so no lookup possible) is distinct from
/// `Unavailable` (a lookup was possible but failed) and `OfflineAttested`
/// (live scholarly APIs were unreachable, and a human reviewer attested offline).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLookup {
    Clean,
    Retracted,
    NotApplicable,
    OfflineAttested {
        signer_id: String,
        reason: String,
        timestamp_unix_ms: i64,
    },
    Unavailable {
        reason: String,
    },
}

pub type ScholarlyTrustLookupOutcome = TrustLookup;

/// Promotion verdict derived from a [`TrustLookup`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromotionEligibility {
    Eligible,
    Ineligible { reason: String },
}

/// Fail-closed promotion verdict. Exhaustive on purpose: a new `TrustLookup`
/// variant must force a decision here.
pub fn promotion_eligibility(lookup: &TrustLookup) -> PromotionEligibility {
    match lookup {
        TrustLookup::Clean | TrustLookup::NotApplicable => PromotionEligibility::Eligible,
        TrustLookup::OfflineAttested {
            signer_id,
            reason,
            timestamp_unix_ms,
        } => {
            if !signer_id.trim().is_empty() && !reason.trim().is_empty() && *timestamp_unix_ms > 0 {
                PromotionEligibility::Eligible
            } else {
                PromotionEligibility::Ineligible {
                    reason: "offline attestation requires non-empty signer_id, reason, and positive timestamp".into(),
                }
            }
        }
        TrustLookup::Retracted => PromotionEligibility::Ineligible {
            reason: "source is retracted".into(),
        },
        TrustLookup::Unavailable { reason } => PromotionEligibility::Ineligible {
            reason: format!("trust lookup unavailable: {reason}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn check_retraction_detects_retracted_work() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "update-to": [{"type": "retraction"}] }
            })))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        assert_eq!(scorer.check_retraction("10.1234/example").await, Some(true));
    }

    #[tokio::test]
    async fn check_retraction_returns_false_for_clean_work() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "update-to": [] }
            })))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        assert_eq!(
            scorer.check_retraction("10.1234/example").await,
            Some(false)
        );
    }

    #[tokio::test]
    async fn check_retraction_returns_none_on_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        assert_eq!(scorer.check_retraction("10.1234/example").await, None);
    }

    #[tokio::test]
    async fn reputation_multiplier_favors_journal_venue() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{"primary_location": {"source": {"type": "journal"}}}]
            })))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls("http://unused.invalid", server.uri());
        assert_eq!(
            scorer.reputation_multiplier("Example Paper Title").await,
            1.5
        );
    }

    #[tokio::test]
    async fn reputation_multiplier_defaults_to_neutral_on_no_results() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": []
            })))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls("http://unused.invalid", server.uri());
        assert_eq!(scorer.reputation_multiplier("Nonexistent Title").await, 1.0);
    }

    #[test]
    fn extract_doi_from_url_matches_doi_org_link() {
        assert_eq!(
            extract_doi_from_url("https://doi.org/10.1234/example.5678"),
            Some("10.1234/example.5678".to_string())
        );
    }

    #[test]
    fn extract_doi_from_url_returns_none_for_non_doi_url() {
        assert_eq!(
            extract_doi_from_url("https://example.com/some-article"),
            None
        );
    }

    #[test]
    fn extract_doi_from_url_handles_dx_doi_org_variant() {
        assert_eq!(
            extract_doi_from_url("https://dx.doi.org/10.9999/xyz"),
            Some("10.9999/xyz".to_string())
        );
    }

    #[tokio::test]
    async fn venue_type_returns_raw_source_type_string() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{"primary_location": {"source": {"type": "journal"}}}]
            })))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls("http://unused.invalid", server.uri());
        assert_eq!(
            scorer.venue_type("Example Paper Title").await,
            Some("journal".to_string())
        );
    }

    #[test]
    fn is_plausibly_academic_gates_correctly() {
        assert!(is_plausibly_academic("https://doi.org/10.1000/xyz123"));
        assert!(is_plausibly_academic("https://arxiv.org/abs/2401.00001"));
        assert!(is_plausibly_academic(
            "https://www.ncbi.nlm.nih.gov/pmc/articles/PMC123"
        ));
        assert!(!is_plausibly_academic(
            "https://en.wikipedia.org/wiki/Research"
        ));
        assert!(!is_plausibly_academic(
            "https://www.reuters.com/world/article"
        ));
        assert!(!is_plausibly_academic("https://blog.example/post"));
    }

    #[tokio::test]
    async fn score_hit_trust_skips_openalex_for_non_academic_url() {
        let score =
            score_hit_trust_for_url("Some Blog Post Title", None, "https://blog.example/post")
                .await;
        assert_eq!(
            score, 1.0,
            "non-academic URL should short-circuit to neutral without an OpenAlex call"
        );
    }

    #[test]
    fn promotion_eligibility_fails_closed() {
        let unavailable = TrustLookup::Unavailable { reason: "x".into() };
        assert!(matches!(
            promotion_eligibility(&unavailable),
            PromotionEligibility::Ineligible { .. }
        ));
        assert!(matches!(
            promotion_eligibility(&TrustLookup::Retracted),
            PromotionEligibility::Ineligible { .. }
        ));
    }

    #[test]
    fn promotion_eligibility_open_for_clean_and_not_applicable() {
        assert_eq!(
            promotion_eligibility(&TrustLookup::Clean),
            PromotionEligibility::Eligible
        );
        assert_eq!(
            promotion_eligibility(&TrustLookup::NotApplicable),
            PromotionEligibility::Eligible
        );
    }

    #[tokio::test]
    async fn promotion_lookup_without_doi_is_not_applicable_and_makes_no_call() {
        let server = MockServer::start().await;
        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        let got = scorer
            .lookup_for_promotion("https://example.com/post", None)
            .await;
        assert_eq!(got, TrustLookup::NotApplicable);
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn promotion_lookup_http_error_is_unavailable_naming_doi() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        match scorer
            .lookup_for_promotion("https://doi.org/10.1/x", None)
            .await
        {
            TrustLookup::Unavailable { reason } => assert!(reason.contains("10.1/x"), "{reason}"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn promotion_lookup_retraction_is_retracted() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "update-to": [{"type": "retraction"}] }
            })))
            .mount(&server)
            .await;
        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        let got = scorer
            .lookup_for_promotion("https://doi.org/10.1/x", None)
            .await;
        assert_eq!(got, TrustLookup::Retracted);
    }

    #[tokio::test]
    async fn promotion_lookup_no_updates_is_clean() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "update-to": [] }
            })))
            .mount(&server)
            .await;
        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        let got = scorer
            .lookup_for_promotion("https://doi.org/10.1/x", None)
            .await;
        assert_eq!(got, TrustLookup::Clean);
    }

    #[tokio::test]
    async fn promotion_lookup_explicit_doi_wins_over_non_doi_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "update-to": [] }
            })))
            .mount(&server)
            .await;
        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        let got = scorer
            .lookup_for_promotion("https://example.com/post", Some("10.1/x"))
            .await;
        assert_eq!(got, TrustLookup::Clean);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn gcra_limiter_enforces_exact_cell_spacing_and_presets() {
        let s2_unauth = GcraLimiter::semantic_scholar_for_key_present(false);
        let s2_auth = GcraLimiter::semantic_scholar_for_key_present(true);
        let crossref = GcraLimiter::crossref_polite();
        let openalex = GcraLimiter::openalex();

        assert_eq!(s2_unauth.emission_interval(), Duration::from_millis(1000));
        assert_eq!(s2_auth.emission_interval(), Duration::from_millis(100));
        assert_eq!(crossref.emission_interval(), Duration::from_millis(200));
        assert_eq!(openalex.emission_interval(), Duration::from_millis(100));

        let limiter = GcraLimiter::new(Duration::from_millis(200), Duration::ZERO);
        let t0 = Instant::now();
        assert_eq!(limiter.reserve_wait_at(t0).await, Duration::ZERO);
        assert_eq!(
            limiter.reserve_wait_at(t0).await,
            Duration::from_millis(200)
        );
        assert_eq!(
            limiter.reserve_wait_at(t0).await,
            Duration::from_millis(400)
        );
    }

    #[test]
    fn crossref_update_to_retraction_withdrawal_and_concern_force_zero_trust() {
        let retraction_json = r#"{"message":{"update-to":[{"type":"retraction"}]}}"#;
        let withdrawal_json = r#"{"message":{"update-to":[{"type":"withdrawal"}]}}"#;
        let concern_json = r#"{"message":{"update-to":[{"type":"expression-of-concern"}]}}"#;
        let clean_json = r#"{"message":{"update-to":[{"type":"correction"}]}}"#;

        assert_eq!(parse_crossref_retraction_json(retraction_json), Some(true));
        assert_eq!(parse_crossref_retraction_json(withdrawal_json), Some(true));
        assert_eq!(parse_crossref_retraction_json(concern_json), Some(true));
        assert_eq!(parse_crossref_retraction_json(clean_json), Some(false));

        assert_eq!(
            trust_score_for_retraction(parse_crossref_retraction_json(retraction_json), 1.5),
            0.0
        );
        assert_eq!(
            trust_score_for_retraction(parse_crossref_retraction_json(withdrawal_json), 1.5),
            0.0
        );
        assert_eq!(
            trust_score_for_retraction(parse_crossref_retraction_json(concern_json), 1.2),
            0.0
        );
        assert_eq!(
            trust_score_for_retraction(parse_crossref_retraction_json(clean_json), 1.5),
            1.5
        );
    }

    #[tokio::test]
    async fn offline_attested_resolves_when_scholarly_api_unreachable() {
        // Point to an unmounted/failing local endpoint so Crossref lookup fails deterministically.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/works/.*"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let scorer = TrustScorer::with_base_urls(server.uri(), "http://unused.invalid");
        let attestation = OfflineAttestation {
            signer_id: "reviewer:alice@vox.computer".to_string(),
            reason: "Air-gapped primary PDF verified against archival hash".to_string(),
            timestamp_unix_ms: 1_763_000_000_000,
        };
        let outcome = scorer
            .lookup_for_promotion_with_attestation(
                "https://doi.org/10.1000/offline-paper",
                None,
                Some(&attestation),
            )
            .await;
        assert_eq!(
            outcome,
            TrustLookup::OfflineAttested {
                signer_id: "reviewer:alice@vox.computer".to_string(),
                reason: "Air-gapped primary PDF verified against archival hash".to_string(),
                timestamp_unix_ms: 1_763_000_000_000,
            }
        );
        assert_eq!(
            promotion_eligibility(&outcome),
            PromotionEligibility::Eligible
        );
    }
}
