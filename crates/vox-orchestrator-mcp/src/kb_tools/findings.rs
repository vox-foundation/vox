//! The single capture chokepoint that turns web/research text into pending `Untrusted`
//! `kb_findings` rows (TRUST-02/04/05). Everything stored is redacted then sanitized; the
//! full text lands only as a content-addressed local snapshot under `.vox/cache/kb-findings/`
//! (evidence for the content hash; it is never read back by retrieval or sent to a model).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use vox_db::kb_text::{ContentHash, FindingOrigin, SanitizedText};
use vox_db::{FindingTier, NewKbFinding, VoxDb};
use vox_search::capture::{CapturedWebHit, WebFindingCapture};

use crate::server_state::ServerState;

pub const MAX_QUOTES: usize = 3;
pub const MAX_QUOTE_CHARS: usize = 300;
pub const MAX_SUMMARY_CHARS: usize = 1000;
pub const MAX_SNAPSHOT_BYTES: usize = 2 * 1024 * 1024;

/// One piece of text to capture.
#[derive(Debug, Clone)]
pub struct CaptureInput {
    pub origin: FindingOrigin,
    pub title: String,
    /// Snippet / summary source.
    pub text: String,
    /// Full body when available (hashed and snapshotted instead of `text`).
    pub full_text: Option<String>,
    pub source_url: Option<String>,
    /// Dedupe discriminator; falls back to `source_url`, then empty.
    pub dedupe_scope: Option<String>,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CaptureOutcome {
    /// `None` when an identical finding already existed.
    pub finding_id: Option<String>,
    pub dedupe_key: String,
    pub snapshot_relpath: Option<String>,
}

/// Repo-relative snapshot path; the name is the content hash only (never URL-derived).
pub fn snapshot_relpath(content_hash_hex: &str) -> String {
    format!(
        "{}/kb-findings/{content_hash_hex}.txt",
        vox_config::paths::REPO_CACHE_DIR
    )
}

fn truncate_chars(s: &str, max: usize) -> &str {
    s.char_indices().nth(max).map_or(s, |(i, _)| &s[..i])
}

fn truncate_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Up to [`MAX_QUOTES`] sentences of at most [`MAX_QUOTE_CHARS`] chars each. A sentence ends at
/// a newline or at `.`/`!`/`?` followed by whitespace, so `3.8` and URLs stay whole.
pub fn extract_quotes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |seg: &str| {
        let seg = seg.trim();
        if !seg.is_empty() && out.len() < MAX_QUOTES {
            out.push(truncate_chars(seg, MAX_QUOTE_CHARS).to_string());
        }
    };
    let mut start = 0;
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let end = c == '\n'
            || (matches!(c, '.' | '!' | '?') && it.peek().is_none_or(|(_, n)| n.is_whitespace()));
        if end {
            let next = i + c.len_utf8();
            push(&text[start..next]);
            start = next;
        }
    }
    push(&text[start..]);
    out
}

fn clean(raw: &str) -> SanitizedText {
    SanitizedText::new(&vox_redact::redact_owned(raw))
}

fn hash_hex(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(bytes))
}

async fn write_snapshot(repo_root: &Path, rel: &str, content: &str) -> Result<(), String> {
    let path: PathBuf = repo_root.join(rel);
    let bytes = truncate_bytes(content, MAX_SNAPSHOT_BYTES)
        .as_bytes()
        .to_vec();
    tokio::task::spawn_blocking(move || {
        if path.exists() {
            return Ok(());
        }
        vox_db::research_doc_io::atomic_write_secure(&path, &bytes)
    })
    .await
    .map_err(|e| format!("snapshot task failed: {e}"))?
    .map_err(|e| format!("snapshot write failed: {e}"))
}

