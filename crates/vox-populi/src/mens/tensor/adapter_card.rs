use anyhow::Result;
use std::path::{Path, PathBuf};
use vox_crypto::facades::{hex_encode, interop_sha256, secure_hash};

/// SHA-256 of zero bytes; a `train_data_sha256` equal to this means "hashed nothing".
const SHA256_OF_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Provenance manifest for a trained LoRA adapter. Written as `adapter_card.json`
/// alongside the adapter weights. Fail-closed: required fields must be non-empty.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterCard {
    /// HF repo id of the base model (e.g., "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18").
    pub base_hf_id: String,
    /// HF commit revision the adapter was trained against.
    pub base_revision: String,
    /// VRAM tier rung used for training (e.g., "qwen3_16g").
    pub base_rung: String,
    /// Quantization method (e.g., "qlora", "lora", "none").
    pub quantization: String,
    pub lora_rank: u32,
    pub lora_alpha: f32,
    pub seed: u64,
    pub corpus_hash: String,
    pub preset_version: String,
    pub metrics: serde_json::Value,
    pub cost_usd: f64,
    pub provider: String, // "local", "runpod", "vast"
    pub git_sha: String,
    pub created: String, // ISO-8601
    /// SHA-256 (via `vox_crypto::interop_sha256`) of the raw adapter file bytes; 64 lowercase hex.
    #[serde(default)]
    pub adapter_sha256: String,
    /// SHA-256 of the raw bytes of the train.jsonl the trainer consumed (the path recorded in
    /// the run's `training_manifest.json`); 64 lowercase hex. Not the xxh3 `corpus_hash`.
    #[serde(default)]
    pub train_data_sha256: String,
    /// BLAKE3 (`vox_crypto::secure_hash`) of the raw `training_manifest.json` bytes; 64 lowercase hex.
    #[serde(default)]
    pub training_manifest_digest: String,
    /// Fingerprint of the syntax the model was trained against (see `syntax_fingerprint_kind`).
    #[serde(default)]
    pub training_syntax_fingerprint: String,
    /// What `training_syntax_fingerprint` is: `"ebnf-sha256"` until the Phase 13 behavioral fingerprint.
    #[serde(default)]
    pub syntax_fingerprint_kind: String,
}

/// Lowercase-hex SHA-256/BLAKE3-sized digest check (64 chars, no uppercase).
fn is_lower_hex_64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// SHA-256 over the raw file bytes (no decoding, no newline normalization), lowercase hex.
pub fn sha256_file_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("cannot read {} for hashing: {e}", path.display()))?;
    Ok(hex_encode(&interop_sha256(&bytes)))
}

const ADAPTER_FILE: &str = "candle_qlora_adapter.safetensors";

/// `<run>/candle_qlora_adapter.safetensors`, else `<run>/final/candle_qlora_adapter.safetensors`.
pub fn locate_adapter(run_dir: &Path) -> Option<PathBuf> {
    [
        run_dir.join(ADAPTER_FILE),
        run_dir.join("final").join(ADAPTER_FILE),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// What the trainer demonstrably consumed, measured host-side right after success.
#[derive(Debug, Clone)]
pub struct TrainingLineage {
    pub adapter_path: PathBuf,
    pub adapter_sha256: String,
    pub train_file: PathBuf,
    pub train_data_sha256: String,
    pub training_manifest_digest: String,
    pub base_model: Option<String>,
}

/// Hash the adapter and the train file named by the run's `training_manifest.json`.
///
/// The manifest is read next to the adapter (falling back to `run_dir`); a relative
/// `train_file` resolves against `workspace_root`.
pub fn read_training_lineage(
    run_dir: &Path,
    workspace_root: Option<&Path>,
) -> Result<TrainingLineage> {
    let adapter_path = locate_adapter(run_dir)
        .ok_or_else(|| anyhow::anyhow!("no {ADAPTER_FILE} under {}", run_dir.display()))?;
    let manifest_path = [adapter_path.parent().unwrap_or(run_dir), run_dir]
        .iter()
        .map(|d| d.join("training_manifest.json"))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            anyhow::anyhow!("no training_manifest.json for {}", adapter_path.display())
        })?;
    let bytes = std::fs::read(&manifest_path)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("{} is not valid JSON: {e}", manifest_path.display()))?;
    let train_file = v
        .get("train_file")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{} has no train_file", manifest_path.display()))?;
    let mut train_path = PathBuf::from(train_file);
    if train_path.is_relative()
        && let Some(root) = workspace_root
    {
        train_path = root.join(train_path);
    }
    if !train_path.is_file() {
        anyhow::bail!("train file {} does not exist", train_path.display());
    }
    Ok(TrainingLineage {
        adapter_sha256: sha256_file_hex(&adapter_path)?,
        train_data_sha256: sha256_file_hex(&train_path)?,
        training_manifest_digest: hex_encode(&secure_hash(&bytes)),
        base_model: v
            .get("base_model")
            .and_then(|b| b.as_str())
            .map(String::from),
        adapter_path,
        train_file: train_path,
    })
}

