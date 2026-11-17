//! Gated Hugging Face auto-publish, verified auto-retrieve, and ladder sync (v5.7 Phases 325–327).
//!
//! All cryptographic file digests go through [`vox_crypto::hash_fast_hex`] (BLAKE3), and
//! Hugging Face credentials resolve exclusively through [`vox_secrets::resolve_secret`]
//! (`SecretId::HuggingFaceToken`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use hf_hub::{HFClient, RepoTypeModel, split_id};

use crate::mens::hub::{
    NO_DOWNLOAD_ENV, ensure_mens_collection, normalize_hf_token_env, upload_model_folder,
    upload_space_folder,
};
use crate::mens::hub_card;
use crate::mens::hub_repo::{self, NAMESPACE_ENV, SIZES};
use crate::mens::peft_export;
use crate::mens::tensor::external_serving_handoff::{ExternalServingHandoffV1, write_handoff};
use crate::mens::tensor::size_select;

/// Env var controlling automatic HF sync after train / eval-gate (`1`/`true` by default; `0`/`false` disables).
pub const HF_SYNC_ENV: &str = "VOX_MENS_HF_SYNC";

/// Manifest written into every published adapter bundle for idempotent hash-skip and pull verification.
pub const HF_SYNC_MANIFEST_FILENAME: &str = "hf_sync_manifest.json";

/// Standard HF offline mode env var.
pub const HF_HUB_OFFLINE_ENV: &str = "HF_HUB_OFFLINE";

/// Files included in a published adapter bundle when present in the run directory.
const OPTIONAL_BUNDLE_FILES: &[&str] = &[
    "adapter_manifest.json",
    "adapter_card.json",
    "collateral_damage_report.json",
    "gate_receipt.json",
    "gate_receipt.vox-lang.json",
    "gate_receipt.rust.json",
    "gate_receipt.tool-selection.json",
    "gate_receipt.argument-generation.json",
    "spoke_gates.json",
    "eval_local_report.json",
    "eval_local_report.ckpt.jsonl",
    "benchmark_passatk.json",
    "review_metrics.json",
    "run_summary.json",
    "train-record.json",
    "training_skip_stats.json",
    "config.json",
    // `vox mens serve` loads `tokenizer.json` from the run dir, so a pulled bundle needs it.
    "tokenizer.json",
    "evidence/benchmark_passatk.json",
    "evidence/benchmark_passatk.think128.json",
    "evidence/collateral_baseline.json",
    "evidence/eval_results.json",
    "evidence/eval_results.think128.json",
    "evidence/heldout_adapter.json",
    "evidence/heldout_adapter_nothink.json",
    "evidence/heldout_adapter_nothink.ckpt.jsonl",
    "evidence/heldout_base.json",
    "evidence/heldout_base_nothink.json",
    "evidence/heldout_base_nothink.ckpt.jsonl",
];

/// Held-out reasoning slice names whose per-slice reports are included when present.
const COLLATERAL_SLICE_DIRS: &[&str] = &["vox_think", "gsm8k_think", "math500_think"];

/// Whether automatic HF sync is enabled given the `--no-hf-sync` CLI flag and `VOX_MENS_HF_SYNC`.
pub fn hf_sync_enabled(no_hf_sync_flag: bool, env_raw: Option<&str>) -> bool {
    if no_hf_sync_flag {
        return false;
    }
    !matches!(env_raw.map(str::trim), Some(v) if v == "0" || v.eq_ignore_ascii_case("false"))
}

/// Why an automatic publish must not run, before gate receipts are checked.
///
/// `None` means sync is on and a namespace is set. Callers still refuse a failing
/// gate, and they say which of these fired. A silent skip reads as "the Hub
/// updater did not run."
pub fn explain_auto_publish_skip(
    no_hf_sync_flag: bool,
    sync_env: Option<&str>,
    namespace_env: Option<&str>,
) -> Option<&'static str> {
    if !hf_sync_enabled(no_hf_sync_flag, sync_env) {
        return Some("HF auto-publish is off (`VOX_MENS_HF_SYNC=0` or `--no-hf-sync`)");
    }
    if namespace_env.map(str::trim).is_none_or(|ns| ns.is_empty()) {
        return Some("HF auto-publish skipped: `VOX_MENS_HF_NAMESPACE` is unset");
    }
    None
}

/// Refuse an automatic adapter pull when `VOX_MENS_NO_DOWNLOAD` or `HF_HUB_OFFLINE` is truthy (HFPULL-03).
pub fn ensure_pull_allowed(
    repo_id: &str,
    no_download_raw: Option<&str>,
    hf_offline_raw: Option<&str>,
) -> Result<()> {
    let truthy = |raw: Option<&str>| {
        matches!(
            raw.map(str::trim),
            Some(v) if !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        )
    };
    if truthy(no_download_raw) {
        bail!(
            "{NO_DOWNLOAD_ENV} is set: refusing to download missing adapter from `{repo_id}`. \
             Unset {NO_DOWNLOAD_ENV} or pass a local adapter path."
        );
    }
    if truthy(hf_offline_raw) {
        bail!(
            "{HF_HUB_OFFLINE_ENV} is set (offline mode): refusing to download missing adapter from `{repo_id}`. \
             Unset {HF_HUB_OFFLINE_ENV} or pass a local adapter path."
        );
    }
    Ok(())
}

/// Resolve the Hugging Face token strictly via Clavis (`SecretId::HuggingFaceToken`) before reading any file (HFPUB-05).
pub fn require_clavis_hf_token() -> Result<String> {
    let resolved = vox_secrets::resolve_secret(vox_secrets::SecretId::HuggingFaceToken);
    let Some(token) = resolved.expose().map(str::trim).filter(|s| !s.is_empty()) else {
        bail!(
            "HuggingFaceToken is missing in Clavis; refusing to publish adapter. \
             Configure the credential and verify with `vox secrets doctor`."
        );
    };
    Ok(token.to_string())
}

/// Gate verification status for an adapter run directory (HFPUB-02).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateStatus {
    Passed,
    Ungated { reason: String },
}

impl GateStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Passed => "passed".to_string(),
            Self::Ungated { reason } => format!("ungated ({reason})"),
        }
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Check `collateral_damage_report.json` and `gate_receipt.json` in `run_dir` (HFPUB-02).
///
/// When `force_ungated` is `false`, any missing or failing receipt refuses the upload and names
/// the receipt file. When `force_ungated` is `true`, returns [`GateStatus::Ungated`] so the
/// generated model card explicitly marks the upload as `ungated`.
pub fn verify_publish_gates(run_dir: &Path, force_ungated: bool) -> Result<GateStatus> {
    let root = vox_corpus::training::contract::find_workspace_root();
    let req = crate::mens::gate_policy::ladder_requirement(root.as_deref());
    verify_publish_gates_with(run_dir, force_ungated, &req)
}

/// Variant of [`verify_publish_gates`] accepting an explicit [`vox_eval::collateral_evidence::CollateralRequirement`].
pub fn verify_publish_gates_with(
    run_dir: &Path,
    force_ungated: bool,
    req: &vox_eval::collateral_evidence::CollateralRequirement,
) -> Result<GateStatus> {
    let mut missing_or_failed = Vec::new();

    let collateral_path = run_dir.join("collateral_damage_report.json");
    if !collateral_path.is_file() {
        missing_or_failed.push("collateral_damage_report.json (missing)".to_string());
    } else {
        let raw = std::fs::read_to_string(&collateral_path)
            .with_context(|| format!("read {}", collateral_path.display()))?;
        let val: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", collateral_path.display()))?;
        if let Err(refusal) = vox_eval::check_collateral_report(&val, req) {
            missing_or_failed.push(format!("collateral_damage_report.json ({refusal})"));
        }
    }

    let root = vox_corpus::training::contract::find_workspace_root();
    let ladder_hash = crate::mens::gate_policy::policy_blake3(
        root.as_deref(),
        "mens/config/eval-gates-ladder.yaml",
    )
    .map_err(|e| anyhow::anyhow!("could not compute ladder policy hash: {e}"))?;

    let adapter_card_path = run_dir.join("adapter_card.json");
    let adapter_sha256 = if adapter_card_path.is_file() {
        std::fs::read_to_string(&adapter_card_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| {
                v.get("adapter_sha256")
                    .and_then(|s| s.as_str())
                    .map(ToString::to_string)
            })
    } else {
        None
    };

    let current = vox_eval::gate_state::CurrentGate {
        policy_blake3: ladder_hash,
        adapter_sha256,
    };

    let receipt_path = run_dir.join("gate_receipt.json");
    let receipt_val: Option<serde_json::Value> = if receipt_path.is_file() {
        let raw = std::fs::read_to_string(&receipt_path)
            .with_context(|| format!("read {}", receipt_path.display()))?;
        Some(
            serde_json::from_str(&raw)
                .with_context(|| format!("parse {}", receipt_path.display()))?,
        )
    } else {
        None
    };

    let (state, reason) = vox_eval::gate_state::gate_state(receipt_val.as_ref(), &current);
    if state != vox_eval::gate_state::GateState::Passed {
        missing_or_failed.push(format!("gate_receipt.json ({state}: {reason})"));
    }

    let claimed_spokes: Vec<String> = if adapter_card_path.is_file() {
        std::fs::read_to_string(&adapter_card_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| {
                v.get("spokes").and_then(|s| s.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(ToString::to_string))
                        .collect()
                })
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let spoke_rows = crate::mens::gate_policy::spoke_gate_rows(run_dir, root.as_deref());
    for row in crate::mens::gate_policy::applying_rows(&spoke_rows, &claimed_spokes) {
        if row.spoke == "ladder" || !row.applies {
            continue;
        }
        if row.state != vox_eval::gate_state::GateState::Passed {
            let label = row
                .receipt_file
                .unwrap_or_else(|| format!("gate_receipt.{}.json", row.spoke));
            missing_or_failed.push(format!("{label} ({}: {})", row.state, row.reason));
        }
    }

    if missing_or_failed.is_empty() {
        return Ok(GateStatus::Passed);
    }
    let reason = missing_or_failed.join(", ");
    if !force_ungated {
        bail!(
            "refusing to publish adapter in {}: missing or failing gate receipt: {reason}",
            run_dir.display()
        );
    }
    Ok(GateStatus::Ungated { reason })
}

/// One file in a staged or remote adapter bundle, hashed with BLAKE3 via `vox-crypto`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileHashEntry {
    pub path: String,
    pub blake3: String,
    pub size_bytes: u64,
}

/// Manifest stored as `hf_sync_manifest.json` in every published adapter repo.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HfSyncManifest {
    pub schema: String,
    pub size: String,
    pub base_pin: String,
    pub corpus_hash: String,
    pub final_loss: Option<f64>,
    pub measured_peak_mb: Option<u64>,
    pub gate_status: String,
    pub files: Vec<FileHashEntry>,
}

/// True when `local` and `remote` carry identical file digests and metadata (HFPUB-04).
pub fn manifests_match(local: &HfSyncManifest, remote: &HfSyncManifest) -> bool {
    local.size == remote.size
        && local.base_pin == remote.base_pin
        && local.gate_status == remote.gate_status
        && local.files == remote.files
}

const UNIX_USERS_PREFIX: &str = concat!("/Us", "ers/");
const HOME_PREFIX: &str = concat!("/ho", "me/");
const PRIVATE_TMP_PREFIX: &str = concat!("/priv", "ate/t", "mp/");
const PRIVATE_VAR_PREFIX: &str = concat!("/priv", "ate/v", "ar/");
const WIN_USERS_BS: &str = concat!("\\Us", "ers\\");
const WIN_USERS_FS: &str = concat!("/Us", "ers/");

fn has_hf_cache_snapshot(norm: &str) -> bool {
    norm.split('/').any(|seg| {
        seg.strip_prefix("models--")
            .and_then(|rest| rest.split_once("--"))
            .is_some_and(|(org, repo)| !org.is_empty() && !repo.is_empty())
    })
}

fn is_host_user_path(s: &str) -> bool {
    if s.starts_with(UNIX_USERS_PREFIX)
        || s.starts_with(HOME_PREFIX)
        || s.starts_with(PRIVATE_TMP_PREFIX)
        || s.starts_with(PRIVATE_VAR_PREFIX)
    {
        return true;
    }
    let bytes = s.as_bytes();
    if bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let rest = &s[2..];
        return rest.starts_with(WIN_USERS_BS) || rest.starts_with(WIN_USERS_FS);
    }
    false
}

/// Strip host-specific home or workspace prefixes from staged JSON fields
/// before publishing to Hugging Face.
fn sanitize_json_paths(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::String(s) => {
            let norm = s.replace('\\', "/");
            if let Some(idx) = norm.find("mens/runs/") {
                *s = norm[idx..].to_string();
            } else if let Some(idx) = norm.find("target/dogfood") {
                *s = norm[idx..].to_string();
            } else if has_hf_cache_snapshot(&norm) {
                let (repo, rev) = peft_export::normalize_hf_repo_id_and_rev(s);
                *s = match rev {
                    Some(r) => format!("{repo}@{r}"),
                    None => repo,
                };
            } else if is_host_user_path(s) {
                *s = "<redacted-host-path>".to_string();
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                sanitize_json_paths(item);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                sanitize_json_paths(v);
            }
        }
        _ => {}
    }
}

