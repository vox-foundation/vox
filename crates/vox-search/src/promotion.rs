//! Bi-directional KB promotion from `kb_findings` and MemPalace learnings into repo-local
//! `docs/wiki/` or global `~/.vox/library/` (`KB-03`), gated by sanitization, fail-closed
//! `promotion_eligibility`, and mechanical source/line citation verification.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use vox_db::kb_text::{FindingStatus, SanitizedText};
use vox_db::{NewKbHealProposal, VoxDb};

use crate::kb_rewrite_guard::{RewriteGuardReport, verify_page_one_rewrite};
use crate::kb_staleness::{
    MechanicalEvidenceInput, MechanicalVerificationResult, verify_doc_mechanically,
};
use crate::trust::{PromotionEligibility, TrustLookup, promotion_eligibility};

/// Destination (or source) tier for promoted Markdown knowledge artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionTargetScope {
    RepoLocalWiki,
    GlobalLibrary,
}

/// Mechanical verification result over inline file / `#Lstart-Lend` citations in a note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanicalCitationCheck {
    pub checked_links: usize,
    pub invalid_links: Vec<String>,
    pub valid: bool,
}

/// Outcome of promoting a `kb_findings` row or MemPalace learning note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionOutcome {
    pub slug: String,
    pub target_scope: PromotionTargetScope,
    pub output_path: String,
    pub checked_citations: usize,
}

fn sanitize_slug(raw: &str) -> Result<String> {
    let cleaned: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() || trimmed.contains("..") {
        bail!("invalid promotion slug `{raw}`");
    }
    Ok(trimmed)
}

fn parse_line_range(fragment: &str) -> Option<(usize, usize)> {
    let f = fragment.strip_prefix('L')?;
    if let Some((start_s, end_s)) = f.split_once("-L") {
        let start = start_s.parse::<usize>().ok()?;
        let end = end_s.parse::<usize>().ok()?;
        Some((start, end))
    } else {
        let line = f.parse::<usize>().ok()?;
        Some((line, line))
    }
}

/// Mechanically verify all repo-relative Markdown links (`[label](path)` / `#Lstart-Lend`)
/// against `repo_root`. External HTTP(S), `mailto:`, `kb://`, and in-page `#anchor` links
/// are ignored; path escapes and out-of-range line spans fail closed.
#[must_use]
pub fn verify_citations_mechanically(repo_root: &Path, markdown: &str) -> MechanicalCitationCheck {
    verify_citations_mechanically_for_doc(repo_root, None, markdown)
}

/// Mechanically verify Markdown links against `repo_root`, optionally resolving `../`
/// document-relative citations from `doc_rel_path`'s parent directory while strictly
/// enforcing canonical containment within `repo_root`.
#[must_use]
pub fn verify_citations_mechanically_for_doc(
    repo_root: &Path,
    doc_rel_path: Option<&str>,
    markdown: &str,
) -> MechanicalCitationCheck {
    let mut checked_links = 0_usize;
    let mut invalid_links = Vec::new();
    let mut cursor = markdown;
    let canon_root = repo_root.canonicalize().ok();

    while let Some(open_br) = cursor.find("](") {
        let after = &cursor[open_br + 2..];
        let Some(close_par) = after.find(')') else {
            break;
        };
        let raw_target = after[..close_par].trim();
        let target = raw_target.split_whitespace().next().unwrap_or("");
        cursor = &after[close_par + 1..];

        if target.is_empty()
            || target.starts_with('#')
            || target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
            || target.starts_with("kb://")
        {
            continue;
        }
        checked_links += 1;

        let (file_part, fragment_opt) = match target.split_once('#') {
            Some((p, frag)) => (p.trim_start_matches("./"), Some(frag)),
            None => (target.trim_start_matches("./"), None),
        };
        if file_part.is_empty() || file_part.starts_with('/') || file_part.contains('\\') {
            invalid_links.push(target.to_string());
            continue;
        }

        let full = if file_part.contains("..") {
            let Some(rel) = doc_rel_path else {
                invalid_links.push(target.to_string());
                continue;
            };
            let doc_dir = repo_root
                .join(rel)
                .parent()
                .unwrap_or(repo_root)
                .to_path_buf();
            let candidate = doc_dir.join(file_part);
            let Some(root_canon) = canon_root.as_ref() else {
                invalid_links.push(target.to_string());
                continue;
            };
            let Ok(cand_canon) = candidate.canonicalize() else {
                invalid_links.push(target.to_string());
                continue;
            };
            if !cand_canon.starts_with(root_canon) || !cand_canon.is_file() {
                invalid_links.push(target.to_string());
                continue;
            }
            cand_canon
        } else {
            let direct = repo_root.join(file_part);
            if !direct.is_file() {
                invalid_links.push(target.to_string());
                continue;
            }
            direct
        };

        if let Some(frag) = fragment_opt
            && let Some((start, end)) = parse_line_range(frag)
        {
            match vox_bounded_fs::read_utf8_path_capped(&full) {
                Ok(text) => {
                    let line_count = text.lines().count().max(1);
                    if start == 0 || start > end || end > line_count {
                        invalid_links.push(target.to_string());
                    }
                }
                Err(_) => invalid_links.push(target.to_string()),
            }
        }
    }

    let valid = invalid_links.is_empty();
    MechanicalCitationCheck {
        checked_links,
        invalid_links,
        valid,
    }
}

