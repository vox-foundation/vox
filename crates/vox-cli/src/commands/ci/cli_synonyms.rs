//! Build/CI-time semantic CLI synonym compiler & antonym firewall (`CLI-TOL-04`).
//!
//! Reads `contracts/operations/catalog.v1.yaml` plus curated slot synonyms & foreign idioms,
//! verifies antonym exclusion (`ANTONYM_BANLIST`), zero canonical collisions, and local
//! embedding cosine margin separation (`MIN_COSINE_MARGIN`), and validates against
//! `contracts/cli/semantic-aliases.v1.schema.json`.
//!
//! Optional LLM mining (`VOX_CLI_SYNONYMS_LLM_MINE=1`) routes exclusively through
//! `vox_actor_runtime::llm::llm_chat` (never direct vendor endpoints).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat, local_embed_text};
use vox_bounded_fs::read_utf8_path_capped;

use super::operations_catalog::{OPERATIONS_CATALOG_REL, OperationsCatalog};

pub const SEMANTIC_ALIASES_REL: &str = "contracts/cli/semantic-aliases.v1.json";
pub const SEMANTIC_ALIASES_SCHEMA_REL: &str = "contracts/cli/semantic-aliases.v1.schema.json";
pub const EMBEDDING_MODEL_ID: &str = "gemini-embedding-2";
pub const MIN_COSINE_SIMILARITY: f32 = 0.72;
pub const MIN_COSINE_MARGIN: f32 = 0.08;

/// Explicit destructive/opposite verb antonym banlist (both directions forbidden).
pub const ANTONYM_BANLIST: &[(&str, &str)] = &[
    ("add", "remove"),
    ("add", "delete"),
    ("add", "rm"),
    ("add", "del"),
    ("add", "uninstall"),
    ("start", "stop"),
    ("lock", "unlock"),
    ("set", "get"),
    ("set", "unset"),
    ("import", "export"),
    ("enable", "disable"),
    ("create", "destroy"),
    ("install", "uninstall"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginStatus {
    AutoAlias,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasSlot {
    Verb,
    Noun,
    Idiom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasSource {
    Curated,
    Catalog,
    LlmMined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignIdiomRule {
    pub trigger: Vec<String>,
    pub canonical_path: Vec<String>,
    #[serde(default)]
    pub insert_args: Vec<String>,
    pub cosine_similarity: f32,
    pub cosine_margin: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasRule {
    pub alias: String,
    pub slot: AliasSlot,
    pub canonical_target: Vec<String>,
    pub source: AliasSource,
    pub status: MarginStatus,
    pub cosine_similarity: f32,
    pub cosine_margin: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAliasesLockfile {
    pub schema_version: u32,
    pub generated_from: String,
    pub min_cosine_similarity: f32,
    pub min_cosine_margin: f32,
    pub antonym_banlist: Vec<[String; 2]>,
    pub verb_aliases: BTreeMap<String, String>,
    pub noun_aliases: BTreeMap<String, String>,
    pub foreign_idioms: Vec<ForeignIdiomRule>,
    pub rules: Vec<AliasRule>,
}

fn curated_verb_aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("diagnose", "status"),
        ("ls", "list"),
        ("show", "get"),
        ("rm", "remove"),
        ("del", "remove"),
        ("uninstall", "remove"),
        ("lint", "check"),
    ]
}

fn curated_noun_aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("clavis", "secrets"),
        ("cred", "secrets"),
        ("creds", "secrets"),
        ("key", "secrets"),
        ("keys", "secrets"),
        ("secret", "secrets"),
        ("skills", "skill"),
    ]
}

struct SeedIdiom {
    trigger: &'static [&'static str],
    canonical_path: &'static [&'static str],
    insert_args: &'static [&'static str],
    description: &'static str,
}