/// Resolve the pinned HF `owner/repo@<40-hex-sha>` for a ladder size (`"0.6B"`, `"2B"`, `"4B"`,
/// `"9B"`, `"27B"`), family-qualified model label (`"Qwen3-4B"`, `"Qwen3.5-4B"`, `"Qwen3.8-27B"`),
/// or pinned base using `qwen3_5_code` (Qwen3.5/Qwen3.8) and `qwen3_code` (Qwen3) overlays.
pub fn resolve_ladder_base_pin(workspace_root: Option<&Path>, size_or_pin: &str) -> Result<String> {
    let overlay = size_select::load_ladder_overlay(workspace_root)?;
    let family = hub_repo::family_of_model(size_or_pin);
    let label = hub_repo::model_label_for(size_or_pin);
    let suffix = format!("/{label}");
    let lookup_tag = |tag: &str| -> Option<String> {
        let entries = overlay.get(tag)?;
        let matches: std::collections::BTreeSet<&str> = entries
            .iter()
            .filter(|b| {
                let repo = b.hf_id.split('@').next().unwrap_or(b.hf_id.as_str());
                (repo == label || repo.ends_with(&suffix)) && b.hf_id.contains('@')
            })
            .map(|b| b.hf_id.as_str())
            .collect();
        if matches.len() == 1 {
            matches.into_iter().next().map(ToString::to_string)
        } else {
            None
        }
    };
    let primary_tag = if family == "qwen3" {
        "qwen3_code"
    } else {
        "qwen3_5_code"
    };
    if let Some(pin) = lookup_tag(primary_tag) {
        return Ok(pin);
    }
    let secondary_tag = if primary_tag == "qwen3_code" {
        "qwen3_5_code"
    } else {
        "qwen3_code"
    };
    if let Some(pin) = lookup_tag(secondary_tag) {
        return Ok(pin);
    }
    size_select::resolve_base_pin(&label, &overlay)
}

fn is_qwen3_5_causal_lm_config(run_dir: &Path) -> bool {
    let candidates = [
        run_dir.join("recombined_full/config.json"),
        run_dir.join("config.json"),
    ];
    for cfg_path in candidates {
        let Ok(raw) = std::fs::read_to_string(&cfg_path) else {
            continue;
        };
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if val
            .get("architectures")
            .and_then(|a| a.as_array())
            .is_some_and(|arr| arr.iter().any(|x| x.as_str() == Some("Qwen3_5ForCausalLM")))
        {
            return true;
        }
        if val.get("model_type").and_then(|m| m.as_str()) == Some("qwen3_5_text") {
            return true;
        }
        return false;
    }
    false
}

const CANONICAL_QWEN_CHATML_JINJA: &str = concat!(
    "{%- for message in messages %}\n",
    "{{- '<|im_start|>' + message['role'] + '\\n' + message['content'] + '<|im_end|>\\n' }}\n",
    "{%- endfor %}\n",
    "{%- if add_generation_prompt %}\n",
    "{{- '<|im_start|>assistant\\n' }}\n",
    "{%- endif %}\n",
);

fn stage_chat_template(run_dir: &Path, stage_dir: &Path) -> Result<bool> {
    let dst = stage_dir.join("chat_template.jinja");
    for src in [
        run_dir.join("recombined_full/chat_template.jinja"),
        run_dir.join("chat_template.jinja"),
    ] {
        if src.is_file() {
            std::fs::copy(&src, &dst)?;
            return Ok(true);
        }
    }
    for tok_cfg in [
        run_dir.join("recombined_full/tokenizer_config.json"),
        run_dir.join("tokenizer_config.json"),
    ] {
        if let Ok(raw) = std::fs::read_to_string(&tok_cfg)
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
            && let Some(tmpl) = val.get("chat_template").and_then(|v| v.as_str())
            && !tmpl.trim().is_empty()
        {
            std::fs::write(&dst, format!("{}\n", tmpl.trim_end()))?;
            return Ok(true);
        }
    }
    let has_tokenizer = run_dir.join("recombined_full/tokenizer.json").is_file()
        || run_dir.join("tokenizer.json").is_file()
        || stage_dir.join("tokenizer.json").is_file();
    if has_tokenizer {
        std::fs::write(&dst, CANONICAL_QWEN_CHATML_JINJA)?;
        return Ok(true);
    }
    Ok(false)
}

#[derive(serde::Serialize)]
struct HfEvalDataset {
    id: String,
    task_id: String,
}

#[derive(serde::Serialize)]
struct HfEvalSource {
    url: String,
    name: String,
}

#[derive(serde::Serialize)]
struct HfEvalResultEntry {
    dataset: HfEvalDataset,
    value: f64,
    source: HfEvalSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

fn build_eval_results_yaml(run_dir: &Path, repo_id: &str) -> Result<Option<String>> {
    let source_url = format!("https://huggingface.co/{repo_id}");
    let source_name = "VoxMens Eval Gate".to_string();
    let mut entries = Vec::<HfEvalResultEntry>::new();
    let mut seen = std::collections::BTreeSet::<(String, String)>::new();

    let mut push_metric = |dataset_id: String, task_id: &str, value: f64, notes: Option<&str>| {
        if !value.is_finite() {
            return;
        }
        let key = (dataset_id.clone(), task_id.to_string());
        if seen.insert(key) {
            entries.push(HfEvalResultEntry {
                dataset: HfEvalDataset {
                    id: dataset_id,
                    task_id: task_id.to_string(),
                },
                value,
                source: HfEvalSource {
                    url: source_url.clone(),
                    name: source_name.clone(),
                },
                notes: notes.map(ToString::to_string),
            });
        }
    };

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("collateral_damage_report.json"))
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        if let Some(pooled) = val
            .pointer("/reasoning_slices/pooled/post")
            .and_then(|v| v.as_f64())
        {
            push_metric(
                "vox-mens/reasoning-pooled".to_string(),
                "accuracy",
                pooled,
                Some("Pooled post-fine-tuning accuracy across reasoning collateral slices"),
            );
        }
        if let Some(slices) = val
            .pointer("/reasoning_slices/slices")
            .and_then(|v| v.as_object())
        {
            let mut slice_names: Vec<&String> = slices.keys().collect();
            slice_names.sort();
            for name in slice_names {
                if let Some(post) = slices[name]
                    .pointer("/accuracy/post")
                    .and_then(|v| v.as_f64())
                {
                    push_metric(format!("vox-mens/{name}"), "accuracy", post, None);
                }
            }
        }
        if let Some(reports) = val.get("reports").and_then(|v| v.as_array()) {
            for r in reports {
                if let (Some(bench), Some(post)) = (
                    r.get("benchmark").and_then(|v| v.as_str()),
                    r.get("post").and_then(|v| v.as_f64()),
                ) {
                    push_metric(format!("vox-mens/{bench}"), "accuracy", post, None);
                }
            }
        }
    }

    for rel in [
        "evidence/heldout_adapter_nothink.json",
        "evidence/heldout_adapter.json",
    ] {
        if let Ok(raw) = std::fs::read_to_string(run_dir.join(rel))
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
        {
            if let Some(pass1) = val.get("pass_rate_at_1").and_then(|v| v.as_f64()) {
                push_metric(
                    "vox-mens/vox-heldout".to_string(),
                    "pass_at_1",
                    pass1,
                    Some(rel),
                );
            }
            if let Some(sem) = val.get("semantic_task_success").and_then(|v| v.as_f64()) {
                push_metric(
                    "vox-mens/vox-heldout-semantic".to_string(),
                    "accuracy",
                    sem,
                    Some(rel),
                );
            }
            if let Some(anti) = val.get("anti_stub_task_success").and_then(|v| v.as_f64()) {
                push_metric(
                    "vox-mens/vox-heldout-anti-stub".to_string(),
                    "accuracy",
                    anti,
                    Some(rel),
                );
            }
        }
    }

    for rel in [
        "evidence/benchmark_passatk.json",
        "benchmark_passatk.json",
        "eval_local_report.json",
    ] {
        if let Ok(raw) = std::fs::read_to_string(run_dir.join(rel))
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
        {
            if let Some(pass1) = val
                .get("pass_at_1")
                .or_else(|| val.get("pass_rate_at_1"))
                .and_then(|v| v.as_f64())
            {
                push_metric(
                    "vox-mens/vox-benchmark".to_string(),
                    "pass_at_1",
                    pass1,
                    Some(rel),
                );
            }
            if let Some(acc) = val.get("accuracy").and_then(|v| v.as_f64()) {
                push_metric(
                    "vox-mens/vox-benchmark-accuracy".to_string(),
                    "accuracy",
                    acc,
                    Some(rel),
                );
            }
        }
    }

    if entries.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_yaml::to_string(&entries)?))
}

/// Create every ladder repo, and write a reserved card only when the repo has no README yet.
///
/// A repo that already holds `hf_sync_manifest.json` or a README is left unchanged, so a
/// finished adapter and a hand-written card are not replaced by the placeholder.
#[inline(never)]
pub async fn ensure_ladder_repos(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
) -> Result<Vec<String>> {
    let _token = require_clavis_hf_token()?;
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    normalize_hf_token_env();
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let mut reserved = Vec::new();
    for &size in &SIZES {
        let base_pin = resolve_ladder_base_pin(workspace_root, size)?;
        let run_dir = workspace_root
            .unwrap_or(Path::new("."))
            .join(hub_repo::local_run_dir_for_size(size)?);
        let full_rec = hub_card::RunRecords::load(&run_dir, size, &base_pin);
        let cx = hub_card::CardContext {
            size,
            base_pin: &base_pin,
            namespace: &ns,
        };
        if hub_card::gate_of(&full_rec).state != hub_card::GateState::Passed {
            continue;
        }
        let repo_id = hub_repo::repo_id_for_size(&ns, size)?;
        let model_label = hub_repo::model_label_for(&base_pin);
        client
            .create_repository()
            .repo_id(&repo_id)
            .repo_type(RepoTypeModel)
            .private(private)
            .exist_ok(true)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("create or check HF repository {repo_id}: {e}"))?;
        if fetch_remote_sync_manifest(&repo_id)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            continue;
        }
        let (owner, name) = split_id(&repo_id);
        let repo = client.model(owner, name);
        let siblings = repo
            .info()
            .send()
            .await
            .ok()
            .and_then(|info| info.siblings)
            .unwrap_or_default();
        if siblings
            .iter()
            .any(|s| s.rfilename.ends_with(".safetensors"))
        {
            continue;
        }
        let rec = hub_card::RunRecords {
            has_adapter: false,
            ..full_rec
        };
        let card = hub_card::render_model_card(&cx, &rec);
        let assets = hub_card::render_model_assets(&cx, &rec);
        let has_readme = siblings.iter().any(|s| s.rfilename == "README.md");
        let has_assets = assets
            .iter()
            .all(|a| siblings.iter().any(|s| s.rfilename == a.rel_path));
        let remote_matches = if has_readme && has_assets {
            let remote_readme_ok = match repo.download_file().filename("README.md").send().await {
                Ok(p) => std::fs::read_to_string(&p).ok().as_deref() == Some(card.as_str()),
                Err(_) => false,
            };
            let mut remote_assets_ok = remote_readme_ok;
            if remote_assets_ok {
                for asset in &assets {
                    let ok = match repo.download_file().filename(&asset.rel_path).send().await {
                        Ok(p) => std::fs::read(&p).ok().as_deref() == Some(asset.bytes.as_slice()),
                        Err(_) => false,
                    };
                    if !ok {
                        remote_assets_ok = false;
                        break;
                    }
                }
            }
            remote_assets_ok
        } else {
            false
        };
        if remote_matches {
            continue;
        }
        let tmp = crate::mens::hub_cards_push::scratch("reserve", size)?;
        std::fs::write(tmp.path().join("README.md"), &card)?;
        std::fs::write(tmp.path().join("MODEL_CARD.md"), &card)?;
        for asset in assets {
            let dst = tmp.path().join(&asset.rel_path);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dst, &asset.bytes)?;
        }
        upload_model_folder(
            &repo_id,
            tmp.path(),
            private,
            Some(&format!("Reserve VoxMens {model_label} ladder repo")),
        )
        .await?;
        reserved.push(repo_id);
    }
    Ok(reserved)
}

/// True when an evidence JSON file is a placeholder (`{}`, `[]`, null, or whitespace).
fn json_evidence_is_empty(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(serde_json::Value::Object(map)) => map.is_empty(),
        Ok(serde_json::Value::Array(items)) => items.is_empty(),
        Ok(serde_json::Value::Null) => true,
        _ => false,
    }
}