/// Outcome of [`check_for_eval`]; a `mens/*` label is allowed only when `problems` is empty.
#[derive(Debug, Clone)]
pub struct EvalCardCheck {
    pub card: Option<AdapterCard>,
    pub problems: Vec<String>,
    /// `Some(true)` when the card's training fingerprint or kind differs from the current one.
    pub syntax_drift: Option<bool>,
}

/// Single eval-side gate: refuse any run whose adapter cannot be tied by hash to a valid card.
///
/// Sub-checkpoint directories (`checkpoint-*`) carry no card of their own and are refused.
/// Syntax drift is recorded, not refused.
pub fn check_for_eval(
    run_dir: &Path,
    current_fingerprint: &str,
    current_kind: &str,
) -> EvalCardCheck {
    let mut problems = Vec::new();
    let refuse = |problems: Vec<String>| EvalCardCheck {
        card: None,
        problems,
        syntax_drift: None,
    };
    let Some(adapter) = locate_adapter(run_dir) else {
        problems.push(format!("no {ADAPTER_FILE} under {}", run_dir.display()));
        return refuse(problems);
    };
    let card = match AdapterCard::read_sidecar(&adapter) {
        Ok(Some(c)) => c,
        Ok(None) => {
            problems.push(format!(
                "no adapter_card.json next to {}",
                adapter.display()
            ));
            return refuse(problems);
        }
        Err(e) => {
            problems.push(format!("adapter_card.json unreadable: {e}"));
            return refuse(problems);
        }
    };
    if let Err(e) = card.validate() {
        problems.push(format!("card invalid: {e}"));
    }
    if let Err(e) = card.validate_lineage() {
        problems.push(format!("card lineage invalid: {e}"));
    }
    match sha256_file_hex(&adapter) {
        Ok(h) if h == card.adapter_sha256 => {}
        Ok(h) => problems.push(format!(
            "adapter_sha256 mismatch: card {} vs file {h}",
            card.adapter_sha256
        )),
        Err(e) => problems.push(format!("adapter not hashable: {e}")),
    }
    let drift = card.training_syntax_fingerprint != current_fingerprint
        || card.syntax_fingerprint_kind != current_kind;
    EvalCardCheck {
        card: Some(card),
        problems,
        syntax_drift: Some(drift),
    }
}

impl AdapterCard {
    /// Fail-closed: returns Err if base_rung, quantization, or base_revision are empty.
    pub fn validate(&self) -> Result<()> {
        if self.base_rung.is_empty() {
            anyhow::bail!("AdapterCard.base_rung must not be empty");
        }
        if self.quantization.is_empty() {
            anyhow::bail!("AdapterCard.quantization must not be empty");
        }
        if self.base_revision.is_empty() {
            anyhow::bail!("AdapterCard.base_revision must not be empty");
        }
        Ok(())
    }

    /// Fail-closed lineage check for eval: every lineage field present, digests 64 lowercase
    /// hex, and `train_data_sha256` not the hash of empty input. Legacy cards fail this.
    pub fn validate_lineage(&self) -> Result<()> {
        for (name, v) in [
            ("base_hf_id", &self.base_hf_id),
            ("base_revision", &self.base_revision),
            ("corpus_hash", &self.corpus_hash),
            (
                "training_syntax_fingerprint",
                &self.training_syntax_fingerprint,
            ),
            ("syntax_fingerprint_kind", &self.syntax_fingerprint_kind),
        ] {
            if v.is_empty() {
                anyhow::bail!("AdapterCard.{name} must not be empty");
            }
        }
        for (name, v) in [
            ("adapter_sha256", &self.adapter_sha256),
            ("train_data_sha256", &self.train_data_sha256),
            ("training_manifest_digest", &self.training_manifest_digest),
        ] {
            if !is_lower_hex_64(v) {
                anyhow::bail!("AdapterCard.{name} must be 64 lowercase hex characters");
            }
        }
        if self.train_data_sha256 == SHA256_OF_EMPTY {
            anyhow::bail!("AdapterCard.train_data_sha256 is the hash of empty input");
        }
        Ok(())
    }

    /// Returns false if rung or quantization do not match this card (serve-side check).
    pub fn is_compatible_with(&self, serve_rung: &str, serve_quant: &str) -> bool {
        self.base_rung == serve_rung && self.quantization == serve_quant
    }

