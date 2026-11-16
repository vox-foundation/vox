//! Zero-tensor-allocation Candle QLoRA to Hugging Face PEFT LoRA export.
//!
//! Reads `candle_qlora_adapter.safetensors` and `adapter_manifest.json` from a local training
//! run directory, rewrites only the JSON header of the `.safetensors` file (stream-copying the
//! raw tensor payload verbatim without decoding floats), and emits HF PEFT `adapter_model.safetensors`
//! plus `adapter_config.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum SafeTensors JSON header size accepted before failing closed (64 MiB).
const MAX_HEADER_BYTES: usize = 64 * 1024 * 1024;

/// Output metadata returned by [`export_peft_from_candle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeftExportOutcome {
    pub target_modules: Vec<String>,
    pub base_model_name_or_path: String,
    pub revision: Option<String>,
    pub use_rslora: bool,
}

#[derive(Debug, Deserialize)]
struct CandleAdapterManifest {
    #[serde(default = "default_rank")]
    rank: usize,
    #[serde(default = "default_alpha")]
    alpha: serde_json::Number,
    #[serde(default)]
    use_rslora: bool,
    #[serde(default)]
    base_model: Option<String>,
    #[serde(default)]
    base_key_map: BTreeMap<String, String>,
}

fn default_rank() -> usize {
    16
}

fn default_alpha() -> serde_json::Number {
    serde_json::Number::from(8_u64)
}

#[derive(Debug, Serialize)]
struct PeftAdapterConfig {
    peft_type: &'static str,
    task_type: &'static str,
    base_model_name_or_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    r: usize,
    lora_alpha: serde_json::Number,
    use_rslora: bool,
    lora_dropout: f64,
    bias: &'static str,
    fan_in_fan_out: bool,
    inference_mode: bool,
    target_modules: Vec<String>,
}

fn is_host_path(norm: &str) -> bool {
    if norm.starts_with('/') || norm.starts_with("//") {
        return true;
    }
    let bytes = norm.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

/// Normalize a base model identifier, pinned HF ID (`org/repo@rev`), or local Hugging Face hub
/// cache snapshot directory (`.../models--<org>--<repo>/snapshots/<sha>`) into `(repo_id, rev)`,
/// stripping any host filesystem prefix.
#[must_use]
pub fn normalize_hf_repo_id_and_rev(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ("unknown-base-model".to_string(), None);
    }
    let norm = trimmed.replace('\\', "/");
    let norm = norm.trim_end_matches('/');

    let segments: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    for (idx, seg) in segments.iter().enumerate() {
        if let Some(slug) = seg.strip_prefix("models--")
            && let Some((org, repo)) = slug.split_once("--")
            && !org.is_empty()
            && !repo.is_empty()
        {
            let rev = if segments.get(idx + 1) == Some(&"snapshots") {
                segments
                    .get(idx + 2)
                    .map(|r| r.trim())
                    .filter(|r| !r.is_empty() && *r != "." && *r != "..")
                    .map(ToString::to_string)
            } else {
                None
            };
            return (format!("{org}/{repo}"), rev);
        }
    }

    if !is_host_path(norm) {
        if let Some((repo, rev)) = norm.rsplit_once('@') {
            let clean_repo = repo.trim();
            let clean_rev = rev.trim();
            return (
                if clean_repo.is_empty() {
                    "unknown-base-model".to_string()
                } else {
                    clean_repo.to_string()
                },
                (!clean_rev.is_empty()).then(|| clean_rev.to_string()),
            );
        }
        return (norm.to_string(), None);
    }

    let (path_only, rev) = match norm.rsplit_once('@') {
        Some((p, r)) if !r.trim().is_empty() => (p, Some(r.trim().to_string())),
        _ => (norm, None),
    };
    let leaf = path_only
        .split('/')
        .rfind(|s| !s.is_empty())
        .unwrap_or("unknown-base-model");
    (leaf.to_string(), rev)
}