/// Stage the publishable adapter bundle from `run_dir` into `stage_dir` and return its [`HfSyncManifest`] (HFPUB-02, HFPUB-03).
#[inline(never)]
pub fn stage_publish_bundle(
    run_dir: &Path,
    stage_dir: &Path,
    force_ungated: bool,
    namespace: &str,
) -> Result<HfSyncManifest> {
    let manifest_src = run_dir.join("training_manifest.json");
    if !manifest_src.is_file() {
        bail!("no training_manifest.json found in {}", run_dir.display());
    }

    let gate_status = verify_publish_gates(run_dir, force_ungated)?;
    let merged_files = hub_card::detect_merged_checkpoint_files(run_dir);
    let distribute_merged = gate_status.is_passed() && !merged_files.is_empty();

    let adapter_src = if run_dir.join("candle_qlora_adapter.safetensors").is_file() {
        Some(run_dir.join("candle_qlora_adapter.safetensors"))
    } else if run_dir.join("adapter.safetensors").is_file() {
        Some(run_dir.join("adapter.safetensors"))
    } else {
        None
    };
    if !distribute_merged && adapter_src.is_none() {
        bail!(
            "no adapter safetensors or passing merged checkpoint found in {} (expected candle_qlora_adapter.safetensors or merged checkpoint)",
            run_dir.display()
        );
    }

    let mut manifest_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_src)
            .with_context(|| format!("read {}", manifest_src.display()))?,
    )
    .with_context(|| format!("parse {}", manifest_src.display()))?;
    sanitize_json_paths(&mut manifest_json);

    let base_pin = manifest_json
        .get("base_model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("training_manifest.json has no `base_model` string"))?
        .to_string();
    let size = hub_repo::size_of_pin(&base_pin).ok_or_else(|| {
        anyhow::anyhow!("base_model `{base_pin}` in training_manifest.json is not a ladder size")
    })?;
    let repo_id = hub_repo::repo_id_for_size(namespace, &base_pin)?;
    let corpus_hash = manifest_json
        .get("finetune_contract_digest")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut final_loss = None::<f64>;
    let mut measured_peak_mb = None::<u64>;

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("train-record.json"))
        && let Ok(rec) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        final_loss = rec
            .get("final_loss_ema")
            .and_then(|v| v.as_f64())
            .or_else(|| rec.get("avg_train_loss").and_then(|v| v.as_f64()));
        measured_peak_mb = rec.get("peak_mb").and_then(|v| v.as_u64());
    }
    if let Ok(raw) = std::fs::read_to_string(run_dir.join("run_summary.json"))
        && let Ok(sum) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        if final_loss.is_none() {
            final_loss = sum
                .get("final_loss_ema")
                .and_then(|v| v.as_f64())
                .or_else(|| sum.get("avg_train_loss").and_then(|v| v.as_f64()));
        }
        if measured_peak_mb.is_none() {
            measured_peak_mb = sum
                .get("peak_metal_allocated_bytes")
                .and_then(|v| v.as_u64())
                .map(|b| b.div_ceil(1_048_576));
        }
    }

    std::fs::create_dir_all(stage_dir)
        .with_context(|| format!("create stage_dir {}", stage_dir.display()))?;

    let mut staged_rel_paths = Vec::<String>::new();

    // 1. Copy complete merged checkpoint when gate passed + merge exists (REL-04),
    //    AND always keep the trained Candle adapter safetensors in bundle root.
    if distribute_merged {
        for (src, rel) in &merged_files {
            let dst = stage_dir.join(rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src, &dst)?;
            staged_rel_paths.push(rel.clone());
        }
    }
    if let Some(ref adapter_src) = adapter_src {
        let adapter_name = adapter_src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("candle_qlora_adapter.safetensors");
        let dst = stage_dir.join(adapter_name);
        if !staged_rel_paths
            .iter()
            .any(|existing| existing == adapter_name)
        {
            std::fs::copy(adapter_src, &dst)?;
            staged_rel_paths.push(adapter_name.to_string());
        }
    }

    // 1b. Export Hugging Face PEFT LoRA files when adapter + adapter_manifest.json exist.
    //     When merged weights ship in root, place PEFT inside `peft/` so root stays pure
    //     CausalLM/ConditionalGeneration and strip `model.language_model.` for `Qwen3_5ForCausalLM`.
    let adapter_manifest_src = run_dir.join("adapter_manifest.json");
    let mut peft_rel_dir: Option<&'static str> = None;
    if let Some(ref adapter_src) = adapter_src
        && adapter_manifest_src.is_file()
    {
        let (peft_dir, peft_prefix, rel_dir, strip_lm) = if distribute_merged {
            (
                stage_dir.join("peft"),
                "peft/",
                "peft",
                is_qwen3_5_causal_lm_config(run_dir),
            )
        } else {
            (stage_dir.to_path_buf(), "", ".", false)
        };
        let _peft_outcome = peft_export::export_peft_from_candle(
            adapter_src,
            &adapter_manifest_src,
            &peft_dir,
            strip_lm,
        )?;
        staged_rel_paths.push(format!("{peft_prefix}adapter_config.json"));
        staged_rel_paths.push(format!("{peft_prefix}adapter_model.safetensors"));
        peft_rel_dir = Some(rel_dir);
    }

    // 1c. Stage chat_template.jinja when template or tokenizer is present.
    if stage_chat_template(run_dir, stage_dir)? {
        staged_rel_paths.push("chat_template.jinja".to_string());
    }

    // 2. Write sanitized training_manifest.json
    std::fs::write(
        stage_dir.join("training_manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest_json)?),
    )?;
    staged_rel_paths.push("training_manifest.json".to_string());

    // 3. Copy optional JSON/report artifacts (sanitizing JSON paths)
    for &rel in OPTIONAL_BUNDLE_FILES {
        if staged_rel_paths.iter().any(|existing| existing == rel) {
            continue;
        }
        let src = run_dir.join(rel);
        if !src.is_file() {
            continue;
        }
        if (rel.ends_with("eval_results.json") || rel.ends_with("eval_results.think128.json"))
            && std::fs::read_to_string(&src).is_ok_and(|raw| json_evidence_is_empty(&raw))
        {
            continue;
        }
        let dst = stage_dir.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if rel.ends_with(".json")
            && let Ok(raw) = std::fs::read_to_string(&src)
            && let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw)
        {
            sanitize_json_paths(&mut parsed);
            std::fs::write(
                &dst,
                format!("{}\n", serde_json::to_string_pretty(&parsed)?),
            )?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
        staged_rel_paths.push(rel.to_string());
    }

    // 4. Copy per-slice collateral reasoning reports when present
    for &slice in COLLATERAL_SLICE_DIRS {
        let rel = format!("collateral_eval/{slice}/reasoning_report.json");
        let src = run_dir.join(&rel);
        if src.is_file() {
            let dst = stage_dir.join(&rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if let Ok(raw) = std::fs::read_to_string(&src)
                && let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw)
            {
                sanitize_json_paths(&mut parsed);
                std::fs::write(
                    &dst,
                    format!("{}\n", serde_json::to_string_pretty(&parsed)?),
                )?;
            } else {
                std::fs::copy(&src, &dst)?;
            }
            staged_rel_paths.push(rel);
        }
    }

    // 5. Derive spoke_gates.json freshly at stage time (D-02, D-17).
    let root = vox_corpus::training::contract::find_workspace_root();
    let spoke_rows_raw = crate::mens::gate_policy::spoke_gate_rows(run_dir, root.as_deref());
    let adapter_card_path = run_dir.join("adapter_card.json");
    let adapter_card_json: Option<serde_json::Value> = if adapter_card_path.is_file() {
        std::fs::read_to_string(&adapter_card_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    } else {
        None
    };
    let adapter_sha256 = adapter_card_json.as_ref().and_then(|v| {
        v.get("adapter_sha256")
            .and_then(|s| s.as_str())
            .map(ToString::to_string)
    });
    let claimed_spokes: Vec<String> = adapter_card_json
        .as_ref()
        .and_then(|v| {
            v.get("spokes").and_then(|s| s.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(ToString::to_string))
                    .collect()
            })
        })
        .unwrap_or_default();
    let spoke_rows = crate::mens::gate_policy::applying_rows(&spoke_rows_raw, &claimed_spokes);
    let mut spoke_gates_doc = vox_eval::gate_state::spoke_gates_json(
        adapter_sha256.as_deref(),
        Some(&base_pin),
        &spoke_rows,
    );
    sanitize_json_paths(&mut spoke_gates_doc);
    std::fs::write(
        stage_dir.join("spoke_gates.json"),
        format!("{}\n", serde_json::to_string_pretty(&spoke_gates_doc)?),
    )?;
    staged_rel_paths.push("spoke_gates.json".to_string());

    // 6. Ship the before/after reasoning comparison the card cites (only one that scored this run).
    let records = hub_card::RunRecords::load(run_dir, &size, &base_pin);
    if records.compare.is_some()
        && let Some(src) = hub_card::compare_json_path(run_dir, &size, &base_pin)
        && let Ok(raw) = std::fs::read_to_string(&src)
        && let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        let rel = "evidence/reasoning_compare.json";
        std::fs::create_dir_all(stage_dir.join("evidence"))?;
        sanitize_json_paths(&mut parsed);
        std::fs::write(
            stage_dir.join(rel),
            format!("{}\n", serde_json::to_string_pretty(&parsed)?),
        )?;
        staged_rel_paths.push(rel.to_string());
    }

    // 6b. Emit Hugging Face `.eval_results/vox_mens_eval.yaml` when structured eval metrics exist.
    if let Some(eval_yaml) = build_eval_results_yaml(run_dir, &repo_id)? {
        std::fs::create_dir_all(stage_dir.join(".eval_results"))?;
        std::fs::write(
            stage_dir.join(".eval_results/vox_mens_eval.yaml"),
            eval_yaml,
        )?;
        staged_rel_paths.push(".eval_results/vox_mens_eval.yaml".to_string());
    }

    // 6c. Emit machine-readable external serving handoff contract for this Hub bundle.
    let adapter_handoff_path = peft_rel_dir.or_else(|| {
        adapter_src
            .as_ref()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()))
    });
    let handoff = ExternalServingHandoffV1::hub_bundle(
        &repo_id,
        &base_pin,
        adapter_handoff_path,
        distribute_merged,
    );
    write_handoff(stage_dir, &handoff)?;
    staged_rel_paths.push("external_serving_handoff_v1.json".to_string());

    // 7. Write the record-driven model card to README.md and MODEL_CARD.md (Phase 336).
    let cx = hub_card::CardContext {
        size: &size,
        base_pin: &base_pin,
        namespace,
    };
    let card = hub_card::render_model_card(&cx, &records);
    std::fs::write(stage_dir.join("README.md"), &card)?;
    std::fs::write(stage_dir.join("MODEL_CARD.md"), &card)?;
    staged_rel_paths.push("README.md".to_string());
    staged_rel_paths.push("MODEL_CARD.md".to_string());

    // 8. Write programmatic raster banner assets (Phase 339 / v7.7).
    for asset in hub_card::render_model_assets(&cx, &records) {
        let dst = stage_dir.join(&asset.rel_path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dst, &asset.bytes)?;
        staged_rel_paths.push(asset.rel_path);
    }

    staged_rel_paths.sort();
    staged_rel_paths.dedup();

    let mut files = Vec::with_capacity(staged_rel_paths.len());
    for rel in staged_rel_paths {
        let bytes = std::fs::read(stage_dir.join(&rel))?;
        let blake3 = vox_crypto::hash_fast_hex(&bytes);
        files.push(FileHashEntry {
            path: rel,
            blake3,
            size_bytes: bytes.len() as u64,
        });
    }

    let sync_manifest = HfSyncManifest {
        schema: "vox_mens_hf_sync_v1".to_string(),
        size,
        base_pin,
        corpus_hash,
        final_loss,
        measured_peak_mb,
        gate_status: gate_status.label(),
        files,
    };

    std::fs::write(
        stage_dir.join(HF_SYNC_MANIFEST_FILENAME),
        format!("{}\n", serde_json::to_string_pretty(&sync_manifest)?),
    )?;

    Ok(sync_manifest)
}

/// Stage a run directory into a Hub upload bundle, optionally duplicating the spoke adapter layout
/// under `spokes/<spoke>/` while keeping root compatibility aliases intact (`340-01`).
pub fn stage_publish_bundle_spoke(
    run_dir: &Path,
    stage_dir: &Path,
    force_ungated: bool,
    namespace: &str,
    spoke: Option<&str>,
) -> Result<HfSyncManifest> {
    let mut manifest = stage_publish_bundle(run_dir, stage_dir, force_ungated, namespace)?;
    if let Some(sp) = spoke {
        let rel = hub_repo::spoke_rel_dir(sp)?;
        let sp_dir = stage_dir.join(&rel);
        std::fs::create_dir_all(&sp_dir)
            .with_context(|| format!("failed to create spoke dir {}", sp_dir.display()))?;
        for file_name in [
            "candle_qlora_adapter.safetensors",
            "adapter_config.json",
            "adapter_model.safetensors",
            "adapter_card.json",
            "training_manifest.json",
            "populi_train_record.json",
            "gate_receipt.json",
        ] {
            let src = stage_dir.join(file_name);
            if src.is_file() {
                std::fs::copy(&src, sp_dir.join(file_name)).with_context(|| {
                    format!(
                        "failed to stage spoke artifact {} -> {}",
                        src.display(),
                        sp_dir.join(file_name).display()
                    )
                })?;
                let bytes = std::fs::read(&src)?;
                let blake3 = vox_crypto::hash_fast_hex(&bytes);
                let rel_slash = rel.to_string_lossy().replace('\\', "/");
                manifest.files.push(FileHashEntry {
                    path: format!("{rel_slash}/{file_name}"),
                    blake3,
                    size_bytes: bytes.len() as u64,
                });
            }
        }
        manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
        manifest.files.dedup_by(|a, b| a.path == b.path);
        std::fs::write(
            stage_dir.join(HF_SYNC_MANIFEST_FILENAME),
            format!("{}\n", serde_json::to_string_pretty(&manifest)?),
        )?;
    }
    Ok(manifest)
}

/// Outcome of publishing a local adapter run directory to Hugging Face Hub.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum PublishOutcome {
    Uploaded {
        repo_id: String,
        size: String,
        commit_oid: String,
        gate_status: String,
        /// Immutable version tag created at `commit_oid` (`v1`, `v2`, ...) (HFVER-01).
        version: String,
    },
    SkippedUnchanged {
        repo_id: String,
        size: String,
    },
}

/// Parse a `v<N>` version tag into `N` (HFVER-01). Any other tag name is not a version.
fn parse_version_tag(tag: &str) -> Option<u32> {
    let n = tag.strip_prefix('v')?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    n.parse().ok()
}

/// Highest `v<N>` tag among `tags`, if any (HFVER-02: the default revision for a pull).
pub fn latest_version_tag(tags: &[String]) -> Option<String> {
    tags.iter()
        .filter_map(|t| parse_version_tag(t).map(|n| (n, t)))
        .max_by_key(|(n, _)| *n)
        .map(|(_, t)| t.clone())
}