/// Capture one finding: validate, hash, redact+sanitize, snapshot, insert pending `Untrusted`.
pub async fn capture_finding(
    db: &VoxDb,
    repo_root: &Path,
    input: CaptureInput,
) -> Result<CaptureOutcome, String> {
    let blank = |s: &str| s.trim().is_empty();
    let full = input.full_text.as_deref().filter(|s| !blank(s));
    if full.is_none() && blank(&input.text) {
        return Err("capture_finding: text and full_text are both empty".into());
    }
    let content = full.unwrap_or(&input.text);
    let summary_src = if blank(&input.text) {
        content
    } else {
        &input.text
    };

    let hex = hash_hex(content.as_bytes());
    let content_hash = ContentHash::from_blake3_hex(&hex)?;
    let scope = input
        .dedupe_scope
        .as_deref()
        .or(input.source_url.as_deref())
        .unwrap_or("");
    let dedupe_key = hash_hex(format!("{}\0{scope}\0{hex}", input.origin.as_db_str()).as_bytes());

    // Redact before splitting/truncating so a secret is never cut in half or split on `.`.
    let redacted = vox_redact::redact_owned(truncate_bytes(summary_src, MAX_SNAPSHOT_BYTES));
    let summary = SanitizedText::new(truncate_chars(&redacted, MAX_SUMMARY_CHARS));
    let quotes: Vec<SanitizedText> = extract_quotes(&redacted)
        .iter()
        .map(|q| SanitizedText::new(q))
        .filter(|q| !q.as_str().is_empty())
        .collect();
    let mut title = clean(&input.title);
    if title.as_str().trim().is_empty() {
        title = clean(input.source_url.as_deref().unwrap_or("untitled"));
    }

    let rel = snapshot_relpath(&hex);
    write_snapshot(repo_root, &rel, content).await?;

    let finding = NewKbFinding {
        dedupe_key: dedupe_key.clone(),
        tier: FindingTier::Untrusted,
        origin: input.origin,
        title,
        summary,
        quotes,
        source_url: input.source_url.as_deref().map(clean),
        fetched_at_ms: vox_db::now_unix_ms() as i64,
        content_hash,
        snapshot_relpath: Some(rel.clone()),
        provenance: input.provenance,
    };
    let finding_id = db
        .insert_pending_finding(&finding)
        .await
        .map_err(|e| format!("insert_pending_finding: {e}"))?;
    Ok(CaptureOutcome {
        finding_id,
        dedupe_key,
        snapshot_relpath: Some(rel),
    })
}

/// Result of filing GUI side-door content: `pending` (new) or `duplicate` (already in the inbox).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FindingFiled {
    pub finding_id: Option<String>,
    pub status: String,
}

fn filed(finding_id: Option<String>) -> FindingFiled {
    let status = if finding_id.is_some() {
        "pending"
    } else {
        "duplicate"
    };
    FindingFiled {
        finding_id,
        status: status.to_string(),
    }
}

/// File GUI-ingested text (a scraped page or pasted text) as a pending `Untrusted` finding. It
/// never touches `knowledge_nodes`, so nothing becomes retrievable before review (GROW-06).
pub async fn file_gui_finding(
    db: &VoxDb,
    repo_root: &Path,
    origin: FindingOrigin,
    title: &str,
    text: &str,
    source_url: Option<&str>,
    provenance: serde_json::Value,
) -> Result<FindingFiled, String> {
    let outcome = capture_finding(
        db,
        repo_root,
        CaptureInput {
            origin,
            title: title.to_string(),
            text: text.to_string(),
            full_text: Some(text.to_string()),
            source_url: source_url.map(str::to_string),
            dedupe_scope: None,
            provenance,
        },
    )
    .await?;
    Ok(filed(outcome.finding_id))
}

/// Result of filing a research-doc publish request: the draft waits in the inbox with the path
/// it was proposed for; nothing is written under `docs/src` (GROW-06).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PublishFiled {
    pub finding_id: Option<String>,
    pub proposed_path: String,
    pub status: String,
}

/// File an LLM-drafted research doc as a pending `Untrusted` finding. The proposed repo-relative
/// path is computed without touching disk, and the slug must be a plain file stem.
pub async fn file_publish_draft(
    _db: &VoxDb,
    _repo_root: &Path,
    _session_id: i64,
    _slug: &str,
    _content: &str,
) -> Result<PublishFiled, String> {
    Err("not implemented".into())
}

/// What one legacy-VoxKB migration pass saw and did (RET-03).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyKbReport {
    pub knowledge_bases: i64,
    pub kb_entries: i64,
    pub kb_routing_rules: i64,
    pub migrated: u64,
    pub skipped_empty: u64,
    pub already_present: u64,
}

const LEGACY_PAGE: i64 = 500;

