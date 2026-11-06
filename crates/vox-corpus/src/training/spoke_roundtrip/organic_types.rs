use super::*;

/// One organic search-to-compilation benchmark task across any of the 8 platform spokes (`ORG-SPOKE-01..05`, `SPOKE8-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeCase {
    /// Stable case identifier (e.g. `"organic_vox_01_actor_counter"`).
    pub case_id: String,
    /// Target spoke across all 8 platform spokes (`OrganicSpokeKind::ALL`).
    pub spoke: crate::training::contract::OrganicSpokeKind,
    /// Natural-language developer search query executed against the repository.
    pub search_query: String,
    /// Candidate workspace directories/files scanned by the organic search pass.
    pub search_scope_paths: Vec<String>,
    /// Expected workspace file path substring that must appear in top-5 search results.
    pub expected_top_path_hint: String,
    /// Grounding symbols/tokens that must be retrieved from workspace evidence.
    pub expected_grounding_tokens: Vec<String>,
    /// Underlying round-trip compilation/execution case grounded in retrieved evidence.
    pub roundtrip_case: SpokeRoundtripCase,
}

/// Detailed receipt for an organic search-to-compilation benchmark evaluation (`ORG-SPOKE-01..05`, `UNBIASED-SEARCH-03`, `RAG-SYNTH-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeReceipt {
    /// Case identifier.
    pub case_id: String,
    /// Target spoke (`OrganicSpokeKind`).
    pub spoke: crate::training::contract::OrganicSpokeKind,
    /// Search query executed.
    pub search_query: String,
    /// Top-ranked workspace file path discovered by organic search.
    pub top_retrieved_path: String,
    /// Rank (`1..=5`) at which `expected_top_path_hint` was found (`None` if missed).
    pub matched_rank: Option<usize>,
    /// Reciprocal rank (`1.0 / rank`, or `0.0` if missed).
    pub reciprocal_rank: f64,
    /// Rank (`1..=5`) under unbiased filename-stripped BM25+RRF search (`UNBIASED-SEARCH-02`).
    #[serde(default)]
    pub unbiased_matched_rank: Option<usize>,
    /// Reciprocal rank under unbiased filename-stripped BM25+RRF search (`UNBIASED-SEARCH-02`).
    #[serde(default)]
    pub unbiased_reciprocal_rank: f64,
    /// Fraction of `expected_grounding_tokens` verified in retrieved workspace evidence (`0.0..=1.0`).
    pub evidence_grounding_ratio: f64,
    /// Whether organic search succeeded (`matched_rank.is_some()` and `evidence_grounding_ratio >= 0.80`).
    pub search_recalled: bool,
    /// Whether RAG context grounding verified all required identifiers in the retrieved chunks (`RAG-SYNTH-02`).
    #[serde(default = "default_true_flag")]
    pub rag_context_verified: bool,
    /// Whether context-conditioned synthesis resisted a Rank-1 distractor chunk (`RAG-SYNTH-02`).
    #[serde(default = "default_true_flag")]
    pub distractor_resilient: bool,
    /// Whether deep code compilation / `rustc` `.rmeta` / AST / schema / disk verification succeeded.
    pub compilation_verified: bool,
    /// Compilation & verification stage summary (`lex+parse+hir+typeck+fmt+interp`, `syn+rustc_rmeta+policy`, etc.).
    pub compilation_stage_detail: String,
    /// End-to-end evaluation latency in microseconds (search + compilation + oracle execution).
    pub latency_us: u64,
    /// Underlying round-trip oracle receipt.
    pub roundtrip_receipt: SpokeRoundtripReceipt,
}

const fn default_true_flag() -> bool {
    true
}