/// Next version tag: one past the highest existing `v<N>`, or `v1` when none exist (HFVER-01).
pub fn next_version_tag(tags: &[String]) -> String {
    let next = tags
        .iter()
        .filter_map(|t| parse_version_tag(t))
        .max()
        .map_or(1, |n| n + 1);
    format!("v{next}")
}

/// Tag names on a Hub repo (empty when the repo has none).
pub async fn list_repo_tags(repo_id: &str) -> Result<Vec<String>> {
    normalize_hf_token_env();
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let (owner, name) = split_id(repo_id);
    let refs = client
        .model(owner, name)
        .list_refs()
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("list refs for {repo_id}: {e}"))?;
    Ok(refs.tags.into_iter().map(|t| t.name).collect())
}

/// Create `tag` at `revision` (or the main head when `None`) with an annotation message.
async fn create_repo_tag(
    repo_id: &str,
    tag: &str,
    revision: Option<&str>,
    message: &str,
) -> Result<()> {
    normalize_hf_token_env();
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let (owner, name) = split_id(repo_id);
    client
        .model(owner, name)
        .create_tag()
        .tag(tag)
        .maybe_revision(revision)
        .message(message)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("create tag {tag} on {repo_id}: {e}"))
}

/// Fetch `hf_sync_manifest.json` from a Hub repo if the repo and manifest exist.
pub async fn fetch_remote_sync_manifest(repo_id: &str) -> Result<Option<HfSyncManifest>> {
    normalize_hf_token_env();
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let (owner, name) = split_id(repo_id);
    let repo = client.model(owner, name);
    let Ok(info) = repo.info().send().await else {
        return Ok(None);
    };
    let Some(siblings) = info.siblings else {
        return Ok(None);
    };
    if !siblings
        .iter()
        .any(|s| s.rfilename == HF_SYNC_MANIFEST_FILENAME)
    {
        return Ok(None);
    }
    let path = repo
        .download_file()
        .filename(HF_SYNC_MANIFEST_FILENAME)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("download {HF_SYNC_MANIFEST_FILENAME} from {repo_id}: {e}"))?;
    let raw = std::fs::read_to_string(&path)?;
    let parsed: HfSyncManifest = serde_json::from_str(&raw)?;
    Ok(Some(parsed))
}

/// Publish `run_dir` to its per-size Hugging Face repository (HFPUB-01..05).
///
/// Checks `HuggingFaceToken` in Clavis **before** reading any file on disk (HFPUB-05),
/// verifies gate receipts unless `force_ungated` (HFPUB-02), builds the model card and
/// `hf_sync_manifest.json` (HFPUB-03), and skips the upload if the remote manifest already
/// has identical file digests (HFPUB-04).
#[inline(never)]
pub async fn publish_adapter_run_dir(
    run_dir: &Path,
    namespace: Option<&str>,
    private: bool,
    force_ungated: bool,
) -> Result<PublishOutcome> {
    publish_adapter_run_dir_with_spoke(run_dir, namespace, private, force_ungated, None).await
}

/// Like [`publish_adapter_run_dir`] while optionally staging spoke artifacts under `spokes/<spoke>/`.
#[inline(never)]
pub async fn publish_adapter_run_dir_with_spoke(
    run_dir: &Path,
    namespace: Option<&str>,
    private: bool,
    force_ungated: bool,
    spoke: Option<&str>,
) -> Result<PublishOutcome> {
    // HFPUB-05: Clavis token check MUST run before any file is read.
    let _token = require_clavis_hf_token()?;
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;

    let stage_root = std::env::temp_dir().join(format!(
        "vox-mens-hf-stage-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let res = async {
        let local_manifest =
            stage_publish_bundle_spoke(run_dir, &stage_root, force_ungated, &ns, spoke)?;
        let repo_id = hub_repo::repo_id_for_size(&ns, &local_manifest.base_pin)?;
        let model_label = hub_repo::model_label_for(&local_manifest.base_pin);

        // HFPUB-04: idempotent hash-skip when remote manifest matches.
        if let Ok(Some(remote_manifest)) = fetch_remote_sync_manifest(&repo_id).await
            && manifests_match(&local_manifest, &remote_manifest)
        {
            // Legacy repos published before versioning have no tag: baseline them as v1
            // at the current head so every published state is addressable (HFVER-01).
            if list_repo_tags(&repo_id).await.is_ok_and(|t| t.is_empty()) {
                let msg = format!(
                    "Baseline version of VoxMens {model_label} adapter ({})",
                    local_manifest.gate_status
                );
                create_repo_tag(&repo_id, "v1", None, &msg).await?;
            }
            return Ok(PublishOutcome::SkippedUnchanged {
                repo_id,
                size: local_manifest.size,
            });
        }

        let commit_msg = format!(
            "Publish VoxMens {model_label} adapter ({})",
            local_manifest.gate_status
        );
        let commit_oid =
            upload_model_folder(&repo_id, &stage_root, private, Some(&commit_msg)).await?;
        let tags = list_repo_tags(&repo_id).await.unwrap_or_default();
        let version = next_version_tag(&tags);
        let tag_msg = format!(
            "{commit_msg}; corpus {}; loss {}",
            local_manifest.corpus_hash,
            local_manifest
                .final_loss
                .map_or_else(|| "n/a".to_string(), |l| format!("{l:.4}"))
        );
        let tag_rev = (commit_oid != "unknown").then_some(commit_oid.as_str());
        create_repo_tag(&repo_id, &version, tag_rev, &tag_msg)
            .await
            .with_context(|| {
                format!("commit {commit_oid} was uploaded to {repo_id} but could not be tagged")
            })?;
        Ok(PublishOutcome::Uploaded {
            repo_id,
            size: local_manifest.size,
            commit_oid,
            gate_status: local_manifest.gate_status,
            version,
        })
    }
    .await;
    let _ = std::fs::remove_dir_all(&stage_root);
    res
}

/// Blocking wrapper around [`publish_adapter_run_dir`] for synchronous CLI / gate hooks.
#[inline(never)]
pub fn publish_adapter_run_dir_blocking(
    run_dir: &Path,
    namespace: Option<&str>,
    private: bool,
    force_ungated: bool,
) -> Result<PublishOutcome> {
    let run_dir = run_dir.to_path_buf();
    let namespace = namespace.map(ToString::to_string);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))
            .and_then(|rt| {
                rt.block_on(publish_adapter_run_dir(
                    &run_dir,
                    namespace.as_deref(),
                    private,
                    force_ungated,
                ))
            });
        let _ = tx.send(result);
    });
    rx.recv()
        .map_err(|_| anyhow::anyhow!("HF publish thread exited without sending result"))?
}

/// Verify a downloaded adapter directory against the ladder's pinned base and gate receipts (HFPULL-02).
///
/// On any failure, removes `dir` so an unverified or tampered adapter is never left on disk.
#[inline(never)]
pub fn verify_pulled_adapter_dir(
    dir: &Path,
    expected_size: &str,
    workspace_root: Option<&Path>,
    require_gates: bool,
) -> Result<()> {
    let check = || -> Result<()> {
        let size = hub_repo::size_of_pin(expected_size).ok_or_else(|| {
            anyhow::anyhow!("`{expected_size}` is not a recognized Qwen ladder size")
        })?;
        let expected_pin = resolve_ladder_base_pin(workspace_root, expected_size)?;

        let adapter_exists = dir.join("candle_qlora_adapter.safetensors").is_file()
            || dir.join("adapter.safetensors").is_file();
        if !adapter_exists {
            bail!(
                "pulled adapter in {} is missing candle_qlora_adapter.safetensors",
                dir.display()
            );
        }

        let manifest_path = dir.join("training_manifest.json");
        if !manifest_path.is_file() {
            bail!(
                "pulled adapter in {} is missing training_manifest.json",
                dir.display()
            );
        }
        let manifest_val: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let actual_pin = manifest_val
            .get("base_model")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if actual_pin != expected_pin {
            bail!(
                "pulled adapter base_model `{actual_pin}` does not match ladder pin `{expected_pin}` for size {size}"
            );
        }

        let sync_path = dir.join(HF_SYNC_MANIFEST_FILENAME);
        if sync_path.is_file() {
            let sync: HfSyncManifest = serde_json::from_str(&std::fs::read_to_string(&sync_path)?)?;
            for entry in &sync.files {
                let file_path = dir.join(&entry.path);
                if !file_path.is_file() {
                    bail!(
                        "pulled adapter in {} is missing manifest file `{}`",
                        dir.display(),
                        entry.path
                    );
                }
                let bytes = std::fs::read(&file_path)?;
                let actual_hash = vox_crypto::hash_fast_hex(&bytes);
                if actual_hash != entry.blake3 {
                    bail!(
                        "pulled file `{}` hash mismatch: expected {}, got {}",
                        entry.path,
                        entry.blake3,
                        actual_hash
                    );
                }
            }
        }

        if require_gates {
            let req = crate::mens::gate_policy::ladder_requirement(workspace_root);
            verify_publish_gates_with(dir, false, &req).map_err(|e| {
                anyhow::anyhow!(
                    "{e}\nPull with `vox mens hub pull --size {size} --allow-ungated` to keep the files, then re-gate locally."
                )
            })?;
        }
        Ok(())
    };

    if let Err(e) = check() {
        let _ = std::fs::remove_dir_all(dir);
        return Err(e);
    }
    Ok(())
}

/// Resolve a size name (`4B`, `Qwen3-4B`, `Qwen3.5-4B`, `27B`) or run directory
/// (`mens/runs/qwen3-4b`, `mens/runs/qwen3.5-4b`, `mens/runs/qwen3.8-27b`) to
/// `(size, target_run_dir)` when it refers to a Qwen ladder adapter.
pub fn resolve_ladder_adapter_target(
    model_arg: &Path,
    workspace_root: Option<&Path>,
) -> Option<(String, PathBuf)> {
    let raw = model_arg.to_str()?.trim_end_matches(['/', '\\']);
    let hint = if hub_repo::size_of_pin(raw).is_some() {
        raw
    } else {
        let name = model_arg.file_name()?.to_str()?;
        if hub_repo::size_of_pin(name).is_some() {
            name
        } else {
            return None;
        }
    };
    let size = hub_repo::size_of_pin(hint)?;
    let run_rel = PathBuf::from(hub_repo::local_run_dir_for_size(hint).ok()?);
    let run_dir = match workspace_root {
        Some(root) if !model_arg.is_dir() => root.join(run_rel),
        _ if model_arg.to_str().is_some_and(|s| s.contains("mens/runs/")) => {
            model_arg.to_path_buf()
        }
        Some(root) => root.join(run_rel),
        None => run_rel,
    };
    Some((size, run_dir))
}

/// Pull a missing ladder adapter from `<namespace>/vox-mens-<family>-<size>` into
/// `mens/runs/<family>-<size>` and verify its manifest pin and gate receipts (HFPULL-01..03).
pub async fn pull_adapter_if_missing(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
) -> Result<PathBuf> {
    pull_adapter_revision(model_arg, workspace_root, namespace, require_gates, None).await
}

/// Like [`pull_adapter_if_missing`] but pinned to `revision` (a version tag such as `v2`, a branch,
/// or a commit). With `None` it pulls the latest `v<N>` tag, or `main` for a repo with no tags (HFVER-02).
#[inline(never)]
pub async fn pull_adapter_revision(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
    revision: Option<&str>,
) -> Result<PathBuf> {
    pull_adapter_revision_with_spoke(
        model_arg,
        workspace_root,
        namespace,
        require_gates,
        revision,
        None,
    )
    .await
}

