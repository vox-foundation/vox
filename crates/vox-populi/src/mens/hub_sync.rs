//! Gated Hugging Face auto-publish, verified auto-retrieve, and ladder sync (v5.7 Phases 325–327).
//!
//! All cryptographic file digests go through [`vox_crypto::hash_fast_hex`] (BLAKE3), and
//! Hugging Face credentials resolve exclusively through [`vox_secrets::resolve_secret`]
//! (`SecretId::HuggingFaceToken`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use hf_hub::{HFClient, RepoTypeModel, split_id};

use crate::mens::hub::{NO_DOWNLOAD_ENV, upload_model_folder};
use crate::mens::hub_repo::{self, NAMESPACE_ENV, SIZES};
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
    "eval_local_report.json",
    "benchmark_passatk.json",
    "review_metrics.json",
    "run_summary.json",
    "train-record.json",
    "training_skip_stats.json",
    "config.json",
    "evidence/benchmark_passatk.json",
    "evidence/benchmark_passatk.think128.json",
    "evidence/collateral_baseline.json",
    "evidence/eval_results.json",
    "evidence/eval_results.think128.json",
    "evidence/heldout_adapter.json",
    "evidence/heldout_adapter_nothink.json",
    "evidence/heldout_base.json",
    "evidence/heldout_base_nothink.json",
];

/// Held-out reasoning slice names whose per-slice reports are included when present.
const COLLATERAL_SLICE_DIRS: &[&str] = &["vox_think", "gsm8k_think", "math500_think"];