/// Per-spoke aggregated organic search-to-compilation metrics across all 8 spokes (`ORG-SPOKE-06`, `SPOKE8-01`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSpokeScorecardEntry {
    /// Spoke kind (`OrganicSpokeKind`).
    pub spoke: crate::training::contract::OrganicSpokeKind,
    /// Number of organic search-to-compilation cases evaluated for this spoke.
    pub total_cases: usize,
    /// Search Recall@5 rate (`0.0..=1.0`).
    pub search_recall_at_5: f64,
    /// Mean Reciprocal Rank (`MRR`, `0.0..=1.0`).
    pub mean_reciprocal_rank: f64,
    /// Unbiased filename-stripped Search Recall@5 rate (`0.0..=1.0`) (`UNBIASED-SEARCH-03`).
    #[serde(default)]
    pub unbiased_search_recall_at_5: f64,
    /// Unbiased filename-stripped Mean Reciprocal Rank (`0.0..=1.0`) (`UNBIASED-SEARCH-03`).
    #[serde(default)]
    pub unbiased_mrr: f64,
    /// Mean evidence grounding token coverage (`0.0..=1.0`).
    pub mean_grounding_ratio: f64,
    /// Rank-1 distractor resilience rate (`0.0..=1.0`) (`RAG-SYNTH-02`).
    #[serde(default)]
    pub distractor_resilience_rate: f64,
    /// Syntax validity rate (`0.0..=1.0`).
    pub syntax_validity_rate: f64,
    /// Semantic / typecheck / schema validity rate (`0.0..=1.0`).
    pub semantic_validity_rate: f64,
    /// Deep compilation & runtime execution pass rate (`0.0..=1.0`).
    pub compilation_pass_rate: f64,
    /// End-to-end organic search -> compilation -> execution `pass@1` (`0.0..=1.0`).
    pub end_to_end_pass_at_1: f64,
    /// Mean end-to-end latency in microseconds.
    pub mean_latency_us: u64,
}

/// Unified 8-spoke organic search-to-compilation scorecard (`contracts/reports/mens-organic-search-spoke-scorecard.v1.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeScorecardV1 {
    /// Schema discriminator (`"mens_organic_search_spoke_scorecard_v1"`).
    pub schema_version: String,
    /// Total organic search-to-compilation cases (`24`, `3` per spoke across all 8 spokes).
    pub total_cases: usize,
    /// Overall Search Recall@5 across all cases.
    pub overall_search_recall_at_5: f64,
    /// Overall Mean Reciprocal Rank (`MRR`).
    pub overall_mrr: f64,
    /// Overall unbiased filename-stripped Search Recall@5 (`UNBIASED-SEARCH-03`).
    #[serde(default)]
    pub overall_unbiased_search_recall_at_5: f64,
    /// Overall unbiased filename-stripped Mean Reciprocal Rank (`UNBIASED-SEARCH-03`).
    #[serde(default)]
    pub overall_unbiased_mrr: f64,
    /// Overall evidence grounding ratio.
    pub overall_grounding_ratio: f64,
    /// Overall Rank-1 distractor resilience rate (`RAG-SYNTH-02`).
    #[serde(default)]
    pub overall_distractor_resilience_rate: f64,
    /// Overall deep compilation pass rate.
    pub overall_compilation_pass_rate: f64,
    /// Overall end-to-end `pass@1` (search + grounding + syntax + semantic + compilation + execution).
    pub overall_end_to_end_pass_at_1: f64,
    /// Per-spoke breakdown in canonical `OrganicSpokeKind::ALL` order.
    pub spokes: Vec<OrganicSpokeScorecardEntry>,
    /// Individual organic search-to-compilation receipts.
    pub receipts: Vec<OrganicSearchSpokeReceipt>,
}

impl OrganicSearchSpokeScorecardV1 {
    /// Schema discriminator for `contracts/reports/mens-organic-search-spoke-scorecard.v1.json`.
    pub const SCHEMA_VERSION: &'static str = "mens_organic_search_spoke_scorecard_v1";