fn rewrite_candle_tensor_key(
    key: &str,
    base_key_map: &BTreeMap<String, String>,
    strip_language_model_prefix: bool,
) -> (String, Option<String>) {
    let (logical, lora_suffix) = if let Some(stem) = key.strip_suffix(".lora_a.weight") {
        (stem, Some(".lora_A.weight"))
    } else if let Some(stem) = key.strip_suffix(".lora_b.weight") {
        (stem, Some(".lora_B.weight"))
    } else if let Some(stem) = key.strip_suffix(".lora_A.weight") {
        (stem, Some(".lora_A.weight"))
    } else if let Some(stem) = key.strip_suffix(".lora_B.weight") {
        (stem, Some(".lora_B.weight"))
    } else {
        (key, None)
    };

    let Some(suffix) = lora_suffix else {
        let adjusted = if strip_language_model_prefix {
            key.replace("model.language_model.", "model.")
        } else {
            key.to_string()
        };
        return (adjusted, None);
    };

    let mapped = base_key_map
        .get(logical)
        .map(String::as_str)
        .unwrap_or(logical);
    let without_weight = mapped.strip_suffix(".weight").unwrap_or(mapped);
    let stripped = if strip_language_model_prefix {
        without_weight.replace("model.language_model.", "model.")
    } else {
        without_weight.to_string()
    };
    let prefixed = if stripped.starts_with("base_model.model.") {
        stripped
    } else {
        format!("base_model.model.{stripped}")
    };
    let leaf = prefixed
        .rsplit('.')
        .next()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    (format!("{prefixed}{suffix}"), leaf)
}