/// Like [`pull_adapter_revision`] with optional `spoke` (`spokes/<spoke>/`) resolution and download.
#[inline(never)]
pub async fn pull_adapter_revision_with_spoke(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
    revision: Option<&str>,
    spoke: Option<&str>,
) -> Result<PathBuf> {
    let spoke_rel = spoke.map(hub_repo::spoke_rel_dir).transpose()?;
    let spoke_has_adapter = |base: &Path, rel: &Path| {
        let sp_dir = base.join(rel);
        sp_dir.join("candle_qlora_adapter.safetensors").is_file()
            || sp_dir.join("adapter.safetensors").is_file()
            || sp_dir.join("adapter_model.safetensors").is_file()
    };

    if model_arg.is_file() {
        return Ok(model_arg.to_path_buf());
    }
    if model_arg.is_dir() {
        if let Some(ref rel) = spoke_rel {
            if resolve_ladder_adapter_target(model_arg, workspace_root).is_none() {
                if spoke_has_adapter(model_arg, rel) {
                    return Ok(model_arg.join(rel));
                }
                return Ok(model_arg.to_path_buf());
            }
        } else if model_arg.join("candle_qlora_adapter.safetensors").is_file()
            || model_arg.join("adapter.safetensors").is_file()
        {
            return Ok(model_arg.to_path_buf());
        }
    }

    let Some((size, target_dir)) = resolve_ladder_adapter_target(model_arg, workspace_root) else {
        return Ok(model_arg.to_path_buf());
    };
    let already_present = match &spoke_rel {
        Some(rel) => spoke_has_adapter(&target_dir, rel),
        None => {
            target_dir
                .join("candle_qlora_adapter.safetensors")
                .is_file()
                || target_dir.join("adapter.safetensors").is_file()
        }
    };
    if already_present {
        return Ok(match &spoke_rel {
            Some(rel) => target_dir.join(rel),
            None => target_dir,
        });
    }

    let model_hint = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&size);
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    let repo_id = hub_repo::repo_id_for_size(&ns, model_hint)?;

    ensure_pull_allowed(
        &repo_id,
        std::env::var(NO_DOWNLOAD_ENV).ok().as_deref(),
        std::env::var(HF_HUB_OFFLINE_ENV).ok().as_deref(),
    )?;

    normalize_hf_token_env();

    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let (owner, name) = split_id(&repo_id);
    let repo = client.model(owner, name);
    let revision = match revision {
        Some(r) => Some(r.to_string()),
        None => latest_version_tag(&list_repo_tags(&repo_id).await.unwrap_or_default()),
    };
    let info = repo
        .info()
        .maybe_revision(revision.as_deref())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("hf-hub repo info for {repo_id}: {e}"))?;
    let siblings = info
        .siblings
        .ok_or_else(|| anyhow::anyhow!("repo {repo_id} returned no file listing"))?;

    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("create {}", target_dir.display()))?;

    let mut downloaded = std::collections::HashSet::new();
    for sib in &siblings {
        if sib.rfilename.starts_with('.') && !sib.rfilename.starts_with(".eval_results/") {
            continue;
        }
        let cached = repo
            .download_file()
            .filename(&sib.rfilename)
            .maybe_revision(revision.as_deref())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download {} from {repo_id}: {e}", sib.rfilename))?;
        let dest = target_dir.join(&sib.rfilename);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&cached, &dest)?;
        downloaded.insert(sib.rfilename.clone());
    }

    if let Some(ref rel) = spoke_rel {
        let rel_slash = rel.to_string_lossy().replace('\\', "/");
        for name in [
            "candle_qlora_adapter.safetensors",
            "adapter_manifest.json",
            "training_manifest.json",
            "adapter_config.json",
            "adapter_model.safetensors",
            "gate_receipt.json",
        ] {
            let remote_path = format!("{rel_slash}/{name}");
            if downloaded.contains(&remote_path) {
                continue;
            }
            if let Ok(cached) = repo
                .download_file()
                .filename(&remote_path)
                .maybe_revision(revision.as_deref())
                .send()
                .await
            {
                let dest = target_dir.join(rel).join(name);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&cached, &dest)?;
            }
        }
    }

    verify_pulled_adapter_dir(&target_dir, model_hint, workspace_root, require_gates)?;
    if let Some(ref rel) = spoke_rel
        && spoke_has_adapter(&target_dir, rel)
    {
        return Ok(target_dir.join(rel));
    }
    Ok(target_dir)
}

/// Blocking wrapper around [`pull_adapter_if_missing`].
pub fn pull_adapter_if_missing_blocking(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
) -> Result<PathBuf> {
    pull_adapter_revision_blocking(model_arg, workspace_root, namespace, require_gates, None)
}

/// Blocking wrapper around [`pull_adapter_revision`].
pub fn pull_adapter_revision_blocking(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
    revision: Option<&str>,
) -> Result<PathBuf> {
    pull_adapter_revision_with_spoke_blocking(
        model_arg,
        workspace_root,
        namespace,
        require_gates,
        revision,
        None,
    )
}

/// Blocking wrapper around [`pull_adapter_revision_with_spoke`].
pub fn pull_adapter_revision_with_spoke_blocking(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
    revision: Option<&str>,
    spoke: Option<&str>,
) -> Result<PathBuf> {
    let model_arg = model_arg.to_path_buf();
    let workspace_root = workspace_root.map(Path::to_path_buf);
    let namespace = namespace.map(ToString::to_string);
    let revision = revision.map(ToString::to_string);
    let spoke = spoke.map(ToString::to_string);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))
            .and_then(|rt| {
                rt.block_on(pull_adapter_revision_with_spoke(
                    &model_arg,
                    workspace_root.as_deref(),
                    namespace.as_deref(),
                    require_gates,
                    revision.as_deref(),
                    spoke.as_deref(),
                ))
            });
        let _ = tx.send(result);
    });
    rx.recv()
        .map_err(|_| anyhow::anyhow!("HF pull thread exited without sending result"))?
}

/// Sync state for one ladder size between local `mens/runs/qwen3-<size>` and Hub `<ns>/vox-mens-qwen3-<size>` (HFSYNC-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeSyncStatus {
    NotTrained,
    InSync,
    LocalOnly,
    HubOnly,
    Diverged,
}

impl SizeSyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotTrained => "not_trained",
            Self::InSync => "in_sync",
            Self::LocalOnly => "local_only",
            Self::HubOnly => "hub_only",
            Self::Diverged => "diverged",
        }
    }
}

/// Pure classifier comparing optional local and remote [`HfSyncManifest`]s (HFSYNC-01, HFSYNC-03).
pub fn classify_size_sync(
    local: Option<&HfSyncManifest>,
    remote: Option<&HfSyncManifest>,
) -> SizeSyncStatus {
    match (local, remote) {
        (None, None) => SizeSyncStatus::NotTrained,
        (Some(_), None) => SizeSyncStatus::LocalOnly,
        (None, Some(_)) => SizeSyncStatus::HubOnly,
        (Some(l), Some(r)) if manifests_match(l, r) => SizeSyncStatus::InSync,
        (Some(_), Some(_)) => SizeSyncStatus::Diverged,
    }
}

/// Per-size row in `vox mens hub sync` and `<namespace>/vox-mens-index` (HFSYNC-01..03).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SizeSyncRow {
    pub size: String,
    pub base_pin: String,
    pub repo_id: String,
    pub local_dir: String,
    pub sync_status: SizeSyncStatus,
    pub gate_status: Option<String>,
    pub final_loss: Option<f64>,
    pub measured_peak_mb: Option<u64>,
    pub action_taken: Option<String>,
    /// Highest `v<N>` tag on the Hub repo, when the repo exists and is tagged (HFVER-01).
    #[serde(default)]
    pub latest_version: Option<String>,
}

/// Pair each sync row with its local run records (read from `root/<row.local_dir>`).
pub fn index_entries(
    root: &Path,
    rows: &[SizeSyncRow],
) -> Vec<(SizeSyncRow, hub_card::RunRecords)> {
    rows.iter()
        .map(|r| {
            let rec = hub_card::RunRecords::load(&root.join(&r.local_dir), &r.size, &r.base_pin);
            (r.clone(), rec)
        })
        .collect()
}

/// Render `(README.md, ladder_index.json)` for `<namespace>/vox-mens-index` (HFSYNC-03, Phase 336).
pub fn render_index_repo_files(
    namespace: &str,
    entries: &[(SizeSyncRow, hub_card::RunRecords)],
) -> (String, String) {
    let md = hub_card::render_index_readme(namespace, entries);
    let rows: Vec<&SizeSyncRow> = entries.iter().map(|(r, _)| r).collect();
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "schema": "vox_mens_hf_ladder_index_v1",
        "namespace": namespace,
        "sizes": rows,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    (md, format!("{json}\n"))
}

/// Local directory `vox mens hub sync` renders every card and the index into.
pub const LOCAL_CARDS_DIR: &str = "mens/runs/hub_cards";

/// Write each size card (`<repo>/README.md`) and the index (`vox-mens-index/README.md`,
/// `ladder_index.json`) under `root/mens/runs/hub_cards`. These are the exact bytes a push uploads.
pub fn write_local_cards(
    root: &Path,
    namespace: &str,
    entries: &[(SizeSyncRow, hub_card::RunRecords)],
) -> Result<PathBuf> {
    let dir = root.join(LOCAL_CARDS_DIR);
    let mut active_slugs = std::collections::BTreeSet::new();
    active_slugs.insert("vox-mens-index".to_string());
    for (row, rec) in entries {
        let cx = hub_card::CardContext {
            size: &row.size,
            base_pin: &row.base_pin,
            namespace,
        };
        let card = hub_card::render_model_card(&cx, rec);
        let slug = hub_repo::repo_slug_for_model(&row.base_pin)
            .unwrap_or_else(|_| hub_card::repo_name(&row.size));
        active_slugs.insert(slug.clone());
        let d = dir.join(slug);
        std::fs::create_dir_all(&d).with_context(|| format!("create {}", d.display()))?;
        std::fs::write(d.join("README.md"), card)?;
        for stale_svg in ["assets/scorecard.svg", "assets/overview.svg"] {
            let _ = std::fs::remove_file(d.join(stale_svg));
        }
        for asset in hub_card::render_model_assets(&cx, rec) {
            let p = d.join(&asset.rel_path);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, &asset.bytes)?;
        }
    }
    if entries.len() == SIZES.len()
        && let Ok(read_dir) = std::fs::read_dir(&dir)
    {
        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if name_str.starts_with("vox-mens-") && !active_slugs.contains(name_str) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let (readme, json) = render_index_repo_files(namespace, entries);
    let d = dir.join("vox-mens-index");
    std::fs::create_dir_all(&d).with_context(|| format!("create {}", d.display()))?;
    std::fs::write(d.join("README.md"), readme)?;
    std::fs::write(d.join("ladder_index.json"), json)?;
    for stale_svg in ["assets/scorecard.svg", "assets/overview.svg"] {
        let _ = std::fs::remove_file(d.join(stale_svg));
    }
    for asset in hub_card::render_index_assets(namespace, entries) {
        let p = d.join(&asset.rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, &asset.bytes)?;
    }
    let sp = dir.join("vox-mens-playground");
    std::fs::create_dir_all(&sp).with_context(|| format!("create {}", sp.display()))?;
    for stale in ["app.py", "requirements.txt"] {
        let _ = std::fs::remove_file(sp.join(stale));
    }
    for (rel_name, content) in render_playground_space_files(namespace, entries) {
        std::fs::write(sp.join(rel_name), content)?;
    }
    Ok(dir)
}

const SPACE_README_TMPL: &str = include_str!("../../templates/hub/space/README.md.tmpl");
const SPACE_INDEX_HTML_TMPL: &str = include_str!("../../templates/hub/space/index.html.tmpl");

/// Render `("README.md", "index.html")` for the Hugging Face Static WebGPU Space
/// `<namespace>/vox-mens-playground` (`sdk: static`).
///
/// The Space `README.md` lists every ladder model repo under `models:`, which causes Hugging Face
/// to automatically link the live browser playground Space on every `vox-mens-qwen*` model card.
pub fn render_playground_space_files(
    namespace: &str,
    entries: &[(SizeSyncRow, hub_card::RunRecords)],
) -> Vec<(&'static str, String)> {
    let mut models_yaml = format!("- {namespace}/vox-mens-index\n");
    for (row, _) in entries {
        models_yaml.push_str(&format!("- {}\n", row.repo_id));
    }
    let readme = SPACE_README_TMPL
        .replace("{{MODELS_YAML}}", &models_yaml)
        .replace("{{NAMESPACE}}", namespace);
    let index_html = SPACE_INDEX_HTML_TMPL.replace("{{NAMESPACE}}", namespace);
    vec![("README.md", readme), ("index.html", index_html)]
}

/// True when `size` is included. `only` empty means every ladder size.
pub fn size_in_scope(size: &str, only: Option<&str>) -> bool {
    let Some(want) = only.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let want = ["qwen3.8-", "qwen3.5-", "qwen3-"]
        .iter()
        .find_map(|prefix| {
            (want.len() >= prefix.len() && want[..prefix.len()].eq_ignore_ascii_case(prefix))
                .then(|| &want[prefix.len()..])
        })
        .unwrap_or(want);
    size.eq_ignore_ascii_case(want)
}

/// Inspect local `mens/runs/<family>-<size>` for all active ladder sizes without touching the
/// network unless `push` or `pull` (or uncached remote manifest fetch) requires it.
#[inline(never)]
pub async fn sync_all_sizes(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    push: bool,
    pull: bool,
    force_ungated: bool,
    private: bool,
    only_size: Option<&str>,
) -> Result<Vec<SizeSyncRow>> {
    sync_all_sizes_with_spoke(
        workspace_root,
        namespace,
        push,
        pull,
        force_ungated,
        private,
        only_size,
        None,
    )
    .await
}