/// Count the legacy VoxKB tables, then copy every non-empty `kb_entries` row into the inbox as
/// a pending `Untrusted` finding (idempotent per legacy id). Reads legacy tables only: nothing
/// is deleted or rewritten (D-03, RET-03). A database without the tables reports zeros.
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-voxkb-retirement" canonical="kb_findings"
pub async fn migrate_legacy_kb_entries(
    db: &VoxDb,
    repo_root: &Path,
) -> Result<LegacyKbReport, String> {
    let c = db
        .legacy_kb_counts()
        .await
        .map_err(|e| format!("legacy_kb_counts: {e}"))?;
    let mut report = LegacyKbReport {
        knowledge_bases: c.knowledge_bases,
        kb_entries: c.kb_entries,
        kb_routing_rules: c.kb_routing_rules,
        ..Default::default()
    };
    if c.kb_entries == 0 {
        return Ok(report);
    }
    let mut after: Option<String> = None;
    loop {
        let page = db
            .legacy_kb_entries_page(after.as_deref(), LEGACY_PAGE)
            .await
            .map_err(|e| format!("legacy_kb_entries_page: {e}"))?;
        let Some(last) = page.last() else { break };
        after = Some(last.id.clone());
        for e in page {
            if e.content.trim().is_empty() {
                report.skipped_empty += 1;
                continue;
            }
            let input = CaptureInput {
                origin: FindingOrigin::LegacyKbEntry,
                title: format!("Legacy KB entry ({})", e.kb_name),
                text: e.content,
                full_text: None,
                source_url: None,
                dedupe_scope: Some(e.id.clone()),
                provenance: serde_json::json!({
                    "legacy_id": e.id,
                    "kb_name": e.kb_name,
                    "source_signal": e.source_signal,
                    "source_ref": e.source_ref,
                    "routing_confidence": e.routing_confidence,
                    "accepted": e.accepted,
                    "mens_queued": e.mens_queued,
                    "created_at_ms": e.created_at_ms,
                }),
            };
            match capture_finding(db, repo_root, input).await?.finding_id {
                Some(_) => report.migrated += 1,
                None => report.already_present += 1,
            }
        }
    }
    Ok(report)
}

/// [`WebFindingCapture`] sink that routes web hits into the findings inbox.
pub struct McpFindingCapture {
    pub db: Arc<VoxDb>,
    pub repo_root: PathBuf,
}

impl WebFindingCapture for McpFindingCapture {
    fn capture(&self, hits: Vec<CapturedWebHit>) {
        let (db, root) = (self.db.clone(), self.repo_root.clone());
        tokio::spawn(async move {
            for h in hits {
                let url = h.url.clone();
                let input = CaptureInput {
                    origin: h.origin,
                    title: h.title,
                    text: h.snippet,
                    full_text: h.full_text,
                    source_url: Some(h.url),
                    dedupe_scope: None,
                    provenance: serde_json::json!({ "engine": h.engine, "query": h.query }),
                };
                if let Err(e) = capture_finding(&db, &root, input).await {
                    tracing::warn!(url = %url, error = %e, "web finding capture failed");
                }
            }
        });
    }
}

/// A capture sink for this server's DB and repo; `None` without a DB (nothing is persisted).
pub fn web_capture_for_state(state: &ServerState) -> Option<Arc<dyn WebFindingCapture>> {
    state.db.clone().map(|db| {
        Arc::new(McpFindingCapture {
            db,
            repo_root: state.repository.root.clone(),
        }) as Arc<dyn WebFindingCapture>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_split_on_sentences_and_keep_decimals() {
        let q =
            extract_quotes("Gemini 3.8 is out. It is fast!\nSee https://a.b/c.d now? Yes. Extra.");
        assert_eq!(
            q,
            vec![
                "Gemini 3.8 is out.",
                "It is fast!",
                "See https://a.b/c.d now?"
            ]
        );
        assert!(extract_quotes("").is_empty());
        assert!(extract_quotes("  \n ").is_empty());
    }

    #[test]
    fn quotes_cap_at_char_boundary() {
        let q = extract_quotes(&"é".repeat(400));
        assert_eq!(q[0].chars().count(), MAX_QUOTE_CHARS);
    }

    #[test]
    fn snapshot_relpath_is_under_cache_dir() {
        assert_eq!(snapshot_relpath("ab"), ".vox/cache/kb-findings/ab.txt");
    }

    #[tokio::test]
    async fn legacy_kb_fresh_database_reports_zeros() {
        let db = VoxDb::open_memory().await.unwrap();
        let tmp = std::env::temp_dir();
        let r = migrate_legacy_kb_entries(&db, &tmp).await.unwrap();
        assert_eq!(r, LegacyKbReport::default());
    }

    #[test]
    fn truncate_bytes_respects_char_boundary() {
        assert_eq!(truncate_bytes("€€", 4), "€");
    }
}