    /// Project deterministic contract summary JSON (excluding machine-specific `latency_us`).
    #[must_use]
    pub fn to_contract_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": self.schema_version,
            "total_cases": self.total_cases,
            "overall_search_recall_at_5": (self.overall_search_recall_at_5 * 10000.0).round() / 10000.0,
            "overall_mrr": (self.overall_mrr * 10000.0).round() / 10000.0,
            "overall_unbiased_search_recall_at_5": (self.overall_unbiased_search_recall_at_5 * 10000.0).round() / 10000.0,
            "overall_unbiased_mrr": (self.overall_unbiased_mrr * 10000.0).round() / 10000.0,
            "overall_grounding_ratio": (self.overall_grounding_ratio * 10000.0).round() / 10000.0,
            "overall_distractor_resilience_rate": (self.overall_distractor_resilience_rate * 10000.0).round() / 10000.0,
            "overall_compilation_pass_rate": (self.overall_compilation_pass_rate * 10000.0).round() / 10000.0,
            "overall_end_to_end_pass_at_1": (self.overall_end_to_end_pass_at_1 * 10000.0).round() / 10000.0,
            "spokes": self.spokes.iter().map(|s| serde_json::json!({
                "spoke": s.spoke.as_str(),
                "total_cases": s.total_cases,
                "search_recall_at_5": (s.search_recall_at_5 * 10000.0).round() / 10000.0,
                "mean_reciprocal_rank": (s.mean_reciprocal_rank * 10000.0).round() / 10000.0,
                "unbiased_search_recall_at_5": (s.unbiased_search_recall_at_5 * 10000.0).round() / 10000.0,
                "unbiased_mrr": (s.unbiased_mrr * 10000.0).round() / 10000.0,
                "mean_grounding_ratio": (s.mean_grounding_ratio * 10000.0).round() / 10000.0,
                "distractor_resilience_rate": (s.distractor_resilience_rate * 10000.0).round() / 10000.0,
                "syntax_validity_rate": (s.syntax_validity_rate * 10000.0).round() / 10000.0,
                "semantic_validity_rate": (s.semantic_validity_rate * 10000.0).round() / 10000.0,
                "compilation_pass_rate": (s.compilation_pass_rate * 10000.0).round() / 10000.0,
                "end_to_end_pass_at_1": (s.end_to_end_pass_at_1 * 10000.0).round() / 10000.0,
            })).collect::<Vec<_>>(),
            "cases": self.receipts.iter().map(|r| serde_json::json!({
                "case_id": r.case_id,
                "spoke": r.spoke.as_str(),
                "search_query": r.search_query,
                "top_retrieved_path": r.top_retrieved_path,
                "matched_rank": r.matched_rank,
                "reciprocal_rank": (r.reciprocal_rank * 10000.0).round() / 10000.0,
                "unbiased_matched_rank": r.unbiased_matched_rank,
                "unbiased_reciprocal_rank": (r.unbiased_reciprocal_rank * 10000.0).round() / 10000.0,
                "evidence_grounding_ratio": (r.evidence_grounding_ratio * 10000.0).round() / 10000.0,
                "search_recalled": r.search_recalled,
                "rag_context_verified": r.rag_context_verified,
                "distractor_resilient": r.distractor_resilient,
                "compilation_verified": r.compilation_verified,
                "compilation_stage_detail": r.compilation_stage_detail,
                "pass_at_1": r.roundtrip_receipt.pass_at_1 && r.search_recalled && r.compilation_verified && r.rag_context_verified && r.distractor_resilient,
            })).collect::<Vec<_>>(),
        })
    }

    /// Render a human-readable Markdown report of the 8-spoke organic search-to-compilation benchmark.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# Mens 8-Spoke Organic Search-to-Compilation Scorecard (`{}`)\n",
            self.schema_version
        );
        let _ = writeln!(
            out,
            "- **Total Organic Tasks**: {} (across {} spokes)",
            self.total_cases,
            self.spokes.len()
        );
        let _ = writeln!(
            out,
            "- **Overall Search Recall@5**: {:.1}% | **MRR**: {:.3} | **Unbiased Recall@5**: {:.1}% | **Unbiased MRR**: {:.3}",
            self.overall_search_recall_at_5 * 100.0,
            self.overall_mrr,
            self.overall_unbiased_search_recall_at_5 * 100.0,
            self.overall_unbiased_mrr,
        );
        let _ = writeln!(
            out,
            "- **Evidence Grounding**: {:.1}% | **Distractor Resilience**: {:.1}% | **Compilation Pass Rate**: {:.1}% | **End-to-End `pass@1`**: {:.1}%\n",
            self.overall_grounding_ratio * 100.0,
            self.overall_distractor_resilience_rate * 100.0,
            self.overall_compilation_pass_rate * 100.0,
            self.overall_end_to_end_pass_at_1 * 100.0,
        );
        let _ = writeln!(
            out,
            "| Spoke | Tasks | Recall@5 | MRR | Unbiased R@5 | Unbiased MRR | Grounding | Distractor Res | Syntax | Semantic | Compilation | E2E `pass@1` | Mean Latency (µs) |"
        );
        let _ = writeln!(
            out,
            "| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
        );
        for s in &self.spokes {
            let _ = writeln!(
                out,
                "| `{}` | {} | {:.1}% | {:.3} | {:.1}% | {:.3} | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
                s.spoke.as_str(),
                s.total_cases,
                s.search_recall_at_5 * 100.0,
                s.mean_reciprocal_rank,
                s.unbiased_search_recall_at_5 * 100.0,
                s.unbiased_mrr,
                s.mean_grounding_ratio * 100.0,
                s.distractor_resilience_rate * 100.0,
                s.syntax_validity_rate * 100.0,
                s.semantic_validity_rate * 100.0,
                s.compilation_pass_rate * 100.0,
                s.end_to_end_pass_at_1 * 100.0,
                s.mean_latency_us,
            );
        }
        out
    }
}