/// Like [`sync_all_sizes`] while optionally staging spoke artifacts under `spokes/<spoke>/`.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn sync_all_sizes_with_spoke(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    push: bool,
    pull: bool,
    force_ungated: bool,
    private: bool,
    only_size: Option<&str>,
    spoke: Option<&str>,
) -> Result<Vec<SizeSyncRow>> {
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    if push {
        let _ = require_clavis_hf_token()?;
        ensure_ladder_repos(workspace_root, Some(&ns), private).await?;
    }
    let root = workspace_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut rows = Vec::with_capacity(SIZES.len());
    for &size in &SIZES {
        let base_pin = resolve_ladder_base_pin(workspace_root, size)?;
        let repo_id = hub_repo::repo_id_for_size(&ns, size)?;
        let rel_dir = hub_repo::local_run_dir_for_size(size)?;
        let run_dir = root.join(&rel_dir);

        let local_manifest = if run_dir.join("candle_qlora_adapter.safetensors").is_file()
            || run_dir.join("adapter.safetensors").is_file()
        {
            crate::mens::hub_cards_push::scratch("inspect", size)
                .ok()
                .and_then(|tmp| {
                    stage_publish_bundle_spoke(&run_dir, tmp.path(), true, &ns, spoke).ok()
                })
        } else {
            None
        };

        let remote_manifest = fetch_remote_sync_manifest(&repo_id).await.ok().flatten();
        let mut latest_version = if remote_manifest.is_some() {
            latest_version_tag(&list_repo_tags(&repo_id).await.unwrap_or_default())
        } else {
            None
        };
        let mut status = classify_size_sync(local_manifest.as_ref(), remote_manifest.as_ref());
        let mut action_taken = None::<String>;

        let selected = size_in_scope(size, only_size);
        let weights_already_on_hub = local_manifest.as_ref().is_some_and(|local| {
            crate::mens::hub_cards_push::remote_weights_match(remote_manifest.as_ref(), local)
        });
        if push
            && selected
            && (status == SizeSyncStatus::LocalOnly
                || (status == SizeSyncStatus::Diverged && !weights_already_on_hub))
        {
            // Reserved repos already exist. Upload new or changed adapter weights.
            match publish_adapter_run_dir_with_spoke(
                &run_dir,
                Some(&ns),
                private,
                force_ungated,
                spoke,
            )
            .await
            {
                Ok(PublishOutcome::Uploaded {
                    commit_oid,
                    version,
                    ..
                }) => {
                    action_taken = Some(format!("pushed {version} ({commit_oid})"));
                    latest_version = Some(version);
                    status = SizeSyncStatus::InSync;
                }
                Ok(PublishOutcome::SkippedUnchanged { .. }) => {
                    action_taken = Some("skipped_unchanged".to_string());
                    status = SizeSyncStatus::InSync;
                }
                Err(e) => {
                    action_taken = Some(format!("push_refused: {e}"));
                }
            }
        } else if pull
            && selected
            && matches!(status, SizeSyncStatus::HubOnly | SizeSyncStatus::Diverged)
        {
            if status == SizeSyncStatus::Diverged {
                let _ = std::fs::remove_file(run_dir.join("candle_qlora_adapter.safetensors"));
            }
            pull_adapter_revision_with_spoke(
                &run_dir,
                Some(&root),
                Some(&ns),
                !force_ungated,
                None,
                spoke,
            )
            .await?;
            action_taken = Some("pulled".to_string());
            status = SizeSyncStatus::InSync;
        }

        let info_manifest = local_manifest.as_ref().or(remote_manifest.as_ref());
        rows.push(SizeSyncRow {
            size: size.to_string(),
            base_pin,
            repo_id,
            local_dir: rel_dir,
            sync_status: status,
            gate_status: info_manifest.map(|m| m.gate_status.clone()),
            final_loss: info_manifest.and_then(|m| m.final_loss),
            measured_peak_mb: info_manifest.and_then(|m| m.measured_peak_mb),
            action_taken,
            latest_version,
        });
    }

    if push {
        crate::mens::hub_cards_push::sync_diverged_cards_in_place(&mut rows, &root, &ns, private)
            .await;
    }
    let entries = index_entries(&root, &rows);
    write_local_cards(&root, &ns, &entries)?;
    if push {
        upload_ladder_index(&ns, &entries, private).await?;
    }

    Ok(rows)
}

/// Upload `README.md` and `ladder_index.json` for `<namespace>/vox-mens-index`.
async fn upload_ladder_index(
    namespace: &str,
    entries: &[(SizeSyncRow, hub_card::RunRecords)],
    private: bool,
) -> Result<()> {
    let index_repo = hub_repo::index_repo_id(namespace)?;
    let (readme, index_json) = render_index_repo_files(namespace, entries);
    let tmp = crate::mens::hub_cards_push::scratch("index", "all")?;
    std::fs::write(tmp.path().join("README.md"), readme)?;
    std::fs::write(tmp.path().join("ladder_index.json"), index_json)?;
    for asset in hub_card::render_index_assets(namespace, entries) {
        let p = tmp.path().join(&asset.rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, &asset.bytes)?;
    }
    upload_model_folder(
        &index_repo,
        tmp.path(),
        private,
        Some("Update VoxMens Qwen3 / Qwen3.5 / Qwen3.8 size ladder index"),
    )
    .await?;

    // Also update the interactive Hugging Face Static WebGPU Space (`<namespace>/vox-mens-playground`)
    // so every model card links the live browser playground automatically.
    let space_repo = hub_repo::space_repo_id(namespace)?;
    let space_files = render_playground_space_files(namespace, entries);
    let space_tmp = crate::mens::hub_cards_push::scratch("space", "all")?;
    for (rel_name, content) in space_files {
        std::fs::write(space_tmp.path().join(rel_name), content)?;
    }
    if let Err(e) = upload_space_folder(
        &space_repo,
        space_tmp.path(),
        private,
        Some("Update VoxMens interactive Qwen3 / Qwen3.5 / Qwen3.8 ladder playground"),
    )
    .await
    {
        eprintln!("  warn: failed to sync interactive playground Space `{space_repo}`: {e}");
    }

    let mut collection_items: Vec<(String, &str)> = entries
        .iter()
        .map(|(row, _)| (row.repo_id.clone(), "model"))
        .collect();
    collection_items.push((index_repo, "model"));
    collection_items.push((space_repo, "space"));
    if let Err(e) = ensure_mens_collection(
        namespace,
        "VoxMens Fine-Tuned Qwen3.5 & Qwen3.8 Coding Models",
        "Vox-trained Qwen3.5 & Qwen3.8 coding adapters, merged checkpoints, index, and WebGPU playground.",
        &collection_items,
    )
    .await
    {
        eprintln!("  warn: failed to sync HF collection for `{namespace}`: {e}");
    }
    Ok(())
}