fn curated_foreign_idioms() -> &'static [SeedIdiom] {
    &[
        SeedIdiom {
            trigger: &["clippy"],
            canonical_path: &["audit", "code"],
            insert_args: &[],
            description: "run static code audit and lint rules",
        },
        SeedIdiom {
            trigger: &["diagnose", "clavis"],
            canonical_path: &["secrets", "status"],
            insert_args: &[],
            description: "diagnose Clavis secret workflow readiness",
        },
        SeedIdiom {
            trigger: &["secrets", "doctor"],
            canonical_path: &["secrets", "status"],
            insert_args: &[],
            description: "check secret workflow readiness",
        },
        SeedIdiom {
            trigger: &["list", "secrets"],
            canonical_path: &["secrets", "status"],
            insert_args: &[],
            description: "list configured secret readiness statuses",
        },
    ]
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-8 || norm_b < 1e-8 {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }
}

pub fn is_antonym_pair(a: &str, b: &str) -> bool {
    let a_lc = a.to_ascii_lowercase();
    let b_lc = b.to_ascii_lowercase();
    ANTONYM_BANLIST.iter().any(|(x, y)| {
        let x_lc = x.to_ascii_lowercase();
        let y_lc = y.to_ascii_lowercase();
        (a_lc == x_lc && b_lc == y_lc) || (a_lc == y_lc && b_lc == x_lc)
    })
}

/// Evaluate embedding margin between `target_vec` and the closest neighbor `other_vecs`.
pub fn verify_embedding_margin(
    alias_vec: &[f32],
    target_vec: &[f32],
    other_vecs: &[&[f32]],
) -> (f32, f32, MarginStatus) {
    let target_sim = cosine_similarity(alias_vec, target_vec);
    let runner_up = other_vecs
        .iter()
        .map(|v| cosine_similarity(alias_vec, v))
        .fold(-1.0_f32, f32::max);
    let margin = target_sim - runner_up;
    let status = if margin + 1e-6 >= MIN_COSINE_MARGIN {
        MarginStatus::AutoAlias
    } else {
        MarginStatus::Ambiguous
    };
    (target_sim, margin, status)
}

fn round4(v: f32) -> f32 {
    (v * 10000.0).round() / 10000.0
}

fn active_cli_paths(cat: &OperationsCatalog) -> BTreeSet<Vec<String>> {
    cat.operations
        .iter()
        .filter_map(|op| op.cli.as_ref())
        .filter(|c| c.status != "retired")
        .map(|c| c.path.iter().map(|s| s.to_ascii_lowercase()).collect())
        .collect()
}

pub fn validate_no_antonyms_or_collisions(
    lockfile: &SemanticAliasesLockfile,
    active_paths: &BTreeSet<Vec<String>>,
) -> Result<()> {
    let top_levels: BTreeSet<String> = active_paths
        .iter()
        .filter_map(|p| p.first().cloned())
        .collect();
    let all_verbs_and_nouns: BTreeSet<String> = active_paths.iter().flatten().cloned().collect();

    for (alias, canonical) in lockfile
        .verb_aliases
        .iter()
        .chain(lockfile.noun_aliases.iter())
    {
        let alias_lc = alias.to_ascii_lowercase();
        let canon_lc = canonical.to_ascii_lowercase();
        if is_antonym_pair(&alias_lc, &canon_lc) {
            bail!(
                "antonym firewall violation: alias `{alias}` must never map to opposite `{canonical}`"
            );
        }
        if alias_lc == canon_lc {
            bail!("redundant identity alias `{alias}` -> `{canonical}`");
        }
        if lockfile.verb_aliases.contains_key(&alias_lc) && all_verbs_and_nouns.contains(&alias_lc)
        {
            bail!(
                "canonical collision: verb alias `{alias}` collides with an existing active CLI segment"
            );
        }
        if lockfile.noun_aliases.contains_key(&alias_lc) && top_levels.contains(&alias_lc) {
            bail!(
                "canonical collision: noun alias `{alias}` collides with an active top-level command"
            );
        }
    }

    let mut seen_triggers: BTreeSet<Vec<String>> = BTreeSet::new();
    for idiom in &lockfile.foreign_idioms {
        let trig: Vec<String> = idiom
            .trigger
            .iter()
            .map(|t| t.to_ascii_lowercase())
            .collect();
        if trig.is_empty() {
            bail!("foreign idiom trigger must not be empty");
        }
        if active_paths.contains(&trig) {
            bail!(
                "canonical collision: foreign idiom `{}` matches an active canonical CLI path",
                trig.join(" ")
            );
        }
        for seg in &trig {
            for target_seg in &idiom.canonical_path {
                if is_antonym_pair(seg, target_seg) {
                    bail!(
                        "antonym firewall violation: foreign idiom `{}` contains antonym `{seg}` <-> `{target_seg}`",
                        trig.join(" ")
                    );
                }
            }
        }
        if !seen_triggers.insert(trig.clone()) {
            bail!("duplicate foreign idiom trigger `{}`", trig.join(" "));
        }
    }

    for rule in &lockfile.rules {
        for target_seg in &rule.canonical_target {
            if is_antonym_pair(&rule.alias, target_seg) {
                bail!(
                    "antonym firewall violation on rule `{}` -> `{}`",
                    rule.alias,
                    rule.canonical_target.join(" ")
                );
            }
        }
    }
    Ok(())
}