pub(super) fn workspace_root_dir() -> std::path::PathBuf {
    crate::training::contract::find_workspace_root().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    })
}

/// Strip filename and relative-path hints (`.rs`, `.vox`, `.json`, `.md`, `.toml`, `.yaml`, `/`)
/// from a developer query so retrieval can be evaluated purely on semantic & symbol tokens (`UNBIASED-SEARCH-02`).
#[must_use]
pub fn strip_filename_hints(query: &str) -> String {
    const FILE_EXTS: [&str; 10] = [
        ".rs", ".vox", ".json", ".md", ".mdx", ".toml", ".yaml", ".yml", ".ts", ".tsx",
    ];
    query
        .split_whitespace()
        .filter(|tok| {
            let lower = tok.to_ascii_lowercase();
            !lower.contains('/') && !FILE_EXTS.iter().any(|ext| lower.ends_with(ext))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_candidate_files_recursive(
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<std::path::PathBuf> =
        entries.filter_map(|e| e.ok().map(|d| d.path())).collect();
    paths.sort();
    for child in paths {
        if child.is_file() {
            out.push(child);
        } else if child.is_dir() {
            let name = child
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !matches!(
                name.as_str(),
                "target" | "node_modules" | ".git" | ".planning" | "dist" | "artifacts"
            ) {
                collect_candidate_files_recursive(&child, out, depth + 1);
            }
        }
    }
}

/// Rank candidate workspace files using recursive traversal + BM25 (`k1 = 1.2`, `b = 0.75`) +
/// symbol/identifier density fused via Reciprocal Rank Fusion (`RRF`, `k = 60.0`) (`UNBIASED-SEARCH-01`).
pub(super) fn discover_workspace_evidence_with_mode(
    repo_root: &std::path::Path,
    query: &str,
    scope_paths: &[String],
    limit: usize,
    allow_path_boost: bool,
) -> Vec<(String, f64, String)> {
    let query_tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '/')
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_ascii_lowercase())
        .collect();
    if query_tokens.is_empty() {
        return Vec::new();
    }

    let mut candidate_files: Vec<std::path::PathBuf> = Vec::new();
    for rel in scope_paths {
        let p = repo_root.join(rel);
        if p.is_file() {
            candidate_files.push(p);
        } else if p.is_dir() {
            collect_candidate_files_recursive(&p, &mut candidate_files, 0);
        }
    }
    candidate_files.sort();
    candidate_files.dedup();

    struct DocEntry {
        rel: String,
        rel_lower: String,
        content: String,
        content_lower: String,
        doc_len: f64,
    }

    let mut docs: Vec<DocEntry> = Vec::with_capacity(candidate_files.len());
    for path in candidate_files {
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let mut content = vox_bounded_fs::read_utf8_path_capped(&path).unwrap_or_default();
        if let Some(stem_dir) = path.parent().and_then(|parent| {
            let stem = path.file_stem()?.to_str()?;
            let sub = parent.join(stem);
            sub.is_dir().then_some(sub)
        }) {
            let mut sub_files = Vec::new();
            collect_candidate_files_recursive(&stem_dir, &mut sub_files, 0);
            sub_files.sort();
            for sf in sub_files {
                if let Ok(sc) = vox_bounded_fs::read_utf8_path_capped(&sf) {
                    content.push('\n');
                    content.push_str(&sc);
                }
            }
        }
        if content.is_empty() {
            continue;
        }
        let doc_len = content.split_whitespace().count().max(1) as f64;
        docs.push(DocEntry {
            rel_lower: rel.to_ascii_lowercase(),
            rel,
            content_lower: content.to_ascii_lowercase(),
            content,
            doc_len,
        });
    }
    if docs.is_empty() {
        return Vec::new();
    }

    let n_docs = docs.len() as f64;
    let avgdl = docs.iter().map(|d| d.doc_len).sum::<f64>() / n_docs;
    let k1 = 1.2_f64;
    let b = 0.75_f64;

    // Document frequencies per query token
    let mut dfs: Vec<f64> = Vec::with_capacity(query_tokens.len());
    for tok in &query_tokens {
        let df = docs
            .iter()
            .filter(|d| {
                d.content_lower.contains(tok) || (allow_path_boost && d.rel_lower.contains(tok))
            })
            .count() as f64;
        dfs.push(df);
    }

    let mut bm25_scores: Vec<(usize, f64)> = Vec::with_capacity(docs.len());
    let mut symbol_scores: Vec<(usize, f64)> = Vec::with_capacity(docs.len());

    for (idx, d) in docs.iter().enumerate() {
        let mut bm25 = 0.0_f64;
        let mut sym = 0.0_f64;
        for (t_idx, tok) in query_tokens.iter().enumerate() {
            let tf_count = d.content_lower.matches(tok.as_str()).count() as f64;
            let path_hit = if allow_path_boost && d.rel_lower.contains(tok.as_str()) {
                2.0
            } else {
                0.0
            };
            let tf = tf_count + path_hit;
            if tf > 0.0 {
                let df = dfs[t_idx];
                let idf = (1.0 + (n_docs - df + 0.5) / (df + 0.5)).ln();
                let norm_tf = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (d.doc_len / avgdl)));
                bm25 += idf * norm_tf;
            }
            if tf_count > 0.0 {
                // Distinct symbol/identifier presence + declaration density bonus (length-normalized)
                let decl_bonus = if d.content_lower.contains(&format!("fn {tok}"))
                    || d.content_lower.contains(&format!("struct {tok}"))
                    || d.content_lower.contains(&format!("enum {tok}"))
                    || d.content_lower.contains(&format!("actor {tok}"))
                    || d.content_lower.contains(&format!("workflow {tok}"))
                    || d.content_lower.contains(&format!("activity {tok}"))
                {
                    2.5
                } else {
                    0.0
                };
                let len_norm = 1.0 + 0.30 * (d.doc_len / avgdl);
                sym += 2.0 + decl_bonus + (tf_count.ln_1p() / len_norm);
            }
            if allow_path_boost && d.rel_lower.contains(tok.as_str()) {
                sym += 3.5;
            }
        }
        if bm25 > 0.0 || sym > 0.0 {
            bm25_scores.push((idx, bm25));
            symbol_scores.push((idx, sym));
        }
    }

    bm25_scores.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| docs[a.0].rel.cmp(&docs[b.0].rel))
    });
    symbol_scores.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| docs[a.0].rel.cmp(&docs[b.0].rel))
    });

    let mut rrf_map: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    let rrf_k = 60.0_f64;
    for (rank, (doc_idx, score)) in bm25_scores.iter().enumerate() {
        *rrf_map.entry(*doc_idx).or_insert(0.0) +=
            (1.0 / (rrf_k + (rank as f64) + 1.0)) + (*score * 1e-4);
    }
    for (rank, (doc_idx, score)) in symbol_scores.iter().enumerate() {
        *rrf_map.entry(*doc_idx).or_insert(0.0) +=
            (1.0 / (rrf_k + (rank as f64) + 1.0)) + (*score * 1e-4);
    }

    let mut fused: Vec<(String, f64, String)> = rrf_map
        .into_iter()
        .map(|(doc_idx, rrf_score)| {
            let d = &docs[doc_idx];
            (d.rel.clone(), rrf_score, d.content.clone())
        })
        .collect();
    fused.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    fused.truncate(limit);
    fused
}