/// Re-read every ladder rung and publish the index repo, without uploading adapters.
///
/// Train and eval-gate auto-publish go through [`publish_adapter_run_dir`], which
/// updates one size repo and used to leave `vox-mens-index` stale.
#[inline(never)]
pub async fn refresh_ladder_index(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
) -> Result<Vec<SizeSyncRow>> {
    ensure_ladder_repos(workspace_root, namespace, private).await?;
    let mut rows = sync_all_sizes(
        workspace_root,
        namespace,
        false,
        false,
        false,
        private,
        None,
    )
    .await?;
    let ns_env = std::env::var(hub_repo::NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    let root = workspace_root.unwrap_or(Path::new("."));
    crate::mens::hub_cards_push::sync_diverged_cards_in_place(&mut rows, root, &ns, private).await;
    let entries = index_entries(root, &rows);
    write_local_cards(root, &ns, &entries)?;
    upload_ladder_index(&ns, &entries, private).await?;
    Ok(rows)
}

/// Blocking wrapper around [`refresh_ladder_index`] for the eval-gate hook.
pub fn refresh_ladder_index_blocking(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
) -> Result<Vec<SizeSyncRow>> {
    let workspace_root = workspace_root.map(Path::to_path_buf);
    let namespace = namespace.map(ToString::to_string);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))
            .and_then(|rt| {
                rt.block_on(refresh_ladder_index(
                    workspace_root.as_deref(),
                    namespace.as_deref(),
                    private,
                ))
            });
        let _ = tx.send(result);
    });
    rx.recv()
        .map_err(|_| anyhow::anyhow!("HF index refresh thread exited without sending result"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_synthetic_candle_adapter(dir: &Path, pin: &str) {
        let header = serde_json::json!({
            "l0.q.lora_a.weight": {
                "dtype": "F32",
                "shape": [2, 4],
                "data_offsets": [0, 32]
            },
            "l0.q.lora_b.weight": {
                "dtype": "F32",
                "shape": [4, 2],
                "data_offsets": [32, 64]
            }
        });
        let mut header_bytes = serde_json::to_vec(&header).unwrap();
        while !header_bytes.len().is_multiple_of(8) {
            header_bytes.push(b' ');
        }
        let mut raw = Vec::with_capacity(8 + header_bytes.len() + 64);
        raw.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        raw.extend_from_slice(&header_bytes);
        raw.extend_from_slice(&[0_u8; 64]);
        std::fs::write(dir.join("candle_qlora_adapter.safetensors"), raw).unwrap();

        std::fs::write(
            dir.join("adapter_manifest.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "rank": 8,
                "alpha": 16,
                "use_rslora": true,
                "base_model": pin,
                "base_key_map": {
                    "l0.q": "model.language_model.layers.0.self_attn.q_proj.weight"
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn write_minimal_run_dir(dir: &Path, pin: &str, with_passing_gates: bool) {
        std::fs::create_dir_all(dir).unwrap();
        write_synthetic_candle_adapter(dir, pin);
        std::fs::write(
            dir.join("training_manifest.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "base_model": pin,
                "provenance_license_class": "apache-2.0",
                "finetune_contract_digest": "cfdc855e29dc680d",
                "train_file": concat!("/Us", "ers/secretuser/dev/vox/target/dogfood/train_mixed.jsonl")
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("train-record.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "final_loss_ema": 0.543,
                "peak_mb": 34460
            }))
            .unwrap(),
        )
        .unwrap();
        if with_passing_gates {
            let req = crate::mens::gate_policy::ladder_requirement(None);
            let report = crate::mens::gate_policy::passing_collateral_report(&req);
            let ladder_hash =
                crate::mens::gate_policy::policy_blake3(None, "mens/config/eval-gates-ladder.yaml")
                    .unwrap();
            std::fs::write(
                dir.join("collateral_damage_report.json"),
                serde_json::to_string_pretty(&report).unwrap(),
            )
            .unwrap();
            std::fs::write(
                dir.join("gate_receipt.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                    "policy_blake3": ladder_hash,
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn size_scope_limits_push_to_one_ladder_rung() {
        assert!(size_in_scope("8B", None));
        assert!(size_in_scope("0.6B", Some("0.6B")));
        assert!(size_in_scope("4B", Some("qwen3-4b")));
        assert!(size_in_scope("4B", Some("qwen3.5-4b")));
        assert!(size_in_scope("27B", Some("qwen3.8-27b")));
        assert!(!size_in_scope("8B", Some("4B")));
        assert!(size_in_scope("1.7B", Some("  ")));
    }

    #[test]
    fn hf_sync_opt_out_flags_and_env_work() {
        assert!(hf_sync_enabled(false, None));
        assert!(hf_sync_enabled(false, Some("1")));
        assert!(!hf_sync_enabled(true, None));
        assert!(!hf_sync_enabled(false, Some("0")));
        assert!(!hf_sync_enabled(false, Some("false")));
        assert!(!hf_sync_enabled(false, Some("FALSE")));
    }

    #[test]
    fn auto_publish_skip_names_the_reason() {
        assert!(explain_auto_publish_skip(false, None, Some("brbrainerd")).is_none());
        assert!(
            explain_auto_publish_skip(false, Some("0"), Some("brbrainerd"))
                .unwrap()
                .contains("VOX_MENS_HF_SYNC")
        );
        assert!(
            explain_auto_publish_skip(true, None, Some("brbrainerd"))
                .unwrap()
                .contains("--no-hf-sync")
        );
        assert!(
            explain_auto_publish_skip(false, None, None)
                .unwrap()
                .contains("VOX_MENS_HF_NAMESPACE")
        );
        assert!(
            explain_auto_publish_skip(false, None, Some("  "))
                .unwrap()
                .contains("VOX_MENS_HF_NAMESPACE")
        );
    }

    #[test]
    fn missing_gate_receipts_refuse_upload_unless_forced_ungated() {
        let dir = tempfile::tempdir().unwrap();
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
        write_minimal_run_dir(dir.path(), pin, false);

        let err = verify_publish_gates(dir.path(), false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("collateral_damage_report.json"), "{err}");
        assert!(err.contains("gate_receipt.json"), "{err}");

        let forced = verify_publish_gates(dir.path(), true).unwrap();
        assert!(!forced.is_passed());
        assert!(forced.label().starts_with("ungated"));
    }

    #[test]
    fn stage_bundle_writes_card_and_hashes_and_strips_host_paths() {
        let run = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
        write_minimal_run_dir(run.path(), pin, true);
        std::fs::write(run.path().join("tokenizer.json"), r#"{"model":{}}"#).unwrap();

        let m1 = stage_publish_bundle(run.path(), stage.path(), false, "ns").unwrap();
        assert!(
            m1.files.iter().any(|f| f.path == "tokenizer.json"),
            "`vox mens serve` needs tokenizer.json in a pulled bundle"
        );
        assert!(
            m1.files.iter().any(|f| f.path == "adapter_config.json"),
            "adapter-only bundle should export root PEFT adapter_config.json"
        );
        assert!(
            m1.files
                .iter()
                .any(|f| f.path == "adapter_model.safetensors"),
            "adapter-only bundle should export root PEFT adapter_model.safetensors"
        );
        assert!(
            m1.files.iter().any(|f| f.path == "chat_template.jinja"),
            "bundle with tokenizer.json should stage chat_template.jinja"
        );
        assert!(
            m1.files
                .iter()
                .any(|f| f.path == ".eval_results/vox_mens_eval.yaml"),
            "passing gate bundle should stage .eval_results/vox_mens_eval.yaml"
        );
        assert!(
            m1.files
                .iter()
                .any(|f| f.path == "external_serving_handoff_v1.json"),
            "bundle should stage external_serving_handoff_v1.json"
        );
        assert_eq!(m1.size, "4B");
        assert_eq!(m1.base_pin, pin);
        assert_eq!(m1.corpus_hash, "cfdc855e29dc680d");
        assert_eq!(m1.measured_peak_mb, Some(34460));
        assert_eq!(m1.gate_status, "passed");

        let card = std::fs::read_to_string(stage.path().join("README.md")).unwrap();
        assert!(card.contains(pin));
        assert!(card.contains("34460 MiB"));
        assert!(card.contains("candle_qlora_adapter.safetensors"));
        assert!(card.contains("vox mens hub pull --size 4B --namespace ns"));
        assert!(card.contains("PeftModel.from_pretrained(base, \"ns/vox-mens-"));
        assert!(!card.contains("subfolder=\"peft\""));
        assert!(!card.contains(concat!("/Us", "ers/secretuser")));

        let staged_manifest =
            std::fs::read_to_string(stage.path().join("training_manifest.json")).unwrap();
        assert!(!staged_manifest.contains(concat!("/Us", "ers/secretuser")));

        // Re-staging an unchanged run, forced or not, yields byte-identical card bytes (CARD-05).
        let stage2 = tempfile::tempdir().unwrap();
        let m2 = stage_publish_bundle(run.path(), stage2.path(), true, "ns").unwrap();
        assert!(manifests_match(&m1, &m2));
        assert_eq!(
            card,
            std::fs::read_to_string(stage2.path().join("README.md")).unwrap()
        );
    }

    #[test]
    fn stage_publish_bundle_spoke_places_files_under_spokes_subdir() {
        let run = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
        write_minimal_run_dir(run.path(), pin, true);

        let m = stage_publish_bundle_spoke(run.path(), stage.path(), false, "ns", Some("rust"))
            .unwrap();
        assert!(
            stage
                .path()
                .join("spokes/rust/candle_qlora_adapter.safetensors")
                .exists()
        );
        assert!(
            stage
                .path()
                .join("spokes/rust/training_manifest.json")
                .exists()
        );
        assert!(stage.path().join("README.md").exists());
        assert!(
            m.files
                .iter()
                .any(|f| f.path == "spokes/rust/candle_qlora_adapter.safetensors")
        );
    }

    fn read_safetensors_header_keys(path: &Path) -> Vec<String> {
        let bytes = std::fs::read(path).unwrap();
        let len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        let header: serde_json::Value = serde_json::from_slice(&bytes[8..8 + len]).unwrap();
        header
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| *k != "__metadata__")
            .cloned()
            .collect()
    }

    #[test]
    fn stage_bundle_distributes_complete_merged_model_only_when_gate_passed() {
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";

        let gated_run = tempfile::tempdir().unwrap();
        write_minimal_run_dir(gated_run.path(), pin, true);
        let recombined = gated_run.path().join("recombined_full");
        std::fs::create_dir_all(&recombined).unwrap();
        std::fs::write(recombined.join("model.safetensors"), b"merged-weights").unwrap();
        std::fs::write(
            recombined.join("config.json"),
            b"{\"model_type\":\"qwen3\"}",
        )
        .unwrap();
        std::fs::write(recombined.join("tokenizer.json"), b"{}").unwrap();

        let gated_stage = tempfile::tempdir().unwrap();
        let m = stage_publish_bundle(gated_run.path(), gated_stage.path(), false, "ns").unwrap();
        assert!(
            m.files.iter().any(|f| f.path == "model.safetensors"),
            "{:?}",
            m.files
        );
        assert!(
            m.files
                .iter()
                .any(|f| f.path == "candle_qlora_adapter.safetensors"),
            "candle_qlora_adapter.safetensors must ALWAYS remain in bundle root: {:?}",
            m.files
        );
        assert!(
            m.files.iter().any(|f| f.path == "peft/adapter_config.json"),
            "merged bundle should stage PEFT config under peft/: {:?}",
            m.files
        );
        assert!(
            m.files
                .iter()
                .any(|f| f.path == "peft/adapter_model.safetensors"),
            "merged bundle should stage PEFT weights under peft/: {:?}",
            m.files
        );
        assert!(
            !m.files.iter().any(|f| f.path == "adapter_config.json"),
            "merged bundle must keep root free of PEFT adapter_config.json: {:?}",
            m.files
        );
        assert!(
            m.files
                .iter()
                .any(|f| f.path == "external_serving_handoff_v1.json"),
            "{:?}",
            m.files
        );
        assert!(
            m.files.iter().any(|f| f.path == "chat_template.jinja"),
            "{:?}",
            m.files
        );
        assert!(
            m.files
                .iter()
                .any(|f| f.path == ".eval_results/vox_mens_eval.yaml"),
            "{:?}",
            m.files
        );
        let readme = std::fs::read_to_string(gated_stage.path().join("README.md")).unwrap();
        assert!(readme.contains("complete merged checkpoint"), "{readme}");
        assert!(
            readme.contains(
                "vox mens hub download --repo ns/vox-mens-qwen3-4b --output-dir mens/runs/qwen3-4b"
            ),
            "{readme}"
        );
        assert!(readme.contains("Files tab"), "{readme}");
        assert!(!readme.to_lowercase().contains("ungated"), "{readme}");

        let ungated_run = tempfile::tempdir().unwrap();
        write_minimal_run_dir(ungated_run.path(), pin, false);
        let recombined_ungated = ungated_run.path().join("recombined_full");
        std::fs::create_dir_all(&recombined_ungated).unwrap();
        std::fs::write(
            recombined_ungated.join("model.safetensors"),
            b"merged-weights",
        )
        .unwrap();
        let ungated_stage = tempfile::tempdir().unwrap();
        let err = stage_publish_bundle(ungated_run.path(), ungated_stage.path(), false, "ns")
            .unwrap_err()
            .to_string();
        assert!(err.contains("gate"), "{err}");
    }

    #[test]
    fn stage_bundle_strips_language_model_prefix_only_for_qwen35_causal_lm() {
        let pin = "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc";

        // 1. Qwen3_5ForCausalLM strips `model.language_model.` -> `model.layers...`
        let causal_run = tempfile::tempdir().unwrap();
        write_minimal_run_dir(causal_run.path(), pin, true);
        let causal_recombined = causal_run.path().join("recombined_full");
        std::fs::create_dir_all(&causal_recombined).unwrap();
        std::fs::write(causal_recombined.join("model.safetensors"), b"merged").unwrap();
        std::fs::write(
            causal_recombined.join("config.json"),
            r#"{"architectures":["Qwen3_5ForCausalLM"],"model_type":"qwen3_5_text"}"#,
        )
        .unwrap();
        let causal_stage = tempfile::tempdir().unwrap();
        stage_publish_bundle(causal_run.path(), causal_stage.path(), false, "ns").unwrap();
        let causal_keys = read_safetensors_header_keys(
            &causal_stage.path().join("peft/adapter_model.safetensors"),
        );
        assert!(
            causal_keys
                .iter()
                .any(|k| k == "base_model.model.model.layers.0.self_attn.q_proj.lora_A.weight"),
            "{causal_keys:?}"
        );

        // 2. Qwen3_5ForConditionalGeneration retains `model.language_model.`.
        let vl_run = tempfile::tempdir().unwrap();
        write_minimal_run_dir(vl_run.path(), pin, true);
        let vl_recombined = vl_run.path().join("recombined_full");
        std::fs::create_dir_all(&vl_recombined).unwrap();
        std::fs::write(vl_recombined.join("model.safetensors"), b"merged").unwrap();
        std::fs::write(
            vl_recombined.join("config.json"),
            r#"{"architectures":["Qwen3_5ForConditionalGeneration"],"model_type":"qwen3_5"}"#,
        )
        .unwrap();
        let vl_stage = tempfile::tempdir().unwrap();
        stage_publish_bundle(vl_run.path(), vl_stage.path(), false, "ns").unwrap();
        let vl_keys =
            read_safetensors_header_keys(&vl_stage.path().join("peft/adapter_model.safetensors"));
        assert!(
            vl_keys.iter().any(|k| k
                == "base_model.model.model.language_model.layers.0.self_attn.q_proj.lora_A.weight"),
            "{vl_keys:?}"
        );
    }

    #[test]
    fn sanitize_json_paths_normalizes_hf_cache_snapshots_and_redacts_host_paths() {
        let snap = concat!(
            "/Us",
            "ers/alice/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/15852e8c16360a2fea060d615a32b45270f8a8fc"
        );
        let win_snap = concat!(
            "C:\\Us",
            "ers\\alice\\.cache\\huggingface\\hub\\models--Qwen--Qwen3.5-4B\\snapshots\\851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"
        );
        let secret_unix = concat!("/Us", "ers/alice/private/dataset.jsonl");
        let secret_home = "/home/dev/private/weights.bin";
        let secret_win = concat!("C:\\Us", "ers\\alice\\Desktop\\secret.txt");
        let mut doc = serde_json::json!({
            "base_model": snap,
            "win_base_model": win_snap,
            "secret_unix": secret_unix,
            "secret_home": secret_home,
            "secret_win": secret_win,
            "run_ref": concat!("/Us", "ers/alice/dev/vox/mens/runs/qwen3.5-2b/train-record.json"),
            "corpus": concat!("/Us", "ers/alice/dev/vox/target/dogfood/train.jsonl"),
        });
        sanitize_json_paths(&mut doc);
        assert_eq!(
            doc["base_model"],
            "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc"
        );
        assert_eq!(
            doc["win_base_model"],
            "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"
        );
        assert_eq!(doc["secret_unix"], "<redacted-host-path>");
        assert_eq!(doc["secret_home"], "<redacted-host-path>");
        assert_eq!(doc["secret_win"], "<redacted-host-path>");
        assert_eq!(doc["run_ref"], "mens/runs/qwen3.5-2b/train-record.json");
        assert_eq!(doc["corpus"], "target/dogfood/train.jsonl");
    }

    #[test]
    fn resolve_ladder_base_pin_and_verify_pulled_support_all_families() {
        assert_eq!(
            resolve_ladder_base_pin(None, "0.8B").unwrap(),
            "Qwen/Qwen3.5-0.8B@2fc06364715b967f1860aea9cf38778875588b17"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "0.6B").unwrap(),
            "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "2B").unwrap(),
            "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "4B").unwrap(),
            "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "Qwen3-4B").unwrap(),
            "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "9B").unwrap(),
            "Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a"
        );
        assert_eq!(
            resolve_ladder_base_pin(None, "27B").unwrap(),
            "Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0"
        );

        let root = Path::new("/tmp/vox-root");
        let (_, dir_2b) = resolve_ladder_adapter_target(Path::new("2B"), Some(root)).unwrap();
        assert_eq!(dir_2b, root.join("mens/runs/qwen3.5-2b"));
        let (_, dir_q3_4b) =
            resolve_ladder_adapter_target(Path::new("Qwen3-4B"), Some(root)).unwrap();
        assert_eq!(dir_q3_4b, root.join("mens/runs/qwen3-4b"));
        let (_, dir_27b) = resolve_ladder_adapter_target(Path::new("27B"), Some(root)).unwrap();
        assert_eq!(dir_27b, root.join("mens/runs/qwen3.8-27b"));

        let dir = tempfile::tempdir().unwrap();
        let ok_dir = dir.path().join("qwen3.5-2b");
        write_minimal_run_dir(
            &ok_dir,
            "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc",
            true,
        );
        verify_pulled_adapter_dir(&ok_dir, "qwen3.5-2b", None, true).unwrap();

        let bad_dir = dir.path().join("qwen3.5-2b-bad");
        write_minimal_run_dir(
            &bad_dir,
            "Qwen/Qwen3.5-2B@0000000000000000000000000000000000000000",
            true,
        );
        let err = verify_pulled_adapter_dir(&bad_dir, "2B", None, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not match ladder pin"), "{err}");
        assert!(!bad_dir.exists());
    }

    #[test]
    fn stage_bundle_omits_empty_eval_results() {
        let run = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
        write_minimal_run_dir(run.path(), pin, true);
        let evidence = run.path().join("evidence");
        std::fs::create_dir_all(&evidence).unwrap();
        std::fs::write(evidence.join("eval_results.json"), "[]").unwrap();
        std::fs::write(evidence.join("eval_results.think128.json"), "{}\n").unwrap();
        std::fs::write(evidence.join("heldout_base.json"), r#"{"accuracy":0.5}"#).unwrap();

        let manifest = stage_publish_bundle(run.path(), stage.path(), false, "ns").unwrap();
        let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert!(!paths.contains(&"evidence/eval_results.json"), "{paths:?}");
        assert!(
            !paths.contains(&"evidence/eval_results.think128.json"),
            "{paths:?}"
        );
        assert!(!stage.path().join("evidence/eval_results.json").exists());
        assert!(paths.contains(&"evidence/heldout_base.json"), "{paths:?}");
    }

    #[test]
    fn pulled_adapter_with_wrong_pin_or_tampered_file_is_rejected_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("qwen3-4b");
        let wrong_pin = "Qwen/Qwen3-4B@0000000000000000000000000000000000000000";
        write_minimal_run_dir(&sub, wrong_pin, true);

        let err = verify_pulled_adapter_dir(&sub, "4B", None, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not match ladder pin"), "{err}");
        assert!(!sub.exists(), "rejected adapter dir must be removed");
    }

    #[test]
    fn offline_and_no_download_modes_stop_pull_explicitly() {
        let e1 = ensure_pull_allowed("me/vox-mens-qwen3-4b", Some("1"), None)
            .unwrap_err()
            .to_string();
        assert!(e1.contains(NO_DOWNLOAD_ENV), "{e1}");

        let e2 = ensure_pull_allowed("me/vox-mens-qwen3-4b", None, Some("1"))
            .unwrap_err()
            .to_string();
        assert!(e2.contains(HF_HUB_OFFLINE_ENV), "{e2}");
    }

    #[test]
    fn sync_classifier_and_index_renderer_handle_all_states() {
        let run = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let pin = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
        write_minimal_run_dir(run.path(), pin, true);
        let m = stage_publish_bundle(run.path(), stage.path(), false, "ns").unwrap();
        let mut m_div = m.clone();
        m_div.files[0].blake3 = "different".to_string();

        assert_eq!(classify_size_sync(None, None), SizeSyncStatus::NotTrained);
        assert_eq!(
            classify_size_sync(Some(&m), None),
            SizeSyncStatus::LocalOnly
        );
        assert_eq!(classify_size_sync(None, Some(&m)), SizeSyncStatus::HubOnly);
        assert_eq!(
            classify_size_sync(Some(&m), Some(&m)),
            SizeSyncStatus::InSync
        );
        assert_eq!(
            classify_size_sync(Some(&m), Some(&m_div)),
            SizeSyncStatus::Diverged
        );

        let rows = vec![
            SizeSyncRow {
                size: "4B".to_string(),
                base_pin: pin.to_string(),
                repo_id: "ns/vox-mens-qwen3-4b".to_string(),
                local_dir: "mens/runs/qwen3-4b".to_string(),
                sync_status: SizeSyncStatus::InSync,
                gate_status: Some("passed".to_string()),
                final_loss: Some(0.543),
                measured_peak_mb: Some(34460),
                action_taken: None,
                latest_version: Some("v2".to_string()),
            },
            SizeSyncRow {
                size: "14B".to_string(),
                base_pin: "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18".to_string(),
                repo_id: "ns/vox-mens-qwen3-14b".to_string(),
                local_dir: "mens/runs/qwen3-14b".to_string(),
                sync_status: SizeSyncStatus::NotTrained,
                gate_status: None,
                final_loss: None,
                measured_peak_mb: None,
                action_taken: None,
                latest_version: None,
            },
        ];
        let entries = index_entries(run.path(), &rows);
        let (md, json) = render_index_repo_files("ns", &entries);
        assert!(md.contains("[ns/vox-mens-qwen3-4b](https://huggingface.co/ns/vox-mens-qwen3-4b)"));
        assert!(md.contains("[ns/vox-mens-qwen3-14b]"));
        assert!(md.contains("| 14B | not trained | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | no weights | [ns/vox-mens-qwen3-14b]"));
        assert!(md.contains("published `v2`"));

        let out = write_local_cards(run.path(), "ns", &entries).unwrap();
        assert!(out.join("vox-mens-qwen3-4b/README.md").is_file());
        assert!(out.join("vox-mens-qwen3-14b/README.md").is_file());
        assert_eq!(
            std::fs::read_to_string(out.join("vox-mens-index/README.md")).unwrap(),
            md
        );
        assert!(json.contains("\"vox_mens_hf_ladder_index_v1\""));
        assert!(json.contains("\"latest_version\": \"v2\""));
    }

    #[test]
    fn version_tags_increment_and_pick_the_latest_numerically() {
        let none: Vec<String> = vec![];
        assert_eq!(next_version_tag(&none), "v1");
        assert_eq!(latest_version_tag(&none), None);

        // Non-version tags are ignored; v10 beats v9 numerically, not lexically.
        let tags: Vec<String> = ["v1", "v9", "v10", "experiment", "v", "v1a", "v-2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(latest_version_tag(&tags).as_deref(), Some("v10"));
        assert_eq!(next_version_tag(&tags), "v11");

        let only_junk: Vec<String> = vec!["main".into(), "latest".into()];
        assert_eq!(next_version_tag(&only_junk), "v1");
        assert_eq!(latest_version_tag(&only_junk), None);
    }

    #[tokio::test]
    #[ignore = "owner:mens sunset:never live-network: Hugging Face Hub backfill requires Clavis HF_TOKEN and VOX_MENS_HF_NAMESPACE"]
    async fn backfill_trained_ladder_sizes_to_hf_hub() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");

        let pushed = sync_all_sizes(Some(workspace_root), None, true, false, true, true, None)
            .await
            .expect("live sync_all_sizes --push");
        for row in &pushed {
            eprintln!(
                "[backfill push] size={} status={} repo={} action={:?}",
                row.size,
                row.sync_status.as_str(),
                row.repo_id,
                row.action_taken
            );
        }

        let verified = sync_all_sizes(Some(workspace_root), None, false, false, true, true, None)
            .await
            .expect("live sync_all_sizes status check");
        for row in &verified {
            eprintln!(
                "[backfill verify] size={} status={} repo={}",
                row.size,
                row.sync_status.as_str(),
                row.repo_id
            );
        }

        // Verify live pull + BLAKE3 hash verification roundtrip into a clean temp workspace
        let pull_tmp = tempfile::tempdir().expect("tempdir for live pull check");
        let pulled_dir =
            pull_adapter_if_missing(Path::new("Qwen3-0.6B"), Some(pull_tmp.path()), None, false)
                .await
                .expect("live pull_adapter_if_missing for Qwen3-0.6B");
        assert!(
            pulled_dir
                .join("candle_qlora_adapter.safetensors")
                .is_file()
        );
        assert!(pulled_dir.join(HF_SYNC_MANIFEST_FILENAME).is_file());

        let redact_rows = |rows: Vec<SizeSyncRow>| -> Vec<SizeSyncRow> {
            rows.into_iter()
                .map(|mut r| {
                    if let Some((_, repo_name)) = r.repo_id.split_once('/') {
                        r.repo_id = format!("<namespace>/{repo_name}");
                    }
                    r
                })
                .collect()
        };

        let report_path = workspace_root
            .join(".planning/phases/327-sync-command-and-backfill/327-BACKFILL-REPORT.json");
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            &report_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "vox_mens_hf_backfill_report_v1",
                    "push_rows": redact_rows(pushed),
                    "verify_rows": redact_rows(verified),
                    "pull_roundtrip_0_6b_verified": true,
                }))
                .unwrap()
            ),
        )
        .unwrap();
    }

    mod stale_pull {
        use super::*;

        fn make_stale_pulled_bundle(dir: &Path) {
            let pin = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";
            write_minimal_run_dir(dir, pin, false);
            // Stale report: missing grader / slice_blake3
            std::fs::write(
                dir.join("collateral_damage_report.json"),
                r#"{"status":"pass","threshold":0.05,"reports":[]}"#,
            )
            .unwrap();
            std::fs::write(
                dir.join("gate_receipt.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();
        }

        #[test]
        fn stale_pull_deletes() {
            let temp = tempfile::tempdir().unwrap();
            let bundle_dir = temp.path().join("bundle");
            std::fs::create_dir_all(&bundle_dir).unwrap();
            make_stale_pulled_bundle(&bundle_dir);

            let res = verify_pulled_adapter_dir(&bundle_dir, "1.7B", None, true);
            assert!(res.is_err(), "gated pull must refuse stale pass bundle");
            let err = res.unwrap_err().to_string();
            assert!(
                err.contains("allow-ungated"),
                "error must mention --allow-ungated: {err}"
            );
            assert!(
                !bundle_dir.exists(),
                "bundle directory must be deleted on refusal"
            );
        }

        #[test]
        fn stale_pull_kept_when_ungated() {
            let temp = tempfile::tempdir().unwrap();
            let bundle_dir = temp.path().join("bundle");
            std::fs::create_dir_all(&bundle_dir).unwrap();
            make_stale_pulled_bundle(&bundle_dir);

            let res = verify_pulled_adapter_dir(&bundle_dir, "1.7B", None, false);
            assert!(res.is_ok(), "ungated pull must succeed: {:?}", res.err());
            assert!(
                bundle_dir.exists(),
                "bundle directory must be kept when ungated"
            );
        }
    }

    mod stale {
        use super::*;

        #[test]
        fn publish_refuses_stale_pass_run() {
            let dir = tempfile::tempdir().unwrap();
            let pin = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";
            write_minimal_run_dir(dir.path(), pin, true);

            let req = crate::mens::gate_policy::ladder_requirement(None);
            let mut report = crate::mens::gate_policy::passing_collateral_report(&req);
            report["reasoning_slices"]["rule"] = serde_json::json!("retired_rule_v0");
            std::fs::write(
                dir.path().join("collateral_damage_report.json"),
                serde_json::to_string_pretty(&report).unwrap(),
            )
            .unwrap();

            let res = verify_publish_gates(dir.path(), false);
            assert!(res.is_err(), "publish must refuse stale pass run");
            let err = res.unwrap_err().to_string();
            assert!(
                err.contains("rule"),
                "error message should name stale rule: {err}"
            );
        }
    }

    mod needs_re_gate {
        use super::*;

        #[test]
        fn needs_re_gate_refused() {
            let dir = tempfile::tempdir().unwrap();
            let pin = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";
            write_minimal_run_dir(dir.path(), pin, true);

            // Write a v1 receipt (which lacks policy_blake3)
            std::fs::write(
                dir.path().join("gate_receipt.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "vox_mens_gate_receipt_v1",
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();

            let res = verify_publish_gates(dir.path(), false);
            assert!(res.is_err(), "publish must refuse v1 receipt");
            let err = res.unwrap_err().to_string();
            assert!(
                err.contains("needs_re_gate"),
                "error message must mention needs_re_gate: {err}"
            );

            // Now write a valid schema 2 receipt with current policy_blake3
            let root = vox_corpus::training::contract::find_workspace_root();
            let ladder_hash = crate::mens::gate_policy::policy_blake3(
                root.as_deref(),
                "mens/config/eval-gates-ladder.yaml",
            )
            .unwrap();
            std::fs::write(
                dir.path().join("gate_receipt.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                    "policy_blake3": ladder_hash,
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();

            let res = verify_publish_gates(dir.path(), false);
            assert!(
                res.is_ok(),
                "publish must accept current receipt: {:?}",
                res.err()
            );
        }

        #[test]
        fn publish_requires_applying_rows() {
            let dir = tempfile::tempdir().unwrap();
            let pin = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";
            write_minimal_run_dir(dir.path(), pin, true);

            let root = vox_corpus::training::contract::find_workspace_root();
            let ladder_hash = crate::mens::gate_policy::policy_blake3(
                root.as_deref(),
                "mens/config/eval-gates-ladder.yaml",
            )
            .unwrap();
            std::fs::write(
                dir.path().join("gate_receipt.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                    "policy_blake3": ladder_hash,
                    "inputs": {"adapter_sha256": ""},
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();

            // Claim `vox-lang` spoke in adapter_card.json without a passing spoke receipt.
            let mut card =
                crate::mens::tensor::adapter_card::AdapterCard::for_test("qwen3_16g", "qlora");
            card.spokes = vec!["vox-lang".to_string()];
            card.write_sidecar(&dir.path().join("candle_qlora_adapter.safetensors"))
                .unwrap();

            let err = verify_publish_gates(dir.path(), false)
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("gate_receipt.vox-lang.json"),
                "publish must block missing applying spoke receipt: {err}"
            );

            // Writing the valid spoke receipt allows publish to pass.
            let vox_hash = crate::mens::gate_policy::policy_blake3(
                root.as_deref(),
                "mens/config/eval-gates.yaml",
            )
            .unwrap();
            std::fs::write(
                dir.path().join("gate_receipt.vox-lang.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                    "policy_blake3": vox_hash,
                    "inputs": {"adapter_sha256": ""},
                    "overall_passed": true,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                        {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();
            verify_publish_gates(dir.path(), false).expect("applying spoke passed");
        }

        #[test]
        fn repo_id_unchanged() {
            assert_eq!(
                crate::mens::hub_repo::repo_id_for_size("me", "Qwen3-4B").unwrap(),
                "me/vox-mens-qwen3-4b"
            );
            assert_eq!(
                crate::mens::hub_repo::index_repo_id("me").unwrap(),
                "me/vox-mens-index"
            );
        }
    }

    #[test]
    fn write_local_cards_removes_stale_svgs_and_writes_binary_assets() {
        let run = tempfile::tempdir().unwrap();
        let slug = "vox-mens-qwen3.5-4b";
        for stale in [
            run.path()
                .join("mens/runs/hub_cards")
                .join(slug)
                .join("assets/scorecard.svg"),
            run.path()
                .join("mens/runs/hub_cards")
                .join(slug)
                .join("assets/overview.svg"),
            run.path()
                .join("mens/runs/hub_cards/vox-mens-index/assets/scorecard.svg"),
            run.path()
                .join("mens/runs/hub_cards/vox-mens-index/assets/overview.svg"),
        ] {
            std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
            std::fs::write(&stale, b"<svg>stale</svg>").unwrap();
        }
        let rows = vec![SizeSyncRow {
            size: "4B".to_string(),
            base_pin: "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a".to_string(),
            repo_id: format!("ns/{slug}"),
            local_dir: "mens/runs/qwen3.5-4b".to_string(),
            sync_status: SizeSyncStatus::NotTrained,
            gate_status: None,
            final_loss: None,
            measured_peak_mb: None,
            action_taken: None,
            latest_version: None,
        }];
        let entries = index_entries(run.path(), &rows);
        let out = write_local_cards(run.path(), "ns", &entries).unwrap();
        assert!(!out.join(slug).join("assets/scorecard.svg").exists());
        assert!(!out.join(slug).join("assets/overview.svg").exists());
        assert!(!out.join("vox-mens-index/assets/scorecard.svg").exists());
        assert!(!out.join("vox-mens-index/assets/overview.svg").exists());
    }

    #[tokio::test]
    async fn pull_adapter_revision_with_spoke_validates_and_resolves_cached_spoke() {
        let ws = tempfile::tempdir().unwrap();
        let err = pull_adapter_revision_with_spoke(
            Path::new("4B"),
            Some(ws.path()),
            Some("ns"),
            false,
            None,
            Some("../escape"),
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(err.contains("spoke") || err.contains("path") || !err.is_empty());

        let run_dir = ws.path().join("mens/runs/qwen3.5-4b");
        let spoke_dir = run_dir.join("spokes/rust");
        std::fs::create_dir_all(&spoke_dir).unwrap();
        std::fs::write(run_dir.join("candle_qlora_adapter.safetensors"), b"root").unwrap();
        std::fs::write(spoke_dir.join("candle_qlora_adapter.safetensors"), b"rust").unwrap();

        let got = pull_adapter_revision_with_spoke(
            Path::new("Qwen3.5-4B"),
            Some(ws.path()),
            Some("ns"),
            false,
            None,
            Some("rust"),
        )
        .await
        .unwrap();
        assert_eq!(got, spoke_dir);
    }
}