/// Optional offline-safe helper to mine additional synonym candidates through the
/// `vox_actor_runtime::llm::llm_chat` boundary (`gemini-3.5-flash-lite`) when opted in.
pub async fn mine_llm_synonym_candidates(
    catalog: &OperationsCatalog,
    limit: usize,
) -> Result<Vec<(Vec<String>, String)>> {
    let summary_lines: Vec<String> = catalog
        .operations
        .iter()
        .filter_map(|op| {
            let cli = op.cli.as_ref()?;
            if cli.status != "active" {
                return None;
            }
            Some(format!("{} — {}", cli.path.join(" "), op.description))
        })
        .take(limit.max(1))
        .collect();
    let prompt = format!(
        "Propose safe verb/noun CLI synonyms as JSON array of {{\"path\": [...], \"alias\": \"...\"}} pairs:\n{}",
        summary_lines.join("\n")
    );
    let cfg = LlmConfig::openrouter("gemini-3.5-flash-lite");
    let opts = vox_actor_runtime::ActivityOptions::default();
    let messages = vec![LlmChatMessage {
        role: "user".to_string(),
        content: prompt,
        ..Default::default()
    }];
    let response = match llm_chat(&opts, messages, cfg).await {
        vox_actor_runtime::ActivityResult::Ok(Ok(resp)) => resp,
        vox_actor_runtime::ActivityResult::Ok(Err(e)) => {
            anyhow::bail!("llm_chat failed: {e}");
        }
        vox_actor_runtime::ActivityResult::Failed(e) => {
            anyhow::bail!("llm_chat activity failed: {e}");
        }
        vox_actor_runtime::ActivityResult::Cancelled => {
            anyhow::bail!("llm_chat activity cancelled");
        }
    };
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(&response.content).unwrap_or_default();
    let mut out = Vec::new();
    for item in parsed {
        let Some(alias) = item.get("alias").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(path_arr) = item.get("path").and_then(|v| v.as_array()) else {
            continue;
        };
        let path: Vec<String> = path_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_ascii_lowercase()))
            .collect();
        if !path.is_empty() && !alias.trim().is_empty() {
            out.push((path, alias.trim().to_ascii_lowercase()));
        }
    }
    Ok(out)
}