fn target_dir_for_scope(
    repo_root: &Path,
    global_library_dir: &Path,
    scope: PromotionTargetScope,
) -> PathBuf {
    match scope {
        PromotionTargetScope::RepoLocalWiki => repo_root.join("docs").join("wiki").join("promoted"),
        PromotionTargetScope::GlobalLibrary => global_library_dir.join("promoted"),
    }
}

fn render_promoted_markdown(
    title: &str,
    summary: &str,
    body: &str,
    citations: &[String],
    origin: &str,
) -> String {
    let clean_title = SanitizedText::new(title).as_str().replace('"', "'");
    let clean_summary = SanitizedText::new(summary).as_str().replace('"', "'");
    let clean_body = SanitizedText::new(body).as_str().to_string();

    let mut cite_section = String::new();
    if !citations.is_empty() {
        cite_section.push_str("\n\n## Citations\n\n");
        for c in citations {
            let clean_c = SanitizedText::new(c);
            cite_section.push_str(&format!("- [{}]({})\n", clean_c.as_str(), clean_c.as_str()));
        }
    }

    format!(
        "---\n\
         title: \"{clean_title}\"\n\
         description: \"{clean_summary}\"\n\
         category: \"Knowledge\"\n\
         status: \"current\"\n\
         origin: \"{origin}\"\n\
         ---\n\n\
         # {clean_title}\n\n\
         <!-- kb:summary -->\n\
         {clean_summary}\n\
         <!-- /kb:summary -->\n\n\
         {clean_body}{cite_section}\n"
    )
}