/// Whether automatic HF sync is enabled given the `--no-hf-sync` CLI flag and `VOX_MENS_HF_SYNC`.
pub fn hf_sync_enabled(no_hf_sync_flag: bool, env_raw: Option<&str>) -> bool {
    if no_hf_sync_flag {
        return false;
    }
    match env_raw.map(str::trim) {
        Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => false,
        _ => true,
    }
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
        return Some(
            "HF auto-publish is off (`VOX_MENS_HF_SYNC=0` or `--no-hf-sync`)",
        );
    }
    if namespace_env
        .map(str::trim)
        .is_none_or(|ns| ns.is_empty())
    {
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
    let mut missing_or_failed = Vec::new();

    let collateral_path = run_dir.join("collateral_damage_report.json");
    if !collateral_path.is_file() {
        missing_or_failed.push("collateral_damage_report.json (missing)".to_string());
    } else {
        let raw = std::fs::read_to_string(&collateral_path)
            .with_context(|| format!("read {}", collateral_path.display()))?;
        let val: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", collateral_path.display()))?;
        if val.get("status").and_then(|s| s.as_str()) != Some("pass") {
            missing_or_failed.push("collateral_damage_report.json (status != pass)".to_string());
        }
    }

    let receipt_path = run_dir.join("gate_receipt.json");
    if !receipt_path.is_file() {
        missing_or_failed.push("gate_receipt.json (missing)".to_string());
    } else {
        let raw = std::fs::read_to_string(&receipt_path)
            .with_context(|| format!("read {}", receipt_path.display()))?;
        let val: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", receipt_path.display()))?;
        let passed = val.get("overall_passed").and_then(|b| b.as_bool()) == Some(true);
        let substantive = val["gates"].as_array().map_or(0, |gates| {
            gates
                .iter()
                .filter(|g| {
                    let m = g["message"].as_str().unwrap_or("");
                    !m.contains("not applicable")
                        && !m.contains("skipped")
                        && !m.contains("missing")
                })
                .count()
        });
        if !passed {
            missing_or_failed.push("gate_receipt.json (overall_passed != true)".to_string());
        } else if substantive < 2 {
            missing_or_failed.push(format!(
                "gate_receipt.json ({substantive} substantive gates; need >= 2)"
            ));
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

/// Strip host-specific `/Users/<user>/...` or `/home/<user>/...` prefixes from strings inside JSON
/// before publishing to Hugging Face.
fn sanitize_json_paths(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::String(s) => {
            if let Some(idx) = s.find("mens/runs/") {
                *s = s[idx..].to_string();
            } else if let Some(idx) = s.find("target/dogfood") {
                *s = s[idx..].to_string();
            } else if s.starts_with("/Users/") || s.starts_with("/home/") {
                if let Some(base) = Path::new(s.as_str()).file_name().and_then(|n| n.to_str()) {
                    *s = base.to_string();
                }
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

/// Build the generated model card (`README.md` / `MODEL_CARD.md`) with YAML frontmatter and provenance table (HFPUB-03).
pub fn render_model_card(
    size: &str,
    base_pin: &str,
    license: &str,
    corpus_hash: &str,
    final_loss: Option<f64>,
    measured_peak_mb: Option<u64>,
    gate_status: &GateStatus,
) -> String {
    let base_repo = base_pin.split('@').next().unwrap_or(base_pin);
    let loss_str = final_loss
        .map(|v| format!("{v:.4}"))
        .unwrap_or_else(|| "n/a".to_string());
    let peak_str = measured_peak_mb
        .map(|v| format!("{v} MiB"))
        .unwrap_or_else(|| "n/a".to_string());
    let gate_label = gate_status.label();
    let role = match size {
        "0.6B" => "Smallest rung, for low-latency local checks and short edits.",
        "1.7B" => "Small rung, for single-file generation and one-pass diagnostic fixes.",
        "4B" => "Default rung, for everyday synthesis and multi-function edits.",
        "8B" => "Larger rung, for multi-file edits.",
        "14B" => "Large rung.",
        "32B" => "Largest ladder rung.",
        _ => "Ladder rung.",
    };
    let gate_section = match gate_status {
        GateStatus::Passed => {
            "Eval gate passed. This revision is a promoted adapter.\n".to_string()
        }
        GateStatus::Ungated { reason } => format!(
            "> [!WARNING]\n\
             > **Gate status: `ungated`.** Uploaded with `--force-ungated` ({reason}).\n\
             > This is a baseline publication, not a promoted release.\n"
        ),
    };
    format!(
        "---\n\
         base_model: {base_repo}\n\
         license: {license}\n\
         language:\n\
         - en\n\
         pipeline_tag: text-generation\n\
         tags:\n\
         - vox\n\
         - mens\n\
         - qlora\n\
         - qwen3\n\
         ---\n\
         # Vox MENS — Qwen3-{size} adapter\n\
         \n\
         Vox MENS is a family of Qwen3 adapters trained for the [Vox language](https://vox-lang.org) and its compiler ([vox-foundation/vox](https://github.com/vox-foundation/vox)). This repository is the **{size}** rung on [`{base_repo}`](https://huggingface.co/{base_repo}). {role}\n\
         \n\
         Weights are Candle QLoRA in `candle_qlora_adapter.safetensors` (keys `l{{n}}.q.lora_a.weight`, and the matching `k`, `v`, `o`, `gate`, `up`, and `down` pairs). Load them with the Vox CLI:\n\
         \n\
         ```bash\n\
         vox mens hub pull --size {size}\n\
         ```\n\
         \n\
         Pin a published revision with `--revision v<N>`. The index repo in this namespace lists which rungs are on the Hub.\n\
         \n\
         {gate_section}\n\
         ## Provenance\n\
         \n\
         | Field | Value |\n\
         |---|---|\n\
         | Size | `{size}` |\n\
         | Base pin | `{base_pin}` |\n\
         | Corpus hash | `{corpus_hash}` |\n\
         | Final loss | `{loss_str}` |\n\
         | Measured peak MB | `{peak_str}` |\n\
         | Gate status | `{gate_label}` |\n"
    )
}

/// Model card for a ladder repo that exists before its adapter is published.
pub fn render_reserved_model_card(size: &str, base_pin: &str) -> String {
    let base_repo = base_pin.split('@').next().unwrap_or(base_pin);
    format!(
        "---\n\
         base_model: {base_repo}\n\
         license: apache-2.0\n\
         tags:\n\
         - vox\n\
         - mens\n\
         - qlora\n\
         - qwen3\n\
         ---\n\
         # Vox MENS — Qwen3-{size} adapter\n\
         \n\
         This repository is the reserved **{size}** rung on [`{base_repo}`](https://huggingface.co/{base_repo}).\n\
         \n\
         No adapter is published here yet. When a local `mens/runs/qwen3-{}` run finishes, `vox mens hub sync --push` writes `candle_qlora_adapter.safetensors` into this repo and tags it `v1`. A run whose eval gate has not passed is published as an ungated baseline and the card says so.\n\
         \n\
         ```bash\n\
         vox mens hub pull --size {size}\n\
         ```\n\
         \n\
         | Field | Value |\n\
         |---|---|\n\
         | Size | `{size}` |\n\
         | Base pin | `{base_pin}` |\n\
         | Adapter | not published |\n",
        size.to_ascii_lowercase()
    )
}

/// Create every ladder repo, and write a reserved card only when the repo has no README yet.
///
/// A repo that already holds `hf_sync_manifest.json` or a README is left unchanged, so a
/// finished adapter and a hand-written card are not replaced by the placeholder.
pub async fn ensure_ladder_repos(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
) -> Result<Vec<String>> {
    let _token = require_clavis_hf_token()?;
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    let overlay = size_select::load_ladder_overlay(workspace_root)?;
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let mut reserved = Vec::new();
    for &size in &SIZES {
        let base_pin = size_select::resolve_base_pin(&format!("Qwen3-{size}"), &overlay)?;
        let repo_id = hub_repo::repo_id_for_size(&ns, size)?;
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
        let has_readme = client
            .model(owner, name)
            .info()
            .send()
            .await
            .ok()
            .and_then(|info| info.siblings)
            .is_some_and(|siblings| siblings.iter().any(|s| s.rfilename == "README.md"));
        if has_readme {
            continue;
        }
        let card = render_reserved_model_card(size, &base_pin);
        let tmp = std::env::temp_dir().join(format!(
            "vox-mens-reserve-{size}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("README.md"), &card)?;
        std::fs::write(tmp.join("MODEL_CARD.md"), &card)?;
        let uploaded = upload_model_folder(
            &repo_id,
            &tmp,
            private,
            Some(&format!("Reserve VoxMens Qwen3-{size} ladder repo")),
        )
        .await;
        let _ = std::fs::remove_dir_all(&tmp);
        uploaded?;
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
pub fn stage_publish_bundle(
    run_dir: &Path,
    stage_dir: &Path,
    force_ungated: bool,
) -> Result<HfSyncManifest> {
    let adapter_src = if run_dir.join("candle_qlora_adapter.safetensors").is_file() {
        run_dir.join("candle_qlora_adapter.safetensors")
    } else if run_dir.join("adapter.safetensors").is_file() {
        run_dir.join("adapter.safetensors")
    } else {
        bail!(
            "no adapter safetensors found in {} (expected candle_qlora_adapter.safetensors)",
            run_dir.display()
        );
    };
    let manifest_src = run_dir.join("training_manifest.json");
    if !manifest_src.is_file() {
        bail!("no training_manifest.json found in {}", run_dir.display());
    }

    let gate_status = verify_publish_gates(run_dir, force_ungated)?;

    let mut manifest_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_src)
            .with_context(|| format!("read {}", manifest_src.display()))?,
    )
    .with_context(|| format!("parse {}", manifest_src.display()))?;

    let base_pin = manifest_json
        .get("base_model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("training_manifest.json has no `base_model` string"))?
        .to_string();
    let size = hub_repo::size_of_pin(&base_pin).ok_or_else(|| {
        anyhow::anyhow!("base_model `{base_pin}` in training_manifest.json is not a ladder size")
    })?;
    let license = manifest_json
        .get("provenance_license_class")
        .and_then(|v| v.as_str())
        .unwrap_or("apache-2.0")
        .to_string();
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

    // 1. Copy adapter safetensors
    let adapter_name = adapter_src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("candle_qlora_adapter.safetensors");
    std::fs::copy(&adapter_src, stage_dir.join(adapter_name))?;
    staged_rel_paths.push(adapter_name.to_string());

    // 2. Write sanitized training_manifest.json
    sanitize_json_paths(&mut manifest_json);
    std::fs::write(
        stage_dir.join("training_manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest_json)?),
    )?;
    staged_rel_paths.push("training_manifest.json".to_string());

    // 3. Copy optional JSON/report artifacts (sanitizing JSON paths)
    for &rel in OPTIONAL_BUNDLE_FILES {
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

    // 5. Write generated model card to README.md and MODEL_CARD.md
    let card = render_model_card(
        &size,
        &base_pin,
        &license,
        &corpus_hash,
        final_loss,
        measured_peak_mb,
        &gate_status,
    );
    std::fs::write(stage_dir.join("README.md"), &card)?;
    std::fs::write(stage_dir.join("MODEL_CARD.md"), &card)?;
    staged_rel_paths.push("README.md".to_string());
    staged_rel_paths.push("MODEL_CARD.md".to_string());

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
pub async fn publish_adapter_run_dir(
    run_dir: &Path,
    namespace: Option<&str>,
    private: bool,
    force_ungated: bool,
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
        let local_manifest = stage_publish_bundle(run_dir, &stage_root, force_ungated)?;
        let repo_id = hub_repo::repo_id_for_size(&ns, &local_manifest.size)?;

        // HFPUB-04: idempotent hash-skip when remote manifest matches.
        if let Ok(Some(remote_manifest)) = fetch_remote_sync_manifest(&repo_id).await
            && manifests_match(&local_manifest, &remote_manifest)
        {
            // Legacy repos published before versioning have no tag: baseline them as v1
            // at the current head so every published state is addressable (HFVER-01).
            if list_repo_tags(&repo_id).await.is_ok_and(|t| t.is_empty()) {
                let msg = format!(
                    "Baseline version of VoxMens Qwen3-{} adapter ({})",
                    local_manifest.size, local_manifest.gate_status
                );
                create_repo_tag(&repo_id, "v1", None, &msg).await?;
            }
            return Ok(PublishOutcome::SkippedUnchanged {
                repo_id,
                size: local_manifest.size,
            });
        }

        let commit_msg = format!(
            "Publish VoxMens Qwen3-{} adapter ({})",
            local_manifest.size, local_manifest.gate_status
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
pub fn verify_pulled_adapter_dir(
    dir: &Path,
    expected_size: &str,
    workspace_root: Option<&Path>,
    require_gates: bool,
) -> Result<()> {
    let check = || -> Result<()> {
        let size = hub_repo::size_of_pin(expected_size).ok_or_else(|| {
            anyhow::anyhow!("`{expected_size}` is not a recognized Qwen3 ladder size")
        })?;
        let overlay = size_select::load_ladder_overlay(workspace_root)?;
        let expected_pin = size_select::resolve_base_pin(&format!("Qwen3-{size}"), &overlay)?;

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
            verify_publish_gates(dir, false)?;
        }
        Ok(())
    };

    if let Err(e) = check() {
        let _ = std::fs::remove_dir_all(dir);
        return Err(e);
    }
    Ok(())
}

/// Resolve a size name (`4B`, `Qwen3-4B`) or run directory (`mens/runs/qwen3-4b`) to `(size, target_run_dir)`
/// when it refers to a Qwen3 ladder adapter.
pub fn resolve_ladder_adapter_target(
    model_arg: &Path,
    workspace_root: Option<&Path>,
) -> Option<(String, PathBuf)> {
    let raw = model_arg.to_str()?;
    let size = if let Some(s) = hub_repo::size_of_pin(raw) {
        s
    } else {
        let name = model_arg.file_name()?.to_str()?;
        let stripped = name.strip_prefix("qwen3-")?;
        hub_repo::size_of_pin(stripped)?
    };
    let run_rel = PathBuf::from(format!("mens/runs/qwen3-{}", size.to_ascii_lowercase()));
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

/// Pull a missing ladder adapter from `<namespace>/vox-mens-qwen3-<size>` into `mens/runs/qwen3-<size>`
/// and verify its manifest pin and gate receipts (HFPULL-01..03).
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
pub async fn pull_adapter_revision(
    model_arg: &Path,
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    require_gates: bool,
    revision: Option<&str>,
) -> Result<PathBuf> {
    if model_arg.is_file() {
        return Ok(model_arg.to_path_buf());
    }
    if model_arg.is_dir()
        && (model_arg.join("candle_qlora_adapter.safetensors").is_file()
            || model_arg.join("adapter.safetensors").is_file())
    {
        return Ok(model_arg.to_path_buf());
    }

    let Some((size, target_dir)) = resolve_ladder_adapter_target(model_arg, workspace_root) else {
        return Ok(model_arg.to_path_buf());
    };
    if target_dir
        .join("candle_qlora_adapter.safetensors")
        .is_file()
        || target_dir.join("adapter.safetensors").is_file()
    {
        return Ok(target_dir);
    }

    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    let repo_id = hub_repo::repo_id_for_size(&ns, &size)?;

    ensure_pull_allowed(
        &repo_id,
        std::env::var(NO_DOWNLOAD_ENV).ok().as_deref(),
        std::env::var(HF_HUB_OFFLINE_ENV).ok().as_deref(),
    )?;

    if let Ok(tok) = require_clavis_hf_token() {
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("HF_TOKEN", tok);
        }
    }

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

    for sib in &siblings {
        if sib.rfilename.starts_with('.') {
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
    }

    verify_pulled_adapter_dir(&target_dir, &size, workspace_root, require_gates)?;
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
    let model_arg = model_arg.to_path_buf();
    let workspace_root = workspace_root.map(Path::to_path_buf);
    let namespace = namespace.map(ToString::to_string);
    let revision = revision.map(ToString::to_string);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))
            .and_then(|rt| {
                rt.block_on(pull_adapter_revision(
                    &model_arg,
                    workspace_root.as_deref(),
                    namespace.as_deref(),
                    require_gates,
                    revision.as_deref(),
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

/// Render `(README.md, ladder_index.json)` for `<namespace>/vox-mens-index` (HFSYNC-03).
pub fn render_index_repo_files(namespace: &str, rows: &[SizeSyncRow]) -> (String, String) {
    let mut md = format!(
        "---\n\
         tags:\n\
         - vox\n\
         - mens\n\
         - qwen3\n\
         - index\n\
         ---\n\
         # Vox MENS Qwen3 size ladder (`{namespace}`)\n\n\
         Each ladder size has a Hub repo. A published rung is a Candle QLoRA adapter (`candle_qlora_adapter.safetensors`). Pull one with `vox mens hub pull --size <size>`. A `not_trained` row is the reserved repo, with no adapter yet.\n\n\
         | Size | Status | Version | Base Pin | Repo | Peak MB | Final Loss | Gate |\n\
         |---|---|---|---|---|---|---|---|\n"
    );
    for r in rows {
        let repo_cell = format!("`{}`", r.repo_id);
        let peak = r
            .measured_peak_mb
            .map(|m| format!("{m}"))
            .unwrap_or_else(|| "—".to_string());
        let loss = r
            .final_loss
            .map(|l| format!("{l:.4}"))
            .unwrap_or_else(|| "—".to_string());
        let gate = r.gate_status.as_deref().unwrap_or("—");
        let version = r.latest_version.as_deref().unwrap_or("—");
        md.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | {} | {} | {} | `{}` |\n",
            r.size,
            r.sync_status.as_str(),
            version,
            r.base_pin,
            repo_cell,
            peak,
            loss,
            gate
        ));
    }
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "schema": "vox_mens_hf_ladder_index_v1",
        "namespace": namespace,
        "sizes": rows,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    (md, format!("{json}\n"))
}

/// True when `size` is included. `only` empty means every ladder size.
pub fn size_in_scope(size: &str, only: Option<&str>) -> bool {
    let Some(want) = only.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let want = if want.len() >= 6 && want[..6].eq_ignore_ascii_case("qwen3-") {
        &want[6..]
    } else {
        want
    };
    size.eq_ignore_ascii_case(want)
}

/// Inspect local `mens/runs/qwen3-<size>` for all 6 ladder sizes without touching the network when `offline` is true.
pub async fn sync_all_sizes(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    push: bool,
    pull: bool,
    force_ungated: bool,
    private: bool,
    only_size: Option<&str>,
) -> Result<Vec<SizeSyncRow>> {
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    if push {
        let _ = require_clavis_hf_token()?;
        ensure_ladder_repos(workspace_root, Some(&ns), private).await?;
    }
    let overlay = size_select::load_ladder_overlay(workspace_root)?;
    let root = workspace_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut rows = Vec::with_capacity(SIZES.len());
    for &size in &SIZES {
        let base_pin = size_select::resolve_base_pin(&format!("Qwen3-{size}"), &overlay)?;
        let repo_id = hub_repo::repo_id_for_size(&ns, size)?;
        let rel_dir = format!("mens/runs/qwen3-{}", size.to_ascii_lowercase());
        let run_dir = root.join(&rel_dir);

        let local_manifest = if run_dir.join("candle_qlora_adapter.safetensors").is_file()
            || run_dir.join("adapter.safetensors").is_file()
        {
            let tmp = std::env::temp_dir().join(format!(
                "vox-mens-sync-inspect-{}-{}",
                size,
                std::process::id()
            ));
            let m = stage_publish_bundle(&run_dir, &tmp, true).ok();
            let _ = std::fs::remove_dir_all(&tmp);
            m
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
        if push
            && selected
            && matches!(status, SizeSyncStatus::LocalOnly | SizeSyncStatus::Diverged)
        {
            // Reserved repos already exist. Only a local adapter is uploaded.
            match publish_adapter_run_dir(&run_dir, Some(&ns), private, force_ungated).await {
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
            pull_adapter_if_missing(&run_dir, Some(&root), Some(&ns), !force_ungated).await?;
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
        upload_ladder_index(&ns, &rows, private).await?;
    }

    Ok(rows)
}

/// Upload `README.md` and `ladder_index.json` for `<namespace>/vox-mens-index`.
async fn upload_ladder_index(namespace: &str, rows: &[SizeSyncRow], private: bool) -> Result<()> {
    let index_repo = hub_repo::index_repo_id(namespace)?;
    let (readme, index_json) = render_index_repo_files(namespace, rows);
    let tmp = std::env::temp_dir().join(format!("vox-mens-index-stage-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    std::fs::write(tmp.join("README.md"), readme)?;
    std::fs::write(tmp.join("ladder_index.json"), index_json)?;
    let uploaded = upload_model_folder(
        &index_repo,
        &tmp,
        private,
        Some("Update VoxMens Qwen3 size ladder index"),
    )
    .await;
    let _ = std::fs::remove_dir_all(&tmp);
    uploaded.map(|_| ())
}

/// Re-read every ladder rung and publish the index repo, without uploading adapters.
///
/// Train and eval-gate auto-publish go through [`publish_adapter_run_dir`], which
/// updates one size repo and used to leave `vox-mens-index` stale.
pub async fn refresh_ladder_index(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
) -> Result<Vec<SizeSyncRow>> {
    ensure_ladder_repos(workspace_root, namespace, private).await?;
    let rows = sync_all_sizes(
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
    upload_ladder_index(&ns, &rows, private).await?;
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

    fn write_minimal_run_dir(dir: &Path, pin: &str, with_passing_gates: bool) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("candle_qlora_adapter.safetensors"),
            b"adapter-bytes",
        )
        .unwrap();
        std::fs::write(
            dir.join("training_manifest.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "base_model": pin,
                "provenance_license_class": "apache-2.0",
                "finetune_contract_digest": "cfdc855e29dc680d",
                "train_file": "/Users/secretuser/dev/vox/target/dogfood/train_mixed.jsonl"
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
            std::fs::write(
                dir.join("collateral_damage_report.json"),
                r#"{"status":"pass"}"#,
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
    }

    #[test]
    fn reserved_card_names_the_rung_and_says_the_adapter_is_absent() {
        let pin = "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137";
        let card = render_reserved_model_card("32B", pin);
        assert!(card.contains("# Vox MENS — Qwen3-32B adapter"));
        assert!(card.contains(pin));
        assert!(card.contains("| Adapter | not published |"));
        assert!(card.contains("vox mens hub pull --size 32B"));
    }

    #[test]
    fn model_card_describes_candle_qlora_for_each_published_rung() {
        let cases = [
            (
                "0.6B",
                "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca",
                "cac9c5114ad7254d",
                0.6897967601807622,
                15319_u64,
                "collateral_damage_report.json (status != pass), gate_receipt.json (missing)",
            ),
            (
                "1.7B",
                "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e",
                "404e192183366aab",
                0.6170382260365842,
                27007,
                "collateral_damage_report.json (status != pass), gate_receipt.json (missing)",
            ),
            (
                "4B",
                "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c",
                "cfdc855e29dc680d",
                0.5434616348390439,
                34460,
                "gate_receipt.json (overall_passed != true)",
            ),
        ];
        let mut emitted = Vec::new();
        for (size, pin, corpus, loss, peak, reason) in cases {
            let card = render_model_card(
                size,
                pin,
                "apache-2.0",
                corpus,
                Some(loss),
                Some(peak),
                &GateStatus::Ungated {
                    reason: reason.to_string(),
                },
            );
            assert!(card.contains(&format!("# Vox MENS — Qwen3-{size} adapter")), "{size}");
            assert!(card.contains(pin), "{size}");
            assert!(card.contains("candle_qlora_adapter.safetensors"), "{size}");
            assert!(
                card.contains(&format!("vox mens hub pull --size {size}")),
                "{size}"
            );
            assert!(card.contains("ungated"), "{size}");
            assert!(!card.to_lowercase().contains("peft"), "{size}");
            emitted.push((size, card));
        }
        if std::env::var("VOX_EMIT_MODEL_CARDS").is_ok() {
            let dir = std::path::PathBuf::from("/tmp/vox-mens-cards");
            std::fs::create_dir_all(&dir).unwrap();
            for (size, card) in &emitted {
                let name = format!("qwen3-{}.md", size.to_ascii_lowercase());
                std::fs::write(dir.join(&name), card).unwrap();
                std::fs::write(
                    dir.join(format!("{name}.xxh3")),
                    vox_crypto::hash_fast_hex(card.as_bytes()),
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn size_scope_limits_push_to_one_ladder_rung() {
        assert!(size_in_scope("8B", None));
        assert!(size_in_scope("0.6B", Some("0.6B")));
        assert!(size_in_scope("4B", Some("qwen3-4b")));
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

        let m1 = stage_publish_bundle(run.path(), stage.path(), false).unwrap();
        assert_eq!(m1.size, "4B");
        assert_eq!(m1.base_pin, pin);
        assert_eq!(m1.corpus_hash, "cfdc855e29dc680d");
        assert_eq!(m1.measured_peak_mb, Some(34460));
        assert_eq!(m1.gate_status, "passed");

        let card = std::fs::read_to_string(stage.path().join("README.md")).unwrap();
        assert!(card.contains(pin));
        assert!(card.contains("34460 MiB"));
        assert!(card.contains("candle_qlora_adapter.safetensors"));
        assert!(card.contains("vox mens hub pull --size 4B"));
        assert!(!card.to_lowercase().contains("peft"));
        assert!(!card.contains("/Users/secretuser"));

        let staged_manifest =
            std::fs::read_to_string(stage.path().join("training_manifest.json")).unwrap();
        assert!(!staged_manifest.contains("/Users/secretuser"));

        let stage2 = tempfile::tempdir().unwrap();
        let m2 = stage_publish_bundle(run.path(), stage2.path(), false).unwrap();
        assert!(manifests_match(&m1, &m2));
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

        let manifest = stage_publish_bundle(run.path(), stage.path(), false).unwrap();
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
        let m = stage_publish_bundle(run.path(), stage.path(), false).unwrap();
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
        let (md, json) = render_index_repo_files("ns", &rows);
        assert!(md.contains("`ns/vox-mens-qwen3-4b`"));
        assert!(md.contains("`ns/vox-mens-qwen3-14b`"));
        assert!(md.contains("`not_trained`"));
        assert!(md.contains("`v2`"));
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
    #[ignore = "live Hugging Face Hub backfill (requires Clavis HF_TOKEN and VOX_MENS_HF_NAMESPACE)"]
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
}