/// Rank candidate workspace files by recursive BM25 + symbol density + RRF fusion.
pub(super) fn discover_workspace_evidence(
    repo_root: &std::path::Path,
    query: &str,
    scope_paths: &[String],
    limit: usize,
) -> Vec<(String, f64, String)> {
    discover_workspace_evidence_with_mode(repo_root, query, scope_paths, limit, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_scorecard() -> OrganicSearchSpokeScorecardV1 {
        OrganicSearchSpokeScorecardV1 {
            schema_version: OrganicSearchSpokeScorecardV1::SCHEMA_VERSION.to_string(),
            total_cases: 0,
            overall_search_recall_at_5: 0.123456,
            overall_mrr: 0.5,
            overall_unbiased_search_recall_at_5: 0.0,
            overall_unbiased_mrr: 0.0,
            overall_grounding_ratio: 1.0,
            overall_distractor_resilience_rate: 1.0,
            overall_compilation_pass_rate: 1.0,
            overall_end_to_end_pass_at_1: 1.0,
            spokes: Vec::new(),
            receipts: Vec::new(),
        }
    }

    #[test]
    fn strip_filename_hints_drops_paths_and_file_extensions_only() {
        assert_eq!(
            strip_filename_hints(
                "actor Counter on increment counter_actor.vox crates/x/pipeline.rs"
            ),
            "actor Counter on increment"
        );
        assert_eq!(strip_filename_hints("README.MD notes"), "notes");
        assert_eq!(strip_filename_hints(""), "");
    }

    #[test]
    fn scorecard_contract_json_rounds_to_four_decimals() {
        let v = empty_scorecard().to_contract_json();
        assert_eq!(
            v["schema_version"],
            "mens_organic_search_spoke_scorecard_v1"
        );
        assert_eq!(v["overall_search_recall_at_5"], 0.1235);
        assert_eq!(v["spokes"].as_array().map(Vec::len), Some(0));
        assert_eq!(v["cases"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn scorecard_markdown_reports_totals_and_table_header() {
        let md = empty_scorecard().to_markdown();
        assert!(md.contains("mens_organic_search_spoke_scorecard_v1"));
        assert!(md.contains("**Total Organic Tasks**: 0 (across 0 spokes)"));
        assert!(md.contains("| Spoke | Tasks |"));
    }
}