/// Promote a `kb_findings` row into repo-local `docs/wiki/promoted/` or global `~/.vox/library/promoted/`,
/// enforcing `SanitizedText`, fail-closed `promotion_eligibility`, and mechanical citation validation.
pub async fn promote_finding_to_kb(
    db: &VoxDb,
    repo_root: &Path,
    global_library_dir: &Path,
    finding_id: &str,
    target: PromotionTargetScope,
    reviewer: &str,
    trust_lookup: &TrustLookup,
    extra_citations: &[String],
) -> Result<PromotionOutcome> {
    if let PromotionEligibility::Ineligible { reason } = promotion_eligibility(trust_lookup) {
        bail!("finding `{finding_id}` blocked by promotion_eligibility: {reason}");
    }

    let row = db
        .kb_finding_by_id(finding_id)
        .await
        .context("lookup finding")?
        .with_context(|| format!("finding `{finding_id}` not found"))?;

    let mut body = row.summary.clone();
    if !row.quotes.is_empty() {
        body.push_str("\n\n### Quotes\n");
        for q in &row.quotes {
            body.push_str(&format!("\n> {}\n", SanitizedText::new(q).as_str()));
        }
    }
    let rendered = render_promoted_markdown(
        &row.title,
        &row.summary,
        &body,
        extra_citations,
        "kb_findings",
    );

    let check = verify_citations_mechanically(repo_root, &rendered);
    if !check.valid {
        bail!(
            "citation verification failed for `{finding_id}`: {:?}",
            check.invalid_links
        );
    }

    let slug = sanitize_slug(&row.title)?;
    let dest_dir = target_dir_for_scope(repo_root, global_library_dir, target);
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("create promotion dir {}", dest_dir.display()))?;
    let dest = dest_dir.join(format!("{slug}.md"));
    std::fs::write(&dest, &rendered)
        .with_context(|| format!("write promoted note {}", dest.display()))?;

    let note = dest.display().to_string();
    db.kb_finding_mark_reviewed(finding_id, FindingStatus::Promoted, reviewer, Some(&note))
        .await
        .context("mark finding promoted")?;

    Ok(PromotionOutcome {
        slug,
        target_scope: target,
        output_path: note,
        checked_citations: check.checked_links,
    })
}

/// Promote a `MemPalace` session learning note into repo-local `docs/wiki/promoted/` or
/// global `~/.vox/library/promoted/`, passing sanitization, trust, and citation gates.
pub fn promote_mempalace_learning_to_kb(
    repo_root: &Path,
    global_library_dir: &Path,
    slug: &str,
    title: &str,
    summary: &str,
    body: &str,
    citations: &[String],
    target: PromotionTargetScope,
    trust_lookup: &TrustLookup,
) -> Result<PromotionOutcome> {
    if let PromotionEligibility::Ineligible { reason } = promotion_eligibility(trust_lookup) {
        bail!("learning `{slug}` blocked by promotion_eligibility: {reason}");
    }

    let clean_slug = sanitize_slug(slug)?;
    let rendered = render_promoted_markdown(title, summary, body, citations, "mempalace");
    let check = verify_citations_mechanically(repo_root, &rendered);
    if !check.valid {
        bail!(
            "citation verification failed for learning `{clean_slug}`: {:?}",
            check.invalid_links
        );
    }

    let dest_dir = target_dir_for_scope(repo_root, global_library_dir, target);
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("create promotion dir {}", dest_dir.display()))?;
    let dest = dest_dir.join(format!("{clean_slug}.md"));
    std::fs::write(&dest, &rendered)
        .with_context(|| format!("write promoted learning {}", dest.display()))?;

    Ok(PromotionOutcome {
        slug: clean_slug,
        target_scope: target,
        output_path: dest.display().to_string(),
        checked_citations: check.checked_links,
    })
}

/// Synchronize an already-promoted note bi-directionally between repo-local `docs/wiki/promoted/`
/// and global `~/.vox/library/promoted/`, re-running sanitization and citation validation.
pub fn sync_promoted_note_bidirectional(
    repo_root: &Path,
    global_library_dir: &Path,
    slug: &str,
    direction: PromotionTargetScope,
) -> Result<PathBuf> {
    let clean_slug = sanitize_slug(slug)?;
    let (src_scope, dst_scope) = match direction {
        PromotionTargetScope::GlobalLibrary => (
            PromotionTargetScope::RepoLocalWiki,
            PromotionTargetScope::GlobalLibrary,
        ),
        PromotionTargetScope::RepoLocalWiki => (
            PromotionTargetScope::GlobalLibrary,
            PromotionTargetScope::RepoLocalWiki,
        ),
    };
    let src = target_dir_for_scope(repo_root, global_library_dir, src_scope)
        .join(format!("{clean_slug}.md"));
    let raw = vox_bounded_fs::read_utf8_path_capped(&src)
        .with_context(|| format!("read promoted source {}", src.display()))?;
    let sanitized = SanitizedText::new(&raw).as_str().to_string();
    let check = verify_citations_mechanically(repo_root, &sanitized);
    if !check.valid {
        bail!(
            "bi-directional sync blocked due to invalid citations in `{clean_slug}`: {:?}",
            check.invalid_links
        );
    }
    let dst_dir = target_dir_for_scope(repo_root, global_library_dir, dst_scope);
    std::fs::create_dir_all(&dst_dir)?;
    let dst = dst_dir.join(format!("{clean_slug}.md"));
    std::fs::write(&dst, sanitized)?;
    Ok(dst)
}