pub fn build_lockfile(catalog: &OperationsCatalog) -> Result<SemanticAliasesLockfile> {
    let active_paths = active_cli_paths(catalog);
    let top_levels: BTreeSet<String> = active_paths
        .iter()
        .filter_map(|p| p.first().cloned())
        .collect();
    let active_leaf_segments: BTreeSet<String> = active_paths.iter().flatten().cloned().collect();

    let mut verb_aliases = BTreeMap::new();
    let mut noun_aliases = BTreeMap::new();
    let mut foreign_idioms: Vec<ForeignIdiomRule> = Vec::new();
    let mut rules: Vec<AliasRule> = Vec::new();

    for (alias, canonical) in curated_verb_aliases() {
        if active_leaf_segments.contains(canonical) && !active_leaf_segments.contains(alias) {
            verb_aliases.insert(alias.to_string(), canonical.to_string());
            rules.push(AliasRule {
                alias: alias.to_string(),
                slot: AliasSlot::Verb,
                canonical_target: vec![canonical.to_string()],
                source: AliasSource::Curated,
                status: MarginStatus::AutoAlias,
                cosine_similarity: 1.0,
                cosine_margin: 1.0,
            });
        }
    }

    for (alias, canonical) in curated_noun_aliases() {
        if active_leaf_segments.contains(canonical) && !top_levels.contains(alias) {
            noun_aliases.insert(alias.to_string(), canonical.to_string());
            rules.push(AliasRule {
                alias: alias.to_string(),
                slot: AliasSlot::Noun,
                canonical_target: vec![canonical.to_string()],
                source: AliasSource::Curated,
                status: MarginStatus::AutoAlias,
                cosine_similarity: 1.0,
                cosine_margin: 1.0,
            });
        }
    }

    // Fold any catalog-declared aliases on CliProjection rows (`verb_aliases`, `noun_aliases`, `foreign_idioms`).
    for op in &catalog.operations {
        let Some(cli) = &op.cli else { continue };
        if cli.status == "retired" || cli.path.is_empty() {
            continue;
        }
        let target_noun = cli.path.first().map(|s| s.to_ascii_lowercase());
        let target_verb = cli.path.last().map(|s| s.to_ascii_lowercase());
        if let Some(canon_verb) = target_verb {
            for va in &cli.verb_aliases {
                let va_lc = va.to_ascii_lowercase();
                if !is_antonym_pair(&va_lc, &canon_verb) {
                    verb_aliases
                        .entry(va_lc.clone())
                        .or_insert_with(|| canon_verb.clone());
                    rules.push(AliasRule {
                        alias: va_lc,
                        slot: AliasSlot::Verb,
                        canonical_target: cli.path.clone(),
                        source: AliasSource::Catalog,
                        status: MarginStatus::AutoAlias,
                        cosine_similarity: 1.0,
                        cosine_margin: 1.0,
                    });
                }
            }
        }
        if let Some(canon_noun) = target_noun {
            for na in &cli.noun_aliases {
                let na_lc = na.to_ascii_lowercase();
                if !is_antonym_pair(&na_lc, &canon_noun) {
                    noun_aliases
                        .entry(na_lc.clone())
                        .or_insert_with(|| canon_noun.clone());
                    rules.push(AliasRule {
                        alias: na_lc,
                        slot: AliasSlot::Noun,
                        canonical_target: cli.path.clone(),
                        source: AliasSource::Catalog,
                        status: MarginStatus::AutoAlias,
                        cosine_similarity: 1.0,
                        cosine_margin: 1.0,
                    });
                }
            }
        }
        for fi in &cli.foreign_idioms {
            let trig: Vec<String> = fi
                .split_whitespace()
                .map(|s| s.to_ascii_lowercase())
                .collect();
            if trig.is_empty() {
                continue;
            }
            let trig_vec = local_embed_text(
                &format!("{} {}", trig.join(" "), op.description),
                EMBEDDING_MODEL_ID,
            );
            let target_vec = local_embed_text(
                &format!("{} {}", cli.path.join(" "), op.description),
                EMBEDDING_MODEL_ID,
            );
            let (sim, margin, status) = verify_embedding_margin(&trig_vec, &target_vec, &[]);
            foreign_idioms.push(ForeignIdiomRule {
                trigger: trig.clone(),
                canonical_path: cli.path.clone(),
                insert_args: Vec::new(),
                cosine_similarity: round4(sim),
                cosine_margin: round4(margin),
            });
            rules.push(AliasRule {
                alias: trig.join(" "),
                slot: AliasSlot::Idiom,
                canonical_target: cli.path.clone(),
                source: AliasSource::Catalog,
                status,
                cosine_similarity: round4(sim),
                cosine_margin: round4(margin),
            });
        }
    }

    // Curated foreign idioms verified against active CLI paths + local embedding projection.
    let op_vectors: Vec<(Vec<String>, Vec<f32>)> = catalog
        .operations
        .iter()
        .filter_map(|op| {
            let cli = op.cli.as_ref()?;
            if cli.status == "retired" {
                return None;
            }
            let text = format!("{} {} {}", cli.path.join(" "), op.title, op.description);
            Some((
                cli.path.iter().map(|s| s.to_ascii_lowercase()).collect(),
                local_embed_text(&text, EMBEDDING_MODEL_ID),
            ))
        })
        .collect();

    for seed in curated_foreign_idioms() {
        let canon: Vec<String> = seed
            .canonical_path
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        if !active_paths.contains(&canon) {
            continue;
        }
        let trig: Vec<String> = seed
            .trigger
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        if foreign_idioms
            .iter()
            .any(|existing| existing.trigger == trig)
        {
            continue;
        }
        let target_vec = local_embed_text(
            &format!("{} {}", canon.join(" "), seed.description),
            EMBEDDING_MODEL_ID,
        );
        let alias_vec = local_embed_text(
            &format!("{} {}", trig.join(" "), seed.description),
            EMBEDDING_MODEL_ID,
        );
        let other_refs: Vec<&[f32]> = op_vectors
            .iter()
            .filter(|(p, _)| *p != canon)
            .map(|(_, v)| v.as_slice())
            .collect();
        let (sim, margin, status) = verify_embedding_margin(&alias_vec, &target_vec, &other_refs);
        foreign_idioms.push(ForeignIdiomRule {
            trigger: trig.clone(),
            canonical_path: canon.clone(),
            insert_args: seed.insert_args.iter().map(|s| s.to_string()).collect(),
            cosine_similarity: round4(sim),
            cosine_margin: round4(margin),
        });
        rules.push(AliasRule {
            alias: trig.join(" "),
            slot: AliasSlot::Idiom,
            canonical_target: canon,
            source: AliasSource::Curated,
            status,
            cosine_similarity: round4(sim),
            cosine_margin: round4(margin),
        });
    }

    foreign_idioms.sort_by(|a, b| a.trigger.cmp(&b.trigger));
    rules.sort_by(|a, b| {
        a.alias
            .cmp(&b.alias)
            .then_with(|| a.canonical_target.cmp(&b.canonical_target))
    });

    let antonym_banlist = ANTONYM_BANLIST
        .iter()
        .map(|(a, b)| [a.to_string(), b.to_string()])
        .collect();

    let lockfile = SemanticAliasesLockfile {
        schema_version: 1,
        generated_from: OPERATIONS_CATALOG_REL.to_string(),
        min_cosine_similarity: MIN_COSINE_SIMILARITY,
        min_cosine_margin: MIN_COSINE_MARGIN,
        antonym_banlist,
        verb_aliases,
        noun_aliases,
        foreign_idioms,
        rules,
    };

    validate_no_antonyms_or_collisions(&lockfile, &active_paths)?;
    Ok(lockfile)
}