    /// Write this card as `adapter_card.json` adjacent to `adapter_path`.
    pub fn write_sidecar(&self, adapter_path: &std::path::Path) -> Result<()> {
        let sidecar = adapter_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("adapter path has no parent"))?
            .join("adapter_card.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&sidecar, json)?;
        Ok(())
    }

    /// Read card from `adapter_card.json` adjacent to `adapter_path`. Returns None if absent.
    pub fn read_sidecar(adapter_path: &std::path::Path) -> Result<Option<Self>> {
        let sidecar = adapter_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("adapter path has no parent"))?
            .join("adapter_card.json");
        if !sidecar.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&sidecar)?;
        Ok(Some(serde_json::from_str(&json)?))
    }

    /// Convenience constructor for tests (non-empty required fields).
    pub fn for_test(rung: &str, quant: &str) -> Self {
        Self {
            base_hf_id: "test/model@abc123".to_string(),
            base_revision: "abc123".to_string(),
            base_rung: rung.to_string(),
            quantization: quant.to_string(),
            lora_rank: 16,
            lora_alpha: 32.0,
            seed: 42,
            corpus_hash: "test_hash".to_string(),
            preset_version: "1".to_string(),
            metrics: serde_json::Value::Null,
            cost_usd: 0.0,
            provider: "local".to_string(),
            git_sha: "abc".to_string(),
            created: "2026-06-21T00:00:00Z".to_string(),
            adapter_sha256: String::new(),
            train_data_sha256: String::new(),
            training_manifest_digest: String::new(),
            training_syntax_fingerprint: String::new(),
            syntax_fingerprint_kind: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_rung() {
        let mut card = AdapterCard::for_test("", "qlora");
        card.base_revision = "abc".to_string();
        assert!(
            card.validate().is_err(),
            "empty base_rung must fail validate"
        );
    }

    #[test]
    fn validate_rejects_empty_quantization() {
        let card = AdapterCard::for_test("qwen3_16g", "");
        assert!(
            card.validate().is_err(),
            "empty quantization must fail validate"
        );
    }

    #[test]
    fn validate_rejects_empty_revision() {
        let mut card = AdapterCard::for_test("qwen3_16g", "qlora");
        card.base_revision = "".to_string();
        assert!(
            card.validate().is_err(),
            "empty base_revision must fail validate"
        );
    }

    #[test]
    fn validate_passes_with_all_required_fields() {
        let card = AdapterCard::for_test("qwen3_16g", "qlora");
        card.validate().unwrap();
    }

    #[test]
    fn is_compatible_with_matches_rung_and_quant() {
        let card = AdapterCard::for_test("qwen3_16g", "qlora");
        assert!(card.is_compatible_with("qwen3_16g", "qlora"));
    }

    #[test]
    fn is_compatible_with_rejects_rung_mismatch() {
        let card = AdapterCard::for_test("qwen3_16g", "qlora");
        assert!(!card.is_compatible_with("qwen3_24g", "qlora"));
    }

    #[test]
    fn is_compatible_with_rejects_quant_mismatch() {
        let card = AdapterCard::for_test("qwen3_16g", "qlora");
        assert!(!card.is_compatible_with("qwen3_16g", "lora"));
    }

    #[test]
    fn sidecar_round_trip() {
        let tmp = std::env::temp_dir().join("vox_adapter_card_sidecar_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let adapter_path = tmp.join("adapter_model.safetensors");
        std::fs::write(&adapter_path, b"fake").unwrap();

        let card = AdapterCard::for_test("qwen3_16g", "qlora");
        card.write_sidecar(&adapter_path).unwrap();

        let loaded = AdapterCard::read_sidecar(&adapter_path).unwrap().unwrap();
        assert_eq!(loaded.base_rung, "qwen3_16g");
        assert_eq!(loaded.quantization, "qlora");
        assert_eq!(loaded.base_revision, "abc123");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn read_sidecar_returns_none_when_absent() {
        let tmp = std::env::temp_dir().join("vox_adapter_card_no_sidecar_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let adapter_path = tmp.join("adapter_model.safetensors");
        std::fs::write(&adapter_path, b"fake").unwrap();

        let result = AdapterCard::read_sidecar(&adapter_path).unwrap();
        assert!(result.is_none());

        std::fs::remove_dir_all(&tmp).ok();
    }

    // ---- lineage / eval-gate tests (12-06) ----

    const GOOD_HEX: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

    fn lineage_card() -> AdapterCard {
        let mut c = AdapterCard::for_test("qwen3_16g", "qlora");
        c.adapter_sha256 = GOOD_HEX.into();
        c.train_data_sha256 = GOOD_HEX.into();
        c.training_manifest_digest = GOOD_HEX.into();
        c.training_syntax_fingerprint = "fp".into();
        c.syntax_fingerprint_kind = "ebnf-sha256".into();
        c
    }

    /// Run dir holding an adapter and a card whose adapter_sha256 matches it.
    fn run_dir_with_card() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let adapter = d.path().join(ADAPTER_FILE);
        std::fs::write(&adapter, b"weights-v1").unwrap();
        let mut c = lineage_card();
        c.adapter_sha256 = sha256_file_hex(&adapter).unwrap();
        c.write_sidecar(&adapter).unwrap();
        (d, adapter)
    }

    #[test]
    fn check_for_eval_refuses_missing_adapter_and_card() {
        let d = tempfile::tempdir().unwrap();
        let r = check_for_eval(d.path(), "fp", "ebnf-sha256");
        assert!(r.card.is_none());
        assert!(r.problems.iter().any(|p| p.contains(ADAPTER_FILE)));

        std::fs::write(d.path().join(ADAPTER_FILE), b"w").unwrap();
        let r = check_for_eval(d.path(), "fp", "ebnf-sha256");
        assert!(r.card.is_none());
        assert!(r.problems.iter().any(|p| p.contains("adapter_card.json")));
    }

    #[test]
    fn check_for_eval_accepts_a_valid_card() {
        let (d, _) = run_dir_with_card();
        let r = check_for_eval(d.path(), "fp", "ebnf-sha256");
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(r.syntax_drift, Some(false));
    }

    #[test]
    fn swapped_adapter_is_refused() {
        let (d, adapter) = run_dir_with_card();
        std::fs::write(&adapter, b"weights-v2-swapped").unwrap();
        let r = check_for_eval(d.path(), "fp", "ebnf-sha256");
        assert!(
            r.problems
                .iter()
                .any(|p| p.contains("adapter_sha256 mismatch")),
            "{:?}",
            r.problems
        );
    }

    #[test]
    fn fingerprint_change_is_drift_not_refusal() {
        let (d, _) = run_dir_with_card();
        let r = check_for_eval(d.path(), "other-fp", "ebnf-sha256");
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(r.syntax_drift, Some(true));
        let r = check_for_eval(d.path(), "fp", "behavioral");
        assert_eq!(r.syntax_drift, Some(true));
    }

    #[test]
    fn legacy_card_parses_but_fails_lineage() {
        let legacy = r#"{"base_hf_id":"m@abc","base_revision":"abc","base_rung":"r","quantization":"qlora",
            "lora_rank":16,"lora_alpha":32.0,"seed":1,"corpus_hash":"h","preset_version":"1",
            "metrics":null,"cost_usd":0.0,"provider":"local","git_sha":"g","created":"2026-01-01T00:00:00Z"}"#;
        let c: AdapterCard = serde_json::from_str(legacy).unwrap();
        c.validate().unwrap();
        assert!(c.validate_lineage().is_err());
    }

    #[test]
    fn validate_lineage_edge_cases() {
        lineage_card().validate_lineage().unwrap();
        let cases: Vec<(&str, Box<dyn Fn(&mut AdapterCard)>)> = vec![
            ("empty base_hf_id", Box::new(|c| c.base_hf_id.clear())),
            ("empty base_revision", Box::new(|c| c.base_revision.clear())),
            ("empty corpus_hash", Box::new(|c| c.corpus_hash.clear())),
            (
                "empty adapter_sha256",
                Box::new(|c| c.adapter_sha256.clear()),
            ),
            (
                "empty train_data_sha256",
                Box::new(|c| c.train_data_sha256.clear()),
            ),
            (
                "empty manifest digest",
                Box::new(|c| c.training_manifest_digest.clear()),
            ),
            (
                "empty fingerprint",
                Box::new(|c| c.training_syntax_fingerprint.clear()),
            ),
            (
                "empty kind",
                Box::new(|c| c.syntax_fingerprint_kind.clear()),
            ),
            (
                "uppercase hex",
                Box::new(|c| c.adapter_sha256 = GOOD_HEX.to_uppercase()),
            ),
            (
                "63 chars",
                Box::new(|c| c.adapter_sha256 = GOOD_HEX[..63].into()),
            ),
            (
                "non-hex char",
                Box::new(|c| c.adapter_sha256 = format!("{}z", &GOOD_HEX[..63])),
            ),
            (
                "sha256 of empty input",
                Box::new(|c| c.train_data_sha256 = SHA256_OF_EMPTY.into()),
            ),
        ];
        for (name, mutate) in cases {
            let mut c = lineage_card();
            mutate(&mut c);
            assert!(c.validate_lineage().is_err(), "{name} must be rejected");
        }
    }
}