#[derive(Debug, Clone)]
pub struct StageHealProposalInput {
    pub proposal_id: String,
    pub repository_id: String,
    pub doc_path: String,
    pub summary_claim_rewrite: Option<String>,
    pub prose_note: Option<String>,
    pub evidence_citations: Vec<String>,
    pub mechanical_input: MechanicalEvidenceInput,
    pub has_mechanical_anchors: bool,
    pub llm_config: Option<vox_actor_runtime::llm::LlmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StagedHealProposalOutcome {
    pub proposal_id: String,
    pub mechanical_verification: MechanicalVerificationResult,
    pub rewrite_guard: Option<RewriteGuardReport>,
    pub used_llm: bool,
    pub model_id: Option<String>,
    pub unified_diff: String,
}

fn compact_unified_diff(original: &str, patched: &str, doc_path: &str) -> String {
    let a: Vec<&str> = original.lines().collect();
    let b: Vec<&str> = patched.lines().collect();
    let mut prefix = 0usize;
    while prefix < a.len() && prefix < b.len() && a[prefix] == b[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix + prefix < a.len()
        && suffix + prefix < b.len()
        && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let mut diff = format!(
        "--- a/{doc_path}\n+++ b/{doc_path}\n@@ -{},{} +{},{} @@\n",
        prefix + 1,
        a.len().saturating_sub(prefix + suffix),
        prefix + 1,
        b.len().saturating_sub(prefix + suffix),
    );
    for l in &a[prefix..a.len() - suffix] {
        diff.push_str(&format!("-{l}\n"));
    }
    for l in &b[prefix..b.len() - suffix] {
        diff.push_str(&format!("+{l}\n"));
    }
    diff
}

fn replace_summary_block(md: &str, new_summary: &str) -> Result<String> {
    let start_tag = "<!-- kb:summary -->";
    let end_tag = "<!-- /kb:summary -->";
    let s = md
        .find(start_tag)
        .ok_or_else(|| anyhow::anyhow!("missing `{start_tag}` in candidate doc"))?;
    let after_start = s + start_tag.len();
    let e_rel = md[after_start..]
        .find(end_tag)
        .ok_or_else(|| anyhow::anyhow!("missing `{end_tag}` in candidate doc"))?;
    let mut out = String::with_capacity(md.len() + new_summary.len());
    out.push_str(&md[..after_start]);
    out.push('\n');
    out.push_str(new_summary.trim());
    out.push('\n');
    out.push_str(&md[after_start + e_rel..]);
    Ok(out)
}

/// Detect narrative/mechanical drift, synthesize a minimal grounded candidate patch via
/// `vox_actor_runtime::llm::llm_chat` (with deterministic offline fallback), gate via
/// `verify_page_one_rewrite` + citation checks, and persist into `kb_heal_proposals` (`WIKI-04`).
pub async fn stage_verified_prose_heal_proposal(
    db: &VoxDb,
    repo_root: &Path,
    input: StageHealProposalInput,
) -> Result<StagedHealProposalOutcome> {
    let doc_rel = input.doc_path.trim().replace('\\', "/");
    if !doc_rel.starts_with("docs/") || doc_rel.contains("..") {
        bail!("refusing heal proposal for out-of-scope path `{doc_rel}`");
    }
    let full = repo_root.join(&doc_rel);
    let original_md = vox_bounded_fs::read_utf8_path_capped(&full)
        .with_context(|| format!("read target doc {}", full.display()))?;

    let mechanical_verification =
        verify_doc_mechanically(&input.mechanical_input, input.has_mechanical_anchors);

    let mut used_llm = false;
    let mut model_id = input.llm_config.as_ref().map(|c| c.model.clone());
    let mut llm_candidate_summary: Option<String> = None;

    if let Some(cfg) = input.llm_config {
        let prompt = format!(
            "Update only the factual summary claim for `{doc_rel}` while keeping every number, negation token, and backticked identifier accurate.\nCitations: {:?}\nSuggested note: {}",
            input.evidence_citations,
            input
                .summary_claim_rewrite
                .as_deref()
                .or(input.prose_note.as_deref())
                .unwrap_or("")
        );
        let msgs = vec![vox_actor_runtime::llm::LlmChatMessage {
            role: "user".to_string(),
            content: prompt,
            ..Default::default()
        }];
        let opts = vox_actor_runtime::ActivityOptions::default();
        if let vox_actor_runtime::ActivityResult::Ok(Ok(resp)) =
            vox_actor_runtime::llm::llm_chat(&opts, msgs, cfg).await
            && !resp.content.trim().is_empty()
        {
            used_llm = true;
            llm_candidate_summary = Some(resp.content.trim().to_string());
        } else {
            model_id = Some("deterministic-fallback".to_string());
        }
    }

    let mut patched_md = original_md.clone();
    let mut rewrite_guard = None;
    if let Some(new_sum) = llm_candidate_summary
        .as_deref()
        .or(input.summary_claim_rewrite.as_deref())
    {
        patched_md = replace_summary_block(&patched_md, new_sum)?;
        let allowed: BTreeSet<String> = input
            .mechanical_input
            .referenced_paths
            .iter()
            .chain(input.mechanical_input.referenced_symbols.iter())
            .chain(input.mechanical_input.resolved_graph_symbols.iter())
            .cloned()
            .collect();
        let report = verify_page_one_rewrite(&original_md, &patched_md, &allowed);
        if !report.passed {
            bail!("tier-2 kb_rewrite_guard rejected candidate: {report:?}");
        }
        rewrite_guard = Some(report);
    } else if let Some(ref note) = input.prose_note {
        if !patched_md.ends_with('\n') {
            patched_md.push('\n');
        }
        let citations_str = input.evidence_citations.join(" ");
        patched_md.push_str(&format!(
            "\n- Grounded heal note: {} ({})\n",
            note.trim(),
            citations_str
        ));
    } else {
        bail!("neither `summary_claim_rewrite` nor `prose_note` provided for prose heal proposal");
    }

    let citation_check =
        verify_citations_mechanically_for_doc(repo_root, Some(&doc_rel), &patched_md);
    if !citation_check.valid {
        bail!(
            "tier-2 heal blocked due to invalid markdown citations: {:?}",
            citation_check.invalid_links
        );
    }

    let unified_diff = compact_unified_diff(&original_md, &patched_md, &doc_rel);
    let original_hash = blake3::hash(original_md.as_bytes()).to_hex().to_string();
    let proposal_id = db
        .kb_heal_proposal_upsert(&NewKbHealProposal {
            id: input.proposal_id,
            repository_id: input.repository_id,
            doc_path: doc_rel,
            tier: 2,
            summary: input
                .prose_note
                .unwrap_or_else(|| "Verified Tier-2 semantic prose heal".to_string()),
            unified_diff: unified_diff.clone(),
            original_hash,
            patched_markdown: patched_md,
            evidence_citations: input.evidence_citations,
            gate_report_json: serde_json::json!({
                "mechanical_verification": mechanical_verification,
                "rewrite_guard": rewrite_guard,
                "citation_check": citation_check,
            }),
            model_id: model_id.clone(),
        })
        .await?;

    Ok(StagedHealProposalOutcome {
        proposal_id,
        mechanical_verification,
        rewrite_guard,
        used_llm,
        model_id,
        unified_diff,
    })
}

/// Apply an approved Tier-2 `kb_heal_proposals` diff on disk and mark status `applied` (`WIKI-04`).
pub async fn apply_staged_heal_proposal(
    db: &VoxDb,
    repo_root: &Path,
    proposal_id: &str,
    reviewed_by: &str,
    review_note: Option<&str>,
) -> Result<PathBuf> {
    let row = db
        .kb_heal_proposal_get(proposal_id)
        .await?
        .with_context(|| format!("kb_heal_proposals row `{proposal_id}` not found"))?;
    if !row.doc_path.starts_with("docs/") || row.doc_path.contains("..") {
        bail!(
            "refusing to apply out-of-scope proposal path `{}`",
            row.doc_path
        );
    }
    let target = repo_root.join(&row.doc_path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, &row.patched_markdown)?;
    db.kb_heal_proposal_update_status(proposal_id, "applied", Some(reviewed_by), review_note)
        .await?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_db::FindingTier;
    use vox_db::NewKbFinding;
    use vox_db::kb_text::{ContentHash, FindingOrigin};

    #[tokio::test]
    async fn promote_finding_and_mempalace_bidirectional_with_gates() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        let global = tmp.path().join("home/.vox/library");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/auth.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();

        let db = VoxDb::open_memory().await.unwrap();
        let fid = db
            .insert_pending_finding(&NewKbFinding {
                dedupe_key: "k-promo-1".into(),
                tier: FindingTier::Untrusted,
                origin: FindingOrigin::WebSearch,
                title: SanitizedText::new("Auth Token Hygiene"),
                summary: SanitizedText::new("Keep JWT validation fail-closed."),
                quotes: vec![SanitizedText::new("Always verify expiry.")],
                source_url: None,
                fetched_at_ms: 1,
                content_hash: ContentHash::from_blake3_hex(&"a".repeat(64)).unwrap(),
                snapshot_relpath: None,
                provenance: serde_json::json!({}),
            })
            .await
            .unwrap()
            .unwrap();

        // Retracted trust lookup must reject promotion fail-closed.
        assert!(
            promote_finding_to_kb(
                &db,
                &repo,
                &global,
                &fid,
                PromotionTargetScope::RepoLocalWiki,
                "alice",
                &TrustLookup::Retracted,
                &["src/auth.rs#L1-L2".to_string()],
            )
            .await
            .is_err()
        );

        // Out-of-bounds line citation must fail mechanical verification.
        assert!(
            promote_finding_to_kb(
                &db,
                &repo,
                &global,
                &fid,
                PromotionTargetScope::RepoLocalWiki,
                "alice",
                &TrustLookup::Clean,
                &["src/auth.rs#L1-L99".to_string()],
            )
            .await
            .is_err()
        );

        // Valid citation promotes to repo-local docs/wiki/promoted/ and marks DB promoted.
        let outcome = promote_finding_to_kb(
            &db,
            &repo,
            &global,
            &fid,
            PromotionTargetScope::RepoLocalWiki,
            "alice",
            &TrustLookup::Clean,
            &["src/auth.rs#L1-L2".to_string()],
        )
        .await
        .unwrap();
        assert_eq!(outcome.checked_citations, 1);
        assert!(Path::new(&outcome.output_path).is_file());
        let row = db.kb_finding_by_id(&fid).await.unwrap().unwrap();
        assert_eq!(row.status, FindingStatus::Promoted);

        // Bi-directional sync replicates repo note -> ~/.vox/library/promoted/.
        let synced = sync_promoted_note_bidirectional(
            &repo,
            &global,
            &outcome.slug,
            PromotionTargetScope::GlobalLibrary,
        )
        .unwrap();
        assert!(synced.is_file());

        // MemPalace learning promotes directly to global library then syncs to repo wiki.
        let mp = promote_mempalace_learning_to_kb(
            &repo,
            &global,
            "ppr-tip",
            "Use PPR",
            "Personalized PageRank finds 2-hop ADRs.",
            "Details citing [auth](src/auth.rs#L1-L2).",
            &[],
            PromotionTargetScope::GlobalLibrary,
            &TrustLookup::NotApplicable,
        )
        .unwrap();
        assert!(Path::new(&mp.output_path).is_file());
        let back_to_repo = sync_promoted_note_bidirectional(
            &repo,
            &global,
            "ppr-tip",
            PromotionTargetScope::RepoLocalWiki,
        )
        .unwrap();
        assert!(back_to_repo.starts_with(repo.join("docs/wiki/promoted")));
    }

    #[tokio::test]
    async fn stage_verified_prose_heal_proposal_enforces_rewrite_guard_and_applies() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join("docs/wiki")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/auth.rs"), "pub fn verify() {}\n").unwrap();

        let orig = "---\ntitle: \"Auth\"\n---\n# Auth\n\n<!-- kb:summary -->\nDo not allow empty 30s tokens when calling `verify`.\n<!-- /kb:summary -->\n\nBody.\n```rust\nfn x() {}\n```\n";
        std::fs::write(repo.join("docs/wiki/interfaces.md"), orig).unwrap();

        let db = VoxDb::open_memory().await.unwrap();
        let mut mech = MechanicalEvidenceInput {
            doc_path: "docs/wiki/interfaces.md".into(),
            ..Default::default()
        };
        mech.referenced_symbols.push("verify".into());
        mech.resolved_graph_symbols.insert("verify".into());

        // Reject candidate that drops "not" or "30"
        let bad = stage_verified_prose_heal_proposal(
            &db,
            &repo,
            StageHealProposalInput {
                proposal_id: "p-bad".into(),
                repository_id: "r1".into(),
                doc_path: "docs/wiki/interfaces.md".into(),
                summary_claim_rewrite: Some("Allow tokens when calling `verify`.".into()),
                prose_note: None,
                evidence_citations: vec!["[src/auth.rs:L1-L1]".into()],
                mechanical_input: mech.clone(),
                has_mechanical_anchors: true,
                llm_config: None,
            },
        )
        .await;
        assert!(bad.is_err());

        // Stage valid grounded rewrite and apply 1-click
        let good = stage_verified_prose_heal_proposal(
            &db,
            &repo,
            StageHealProposalInput {
                proposal_id: "p-good".into(),
                repository_id: "r1".into(),
                doc_path: "docs/wiki/interfaces.md".into(),
                summary_claim_rewrite: Some(
                    "Tokens must not expire before 30s and are verified by `verify` [auth](src/auth.rs#L1-L1)."
                        .into(),
                ),
                prose_note: None,
                evidence_citations: vec!["[src/auth.rs:L1-L1]".into()],
                mechanical_input: mech,
                has_mechanical_anchors: true,
                llm_config: None,
            },
        )
        .await
        .expect("stage valid");
        assert!(good.rewrite_guard.as_ref().is_some_and(|r| r.passed));

        let applied_path =
            apply_staged_heal_proposal(&db, &repo, &good.proposal_id, "operator", Some("LGTM"))
                .await
                .expect("apply");
        assert!(
            std::fs::read_to_string(applied_path)
                .unwrap()
                .contains("Tokens must not expire before 30s")
        );
        let row = db
            .kb_heal_proposal_get(&good.proposal_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "applied");

        // Doc-relative `../` links inside `docs/wiki/*.md` resolve inside repo_root, while escapes fail closed.
        let rel_ok = verify_citations_mechanically_for_doc(
            &repo,
            Some("docs/wiki/interfaces.md"),
            "See [auth](../../src/auth.rs#L1-L1).",
        );
        assert!(
            rel_ok.valid,
            "doc-relative citation should verify: {rel_ok:?}"
        );
        let escape_bad = verify_citations_mechanically_for_doc(
            &repo,
            Some("docs/wiki/interfaces.md"),
            "Escape [bad](../../../../etc/passwd).",
        );
        assert!(
            !escape_bad.valid,
            "path escape outside repo_root must fail closed"
        );
    }
}