pub fn serialize_lockfile(lockfile: &SemanticAliasesLockfile) -> Result<String> {
    let mut s =
        serde_json::to_string_pretty(lockfile).context("serialize semantic-aliases.v1.json")?;
    s.push('\n');
    Ok(s)
}

pub fn run(repo_root: &Path, write: bool) -> Result<()> {
    let cat_path = repo_root.join(OPERATIONS_CATALOG_REL);
    let cat_raw =
        read_utf8_path_capped(&cat_path).with_context(|| format!("read {}", cat_path.display()))?;
    let catalog: OperationsCatalog =
        serde_yaml::from_str(&cat_raw).with_context(|| format!("parse {}", cat_path.display()))?;

    let lockfile = build_lockfile(&catalog)?;
    let rendered = serialize_lockfile(&lockfile)?;

    // Validate against JSON schema.
    let schema_path = repo_root.join(SEMANTIC_ALIASES_SCHEMA_REL);
    let schema_raw = read_utf8_path_capped(&schema_path)
        .with_context(|| format!("read {}", schema_path.display()))?;
    let schema_val: serde_json::Value = serde_json::from_str(&schema_raw)
        .with_context(|| format!("parse {}", schema_path.display()))?;
    let validator =
        vox_jsonschema_util::compile_validator(&schema_val, schema_path.display().to_string())?;
    let doc_val: serde_json::Value = serde_json::from_str(&rendered)?;
    vox_jsonschema_util::validate(
        &doc_val,
        &validator,
        "contracts/cli/semantic-aliases.v1.json vs semantic-aliases.v1.schema.json",
    )?;

    let out_path = repo_root.join(SEMANTIC_ALIASES_REL);
    if write {
        std::fs::write(&out_path, &rendered)
            .with_context(|| format!("write {}", out_path.display()))?;
        println!(
            "cli-synonyms wrote {} ({} verb, {} noun, {} foreign idioms)",
            SEMANTIC_ALIASES_REL,
            lockfile.verb_aliases.len(),
            lockfile.noun_aliases.len(),
            lockfile.foreign_idioms.len()
        );
    } else {
        let on_disk = read_utf8_path_capped(&out_path)
            .with_context(|| format!("read {}", out_path.display()))?;
        if on_disk != rendered {
            bail!(
                "{} is out of date; run `vox ci cli-synonyms --write` to regenerate",
                SEMANTIC_ALIASES_REL
            );
        }
        println!(
            "cli-synonyms OK ({} verb, {} noun, {} foreign idioms verified)",
            lockfile.verb_aliases.len(),
            lockfile.noun_aliases.len(),
            lockfile.foreign_idioms.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ci::operations_catalog::{CliProjection, OperationRow};

    fn sample_catalog() -> OperationsCatalog {
        OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                OperationRow {
                    id: "secrets.status".into(),
                    title: "Secrets Status".into(),
                    description: "Show secret readiness".into(),
                    description_human: None,
                    owner_crate: None,
                    domain: None,
                    product_lane: "platform".into(),
                    intent_tags: vec![],
                    side_effect_class: Some("read_only".into()),
                    scope_kind: None,
                    reversible: None,
                    requires_repo: None,
                    preferred_for_models: None,
                    human_takeover_friendly: None,
                    mens_planner_visible: None,
                    canonical_name: None,
                    latin_aliases: None,
                    mcp: None,
                    cli: Some(CliProjection {
                        path: vec!["secrets".into(), "status".into()],
                        status: "active".into(),
                        latin_ns: None,
                        handler_rust: None,
                        feature_gate: None,
                        catalog_group: None,
                        ref_cli_required: None,
                        reachability_required: None,
                        verb_aliases: vec![],
                        noun_aliases: vec![],
                        foreign_idioms: vec![],
                    }),
                },
                OperationRow {
                    id: "audit.code".into(),
                    title: "Audit Code".into(),
                    description: "Run static code audit".into(),
                    description_human: None,
                    owner_crate: None,
                    domain: None,
                    product_lane: "platform".into(),
                    intent_tags: vec![],
                    side_effect_class: Some("read_only".into()),
                    scope_kind: None,
                    reversible: None,
                    requires_repo: None,
                    preferred_for_models: None,
                    human_takeover_friendly: None,
                    mens_planner_visible: None,
                    canonical_name: None,
                    latin_aliases: None,
                    mcp: None,
                    cli: Some(CliProjection {
                        path: vec!["audit".into(), "code".into()],
                        status: "active".into(),
                        latin_ns: None,
                        handler_rust: None,
                        feature_gate: None,
                        catalog_group: None,
                        ref_cli_required: None,
                        reachability_required: None,
                        verb_aliases: vec![],
                        noun_aliases: vec![],
                        foreign_idioms: vec![],
                    }),
                },
                OperationRow {
                    id: "catalog.remove".into(),
                    title: "Catalog Remove".into(),
                    description: "Remove a catalog entry".into(),
                    description_human: None,
                    owner_crate: None,
                    domain: None,
                    product_lane: "platform".into(),
                    intent_tags: vec![],
                    side_effect_class: Some("writes_files".into()),
                    scope_kind: None,
                    reversible: None,
                    requires_repo: None,
                    preferred_for_models: None,
                    human_takeover_friendly: None,
                    mens_planner_visible: None,
                    canonical_name: None,
                    latin_aliases: None,
                    mcp: None,
                    cli: Some(CliProjection {
                        path: vec!["catalog".into(), "remove".into()],
                        status: "active".into(),
                        latin_ns: None,
                        handler_rust: None,
                        feature_gate: None,
                        catalog_group: None,
                        ref_cli_required: None,
                        reachability_required: None,
                        verb_aliases: vec![],
                        noun_aliases: vec![],
                        foreign_idioms: vec![],
                    }),
                },
                OperationRow {
                    id: "catalog.list".into(),
                    title: "Catalog List".into(),
                    description: "List catalog entries".into(),
                    description_human: None,
                    owner_crate: None,
                    domain: None,
                    product_lane: "platform".into(),
                    intent_tags: vec![],
                    side_effect_class: Some("read_only".into()),
                    scope_kind: None,
                    reversible: None,
                    requires_repo: None,
                    preferred_for_models: None,
                    human_takeover_friendly: None,
                    mens_planner_visible: None,
                    canonical_name: None,
                    latin_aliases: None,
                    mcp: None,
                    cli: Some(CliProjection {
                        path: vec!["catalog".into(), "list".into()],
                        status: "active".into(),
                        latin_ns: None,
                        handler_rust: None,
                        feature_gate: None,
                        catalog_group: None,
                        ref_cli_required: None,
                        reachability_required: None,
                        verb_aliases: vec![],
                        noun_aliases: vec![],
                        foreign_idioms: vec![],
                    }),
                },
            ],
        }
    }

    #[test]
    fn antonym_banlist_rejects_opposite_verb_alias() {
        let cat = sample_catalog();
        let mut lockfile = build_lockfile(&cat).expect("baseline valid");
        lockfile
            .verb_aliases
            .insert("remove".to_string(), "add".to_string());
        let paths = active_cli_paths(&cat);
        let err = validate_no_antonyms_or_collisions(&lockfile, &paths)
            .expect_err("antonym alias remove->add must fail");
        assert!(err.to_string().contains("antonym firewall"));

        let mut lockfile2 = build_lockfile(&cat).expect("baseline valid");
        lockfile2
            .verb_aliases
            .insert("get".to_string(), "set".to_string());
        let err2 = validate_no_antonyms_or_collisions(&lockfile2, &paths)
            .expect_err("antonym alias get->set must fail");
        assert!(err2.to_string().contains("antonym firewall"));
    }

    #[test]
    fn canonical_collision_is_rejected() {
        let cat = sample_catalog();
        let mut lockfile = build_lockfile(&cat).expect("baseline valid");
        lockfile.foreign_idioms.push(ForeignIdiomRule {
            trigger: vec!["audit".to_string(), "code".to_string()],
            canonical_path: vec!["secrets".to_string(), "status".to_string()],
            insert_args: vec![],
            cosine_similarity: 0.9,
            cosine_margin: 0.2,
        });
        let paths = active_cli_paths(&cat);
        let err = validate_no_antonyms_or_collisions(&lockfile, &paths)
            .expect_err("collision with active path must fail");
        assert!(err.to_string().contains("canonical collision"));
    }

    #[test]
    fn cosine_margin_gate_demotes_ambiguous_neighbors() {
        let alias = [1.0_f32, 0.0, 0.0];
        let target = [0.8_f32, 0.6, 0.0];
        let neighbor = [0.78_f32, 0.625, 0.0];
        let (_sim, margin, status) = verify_embedding_margin(&alias, &target, &[&neighbor]);
        assert!(margin < MIN_COSINE_MARGIN);
        assert_eq!(status, MarginStatus::Ambiguous);

        let distant_neighbor = [0.2_f32, 0.98, 0.0];
        let (_sim2, margin2, status2) =
            verify_embedding_margin(&alias, &target, &[&distant_neighbor]);
        assert!(margin2 >= MIN_COSINE_MARGIN);
        assert_eq!(status2, MarginStatus::AutoAlias);
    }

    #[test]
    fn lockfile_generation_is_deterministic_and_idempotent() {
        let cat = sample_catalog();
        let a = serialize_lockfile(&build_lockfile(&cat).expect("first build")).expect("ser 1");
        let b = serialize_lockfile(&build_lockfile(&cat).expect("second build")).expect("ser 2");
        assert_eq!(a, b);
    }
}