/// Export a Hugging Face PEFT LoRA directory (`adapter_model.safetensors` + `adapter_config.json`)
/// from a Candle QLoRA training adapter without loading or copying tensor weights into float
/// buffers.
pub fn export_peft_from_candle(
    adapter_safetensors: &Path,
    adapter_manifest_json: &Path,
    out_dir: &Path,
    strip_language_model_prefix: bool,
) -> Result<PeftExportOutcome> {
    let manifest_raw = std::fs::read_to_string(adapter_manifest_json).with_context(|| {
        format!(
            "read adapter manifest for PEFT export {}",
            adapter_manifest_json.display()
        )
    })?;
    let manifest: CandleAdapterManifest =
        serde_json::from_str(&manifest_raw).with_context(|| {
            format!(
                "parse adapter manifest for PEFT export {}",
                adapter_manifest_json.display()
            )
        })?;

    let (base_model_name_or_path, revision) =
        normalize_hf_repo_id_and_rev(manifest.base_model.as_deref().unwrap_or(""));

    let in_file = File::open(adapter_safetensors).with_context(|| {
        format!(
            "open Candle adapter safetensors {}",
            adapter_safetensors.display()
        )
    })?;
    let total_file_len = in_file
        .metadata()
        .with_context(|| format!("stat {}", adapter_safetensors.display()))?
        .len();
    if total_file_len < 8 {
        bail!(
            "Candle adapter safetensors {} is smaller than 8-byte header ({total_file_len} bytes)",
            adapter_safetensors.display()
        );
    }

    let mut reader = BufReader::new(in_file);
    let mut len_buf = [0_u8; 8];
    reader
        .read_exact(&mut len_buf)
        .context("read safetensors header length")?;
    let header_len_u64 = u64::from_le_bytes(len_buf);
    let header_len =
        usize::try_from(header_len_u64).context("safetensors header length exceeds usize")?;
    if header_len == 0 || header_len > MAX_HEADER_BYTES {
        bail!("safetensors header length {header_len} out of valid bounds");
    }
    if 8_u64
        .checked_add(header_len_u64)
        .is_none_or(|end| end > total_file_len)
    {
        bail!("safetensors header ({header_len} bytes) exceeds file size ({total_file_len} bytes)");
    }

    let mut header_bytes = vec![0_u8; header_len];
    reader
        .read_exact(&mut header_bytes)
        .context("read safetensors JSON header")?;
    let header_obj: BTreeMap<String, Value> = serde_json::from_slice(&header_bytes)
        .context("parse Candle safetensors JSON header as object")?;

    let mut new_header: BTreeMap<String, Value> = BTreeMap::new();
    let mut meta_map: BTreeMap<String, Value> = BTreeMap::new();
    let mut target_modules_set = BTreeSet::new();

    for (key, value) in header_obj {
        if key == "__metadata__" {
            if let Some(obj) = value.as_object() {
                for (mk, mv) in obj {
                    if let Some(s) = mv.as_str() {
                        meta_map.insert(mk.clone(), Value::String(s.to_string()));
                    }
                }
            }
            continue;
        }
        let (peft_key, leaf) =
            rewrite_candle_tensor_key(&key, &manifest.base_key_map, strip_language_model_prefix);
        if let Some(leaf_name) = leaf {
            target_modules_set.insert(leaf_name);
        }
        if new_header.insert(peft_key.clone(), value).is_some() {
            bail!("PEFT key collision after mapping `{key}` -> `{peft_key}`");
        }
    }
    meta_map.insert("format".to_string(), Value::String("pt".to_string()));
    new_header.insert(
        "__metadata__".to_string(),
        serde_json::to_value(meta_map).context("serialize safetensors __metadata__")?,
    );

    let mut out_header_bytes =
        serde_json::to_vec(&new_header).context("serialize rewritten PEFT safetensors header")?;
    while out_header_bytes.len() % 8 != 0 {
        out_header_bytes.push(b' ');
    }
    let out_header_len = out_header_bytes.len() as u64;

    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("create PEFT output dir {}", out_dir.display()))?;
    let out_weights_path = out_dir.join("adapter_model.safetensors");
    {
        let out_file = File::create(&out_weights_path).with_context(|| {
            format!("create PEFT adapter weights {}", out_weights_path.display())
        })?;
        let mut writer = BufWriter::new(out_file);
        writer
            .write_all(&out_header_len.to_le_bytes())
            .context("write PEFT safetensors header length")?;
        writer
            .write_all(&out_header_bytes)
            .context("write PEFT safetensors JSON header")?;
        let expected_payload = total_file_len - (8 + header_len_u64);
        let copied = std::io::copy(&mut reader, &mut writer)
            .context("stream raw safetensors payload to PEFT adapter_model.safetensors")?;
        if copied != expected_payload {
            bail!(
                "short copy writing PEFT safetensors payload: expected {expected_payload} bytes, wrote {copied}"
            );
        }
        writer
            .flush()
            .context("flush PEFT adapter_model.safetensors")?;
    }

    let target_modules: Vec<String> = target_modules_set.into_iter().collect();
    let cfg = PeftAdapterConfig {
        peft_type: "LORA",
        task_type: "CAUSAL_LM",
        base_model_name_or_path: base_model_name_or_path.clone(),
        revision: revision.clone(),
        r: manifest.rank,
        lora_alpha: manifest.alpha,
        use_rslora: manifest.use_rslora,
        lora_dropout: 0.0,
        bias: "none",
        fan_in_fan_out: false,
        inference_mode: true,
        target_modules: target_modules.clone(),
    };
    let cfg_json =
        serde_json::to_string_pretty(&cfg).context("serialize PEFT adapter_config.json")?;
    std::fs::write(out_dir.join("adapter_config.json"), cfg_json)
        .context("write PEFT adapter_config.json")?;

    Ok(PeftExportOutcome {
        target_modules,
        base_model_name_or_path,
        revision,
        use_rslora: manifest.use_rslora,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_synthetic_safetensors(tensors: &[(&str, &[u8])]) -> Vec<u8> {
        let mut header: BTreeMap<String, Value> = BTreeMap::new();
        let mut payload = Vec::new();
        for (name, bytes) in tensors {
            let start = payload.len();
            payload.extend_from_slice(bytes);
            let end = payload.len();
            header.insert(
                (*name).to_string(),
                json!({
                    "dtype": "F32",
                    "shape": [1, bytes.len() / 4],
                    "data_offsets": [start, end]
                }),
            );
        }
        let mut header_bytes = serde_json::to_vec(&header).unwrap();
        while !header_bytes.len().is_multiple_of(8) {
            header_bytes.push(b' ');
        }
        let mut out = (header_bytes.len() as u64).to_le_bytes().to_vec();
        out.extend_from_slice(&header_bytes);
        out.extend_from_slice(&payload);
        out
    }

    fn parse_safetensors_header(bytes: &[u8]) -> (BTreeMap<String, Value>, &[u8]) {
        assert!(bytes.len() >= 8);
        let hlen = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        assert_eq!(hlen % 8, 0, "header must be 8-byte aligned");
        let header: BTreeMap<String, Value> = serde_json::from_slice(&bytes[8..8 + hlen]).unwrap();
        (header, &bytes[8 + hlen..])
    }

    #[test]
    fn normalizes_hf_cache_snapshot_pinned_and_bare_ids_without_host_leak() {
        let sha = "15852e8c16360a2fea060d615a32b45270f8a8fc";
        let cache_path = format!(
            "{}/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/{sha}",
            concat!("/Us", "ers/alice")
        );
        assert_eq!(
            normalize_hf_repo_id_and_rev(&cache_path),
            ("Qwen/Qwen3.5-2B".to_string(), Some(sha.to_string()))
        );

        let win_cache = format!(
            "C:\\Users\\alice\\.cache\\huggingface\\hub\\models--Qwen--Qwen3.8-27B\\snapshots\\{sha}\\"
        );
        assert_eq!(
            normalize_hf_repo_id_and_rev(&win_cache),
            ("Qwen/Qwen3.8-27B".to_string(), Some(sha.to_string()))
        );

        assert_eq!(
            normalize_hf_repo_id_and_rev(&format!("Qwen/Qwen3-0.6B@{sha}")),
            ("Qwen/Qwen3-0.6B".to_string(), Some(sha.to_string()))
        );
        assert_eq!(
            normalize_hf_repo_id_and_rev("Qwen/Qwen3.5-4B"),
            ("Qwen/Qwen3.5-4B".to_string(), None)
        );

        let local_host = concat!("/Us", "ers/alice/models/custom-local-base");
        let (norm, rev) = normalize_hf_repo_id_and_rev(local_host);
        assert_eq!(norm, "custom-local-base");
        assert_eq!(rev, None);
        assert!(!norm.contains("alice"));
    }

    #[test]
    fn exports_peft_with_rslora_prefix_stripping_and_identical_payload_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let sha = "15852e8c16360a2fea060d615a32b45270f8a8fc";
        let snapshot_path = format!(
            "{}/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/{sha}",
            concat!("/Us", "ers/dev")
        );

        let manifest = json!({
            "rank": 16,
            "alpha": 8,
            "use_rslora": true,
            "base_model": snapshot_path,
            "base_key_map": {
                "l0.down": "model.language_model.layers.0.mlp.down_proj.weight",
                "l17.lin_qkv": "model.language_model.layers.17.linear_attn.in_proj_qkv.weight"
            }
        });
        let manifest_path = dir.path().join("adapter_manifest.json");
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let tensor_a = [1_u8, 2, 3, 4, 5, 6, 7, 8];
        let tensor_b = [9_u8, 10, 11, 12];
        let tensor_qkv_a = [13_u8, 14, 15, 16];
        let tensor_qkv_b = [17_u8, 18, 19, 20];
        let raw_st = build_synthetic_safetensors(&[
            ("l0.down.lora_a.weight", &tensor_a),
            ("l0.down.lora_b.weight", &tensor_b),
            ("l17.lin_qkv.lora_a.weight", &tensor_qkv_a),
            ("l17.lin_qkv.lora_b.weight", &tensor_qkv_b),
        ]);
        let (_, orig_payload) = parse_safetensors_header(&raw_st);
        let orig_payload = orig_payload.to_vec();

        let adapter_path = dir.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&adapter_path, &raw_st).unwrap();

        let out_stripped = dir.path().join("peft_stripped");
        let outcome =
            export_peft_from_candle(&adapter_path, &manifest_path, &out_stripped, true).unwrap();
        assert_eq!(outcome.base_model_name_or_path, "Qwen/Qwen3.5-2B");
        assert_eq!(outcome.revision.as_deref(), Some(sha));
        assert!(outcome.use_rslora);
        assert_eq!(
            outcome.target_modules,
            vec!["down_proj".to_string(), "in_proj_qkv".to_string()]
        );

        let peft_bytes = std::fs::read(out_stripped.join("adapter_model.safetensors")).unwrap();
        let (peft_header, peft_payload) = parse_safetensors_header(&peft_bytes);
        assert_eq!(peft_payload, orig_payload.as_slice());
        assert_eq!(peft_header["__metadata__"], json!({ "format": "pt" }));
        assert!(
            peft_header.contains_key("base_model.model.model.layers.0.mlp.down_proj.lora_A.weight")
        );
        assert!(
            peft_header.contains_key("base_model.model.model.layers.0.mlp.down_proj.lora_B.weight")
        );
        assert!(peft_header.contains_key(
            "base_model.model.model.layers.17.linear_attn.in_proj_qkv.lora_A.weight"
        ));
        assert!(peft_header.contains_key(
            "base_model.model.model.layers.17.linear_attn.in_proj_qkv.lora_B.weight"
        ));
        let off = peft_header["base_model.model.model.layers.0.mlp.down_proj.lora_A.weight"]
            ["data_offsets"]
            .as_array()
            .unwrap();
        let s = off[0].as_u64().unwrap() as usize;
        let e = off[1].as_u64().unwrap() as usize;
        assert_eq!(&peft_payload[s..e], &tensor_a);

        let cfg_raw = std::fs::read_to_string(out_stripped.join("adapter_config.json")).unwrap();
        assert!(!cfg_raw.contains("models--"));
        assert!(!cfg_raw.contains(concat!("/Us", "ers/")));
        let cfg: Value = serde_json::from_str(&cfg_raw).unwrap();
        assert_eq!(cfg["peft_type"], "LORA");
        assert_eq!(cfg["task_type"], "CAUSAL_LM");
        assert_eq!(cfg["base_model_name_or_path"], "Qwen/Qwen3.5-2B");
        assert_eq!(cfg["revision"], sha);
        assert_eq!(cfg["r"], 16);
        assert_eq!(cfg["lora_alpha"], 8);
        assert_eq!(cfg["use_rslora"], true);
        assert_eq!(cfg["lora_dropout"], 0.0);
        assert_eq!(cfg["bias"], "none");
        assert_eq!(cfg["fan_in_fan_out"], false);
        assert_eq!(cfg["inference_mode"], true);
        assert_eq!(cfg["target_modules"], json!(["down_proj", "in_proj_qkv"]));

        // Without stripping language_model prefix, nested prefix is preserved cleanly.
        let out_kept = dir.path().join("peft_kept");
        export_peft_from_candle(&adapter_path, &manifest_path, &out_kept, false).unwrap();
        let peft_kept_bytes = std::fs::read(out_kept.join("adapter_model.safetensors")).unwrap();
        let (kept_header, _) = parse_safetensors_header(&peft_kept_bytes);
        assert!(kept_header.contains_key(
            "base_model.model.model.language_model.layers.0.mlp.down_proj.lora_A.weight"
        ));
    }
}
