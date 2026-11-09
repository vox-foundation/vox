//! Append-only per-item checkpoint shared by `eval-reasoning-compare`,
//! `eval-local` and `eval-collateral-damage`.
//!
//! One JSONL file per scored run. Line 1 is a header naming every input that
//! determines a generation (model identity, scorer, decode settings, the ordered
//! item set) and its BLAKE3 `key`. Each later line is one generation:
//! `{"item": <hash of the prompt request>, "response": <raw backend response>}`,
//! written and fsynced before the next generation starts.
//!
//! On reopen, recorded generations are reused, so a resumed run makes the same
//! grading decisions on the same text as an uninterrupted one. A header whose key
//! differs is refused unless the caller asks to discard the file. A torn last
//! line (a kill mid-write) is dropped and that item is generated again; a corrupt
//! line anywhere else is refused.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Value, json};

pub(crate) const ITEM_CHECKPOINT_SCHEMA: &str = "vox_eval_item_checkpoint.v1";

/// Where `eval-reasoning-compare` and `eval-collateral-damage` keep reasoning
/// generations (relative to the repo root), one directory per model. Shared, so
/// collateral reuses the compare's adapter generations for the same slice.
pub(crate) const REASONING_CHECKPOINT_ROOT: &str = "mens/runs/reasoning_compare/checkpoints";

/// Names the reasoning prompt builder and grader in every reasoning header.
const REASONING_SCORER: &str =
    "reasoning_eval::score_loaded thinking_mode_prompt grade_final_answer v1";

/// Side restart flags for reasoning evaluation.
///
/// An adapter restart (`--force-restart`) discards only the adapter's checkpoint
/// generations; base generations are discarded only by `--force-restart-base`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ForceRestart {
    pub adapter: bool,
    pub base: bool,
}

impl ForceRestart {
    #[inline]
    pub fn for_adapter(&self) -> bool {
        self.adapter
    }

    #[inline]
    pub fn for_base(&self) -> bool {
        self.base
    }
}

/// Inputs that determine the reasoning generations for `model`, `bench`, `items` and `settings`.
pub(crate) fn reasoning_inputs(
    model: &Value,
    bench: &str,
    items: &[vox_eval::reasoning_bench::ReasoningItem],
    settings: &vox_eval::decode_protocol::DecodeSettings,
) -> Value {
    debug_assert_eq!(settings.temperature, 0.0);
    debug_assert_eq!(settings.seed, 0);
    let item_set: Vec<Value> = items
        .iter()
        .map(|i| json!([i.id, i.question, i.answer]))
        .collect();
    let mut inputs = json!({
        "scorer": REASONING_SCORER,
        "model": model,
        "bench": bench,
        "max_tokens": settings.max_tokens,
        "temperature": 0.0,
        "seed": 0,
        "items_blake3": hash_json(&Value::Array(item_set)),
    });
    if let Some(obj) = inputs.as_object_mut() {
        for (k, v) in settings.checkpoint_protocol_fields() {
            obj.insert(k.to_string(), v);
        }
    }
    inputs
}

/// The checkpoint for one model on one reasoning slice under `root`.
pub(crate) fn reasoning_checkpoint(
    root: &Path,
    model_dir: &Path,
    bench: &str,
    items: &[vox_eval::reasoning_bench::ReasoningItem],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
) -> Result<ItemCheckpoint> {
    let model = model_identity(model_dir)?;
    let inputs = reasoning_inputs(&model, bench, items, settings);
    let model_dir = root.join(&hash_json(&model)[..16]);
    let path = reasoning_checkpoint_path(&model_dir, bench, &inputs);
    let mut ck = ItemCheckpoint::open(&path, &inputs, force_restart)?;
    if !force_restart {
        ck.seed = generations_for_other_item_sets(&model_dir, &inputs, &path);
    }
    Ok(ck)
}

/// Generations recorded in `model_dir` by files whose inputs equal `inputs` in
/// everything but the item set (same model, bench, scorer and decode settings).
/// A grown slice reuses the text its unchanged items were already given: a line
/// is keyed by the hash of its own prompt request, so only identical requests match.
fn generations_for_other_item_sets(
    model_dir: &Path,
    inputs: &Value,
    own: &Path,
) -> HashMap<String, String> {
    let settings = |v: &Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("items_blake3");
        }
        v
    };
    let want = settings(inputs);
    let mut seed = HashMap::new();
    let Ok(dir) = std::fs::read_dir(model_dir) else {
        return seed;
    };
    for path in dir.flatten().map(|e| e.path()) {
        if path == own || header(&path).is_none_or(|h| settings(&h["inputs"]) != want) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for v in text
            .lines()
            .skip(1)
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        {
            if let (Some(item), Some(resp)) = (v["item"].as_str(), v["response"].as_str()) {
                seed.insert(item.to_string(), resp.to_string());
            }
        }
    }
    seed
}

/// The file for `inputs` in a model's checkpoint directory. Each set of decode
/// settings and items gets its own file (`<bench>-<key16>.jsonl`), so changing
/// the token cap never makes another setting's generations "for different
/// inputs" and `--force-restart` only ever discards this setting's file. A
/// legacy `<bench>.jsonl` (written before files were named by key) is used when
/// its header key matches, and is otherwise left untouched.
fn reasoning_checkpoint_path(model_dir: &Path, bench: &str, inputs: &Value) -> PathBuf {
    let key = hash_json(inputs);
    let legacy = model_dir.join(format!("{bench}.jsonl"));
    if header_key(&legacy).as_deref() == Some(key.as_str()) {
        return legacy;
    }
    model_dir.join(format!("{bench}-{}.jsonl", &key[..16]))
}

/// A checkpoint file's header line, if it has a readable one.
fn header(path: &Path) -> Option<Value> {
    use std::io::BufRead;
    let mut line = String::new();
    std::io::BufReader::new(std::fs::File::open(path).ok()?)
        .read_line(&mut line)
        .ok()?;
    let v: Value = serde_json::from_str(line.trim_end()).ok()?;
    (v["schema"] == ITEM_CHECKPOINT_SCHEMA).then_some(v)
}

/// The `key` in a checkpoint file's header line, if it has a readable one.
fn header_key(path: &Path) -> Option<String> {
    header(path)?["key"].as_str().map(str::to_string)
}

/// BLAKE3 hex of the compact JSON text of `v`. Object keys are preserved in
/// insertion order because serde_json's `preserve_order` feature is enabled workspace-wide.
pub(crate) fn hash_json(v: &Value) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(v.to_string().as_bytes()))
}

/// How the inference plugins serve an adapter. Part of every adapter identity, so
/// generations from an older serving path are never reused. v2 serves NF4(W) plus
/// the full-precision LoRA arm, exactly as training computed it; v1 (no field)
/// folded the delta in first and quantized NF4(W+ΔW), which lost most of it.
pub(crate) const ADAPTER_SERVING: &str = "nf4_base_plus_lora_arm v2";

/// What a model directory is, for a checkpoint header. An adapter run directory
/// is named by the BLAKE3 of its adapter weights and manifest, so a retrained
/// adapter at the same path never reuses old generations. Anything else (a base
/// snapshot) is named by its canonical path, which in the Hugging Face cache
/// layout carries the pinned revision.
pub(crate) fn model_identity(dir: &Path) -> Result<Value> {
    let weights = dir.join("candle_qlora_adapter.safetensors");
    if weights.is_file() {
        let bytes = std::fs::read(&weights)
            .with_context(|| format!("reading {} for its hash", weights.display()))?;
        let manifest = std::fs::read(dir.join("adapter_manifest.json")).unwrap_or_default();
        return Ok(json!({
            "adapter_weights_blake3": vox_crypto::hex_encode(&vox_crypto::secure_hash(&bytes)),
            "adapter_manifest_blake3": vox_crypto::hex_encode(&vox_crypto::secure_hash(&manifest)),
            "adapter_serving": ADAPTER_SERVING,
        }));
    }
    let canonical = dir
        .canonicalize()
        .with_context(|| format!("resolving model dir {}", dir.display()))?;
    Ok(json!({ "snapshot": canonical.display().to_string() }))
}

/// An open checkpoint: the generations already recorded, and the file to append to.
pub(crate) struct ItemCheckpoint {
    path: PathBuf,
    file: std::fs::File,
    done: HashMap<String, String>,
    /// Generations from files for other item sets (see
    /// [`generations_for_other_item_sets`]); copied into this file when used.
    seed: HashMap<String, String>,
}

impl ItemCheckpoint {
    /// Open or create the checkpoint at `path` for `inputs`. `force_restart`
    /// discards any existing file first.
    pub(crate) fn open(path: &Path, inputs: &Value, force_restart: bool) -> Result<Self> {
        let key = hash_json(inputs);
        if force_restart {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| format!("removing {}", path.display())),
            }
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };

        let mut done = HashMap::new();
        let mut keep = 0usize;
        let mut header_seen = false;
        let mut offset = 0usize;
        for (n, raw) in bytes.split_inclusive(|b| *b == b'\n').enumerate() {
            let is_last = offset + raw.len() == bytes.len();
            let parsed = raw
                .strip_suffix(b"\n")
                .and_then(|line| serde_json::from_slice::<Value>(line).ok());
            let Some(v) = parsed else {
                if is_last {
                    // A kill mid-write: drop the torn line, generate that item again.
                    break;
                }
                anyhow::bail!(
                    "checkpoint {} line {} is corrupt and is not the last line; refusing to \
                     reuse it (pass --force-restart to discard it)",
                    path.display(),
                    n + 1
                );
            };
            if !header_seen {
                if v["schema"] != ITEM_CHECKPOINT_SCHEMA || v["key"] != key.as_str() {
                    anyhow::bail!(
                        "checkpoint {} was written for different inputs (key {} , this run {key}); \
                         refusing to reuse it (pass --force-restart to discard it)",
                        path.display(),
                        v["key"]
                    );
                }
                header_seen = true;
            } else {
                match (v["item"].as_str(), v["response"].as_str()) {
                    (Some(item), Some(resp)) => {
                        done.insert(item.to_string(), resp.to_string());
                    }
                    _ => anyhow::bail!(
                        "checkpoint {} line {} has no item/response; refusing to reuse it \
                         (pass --force-restart to discard it)",
                        path.display(),
                        n + 1
                    ),
                }
            }
            offset += raw.len();
            keep = offset;
        }

        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("opening {}", path.display()))?;
        file.set_len(keep as u64)?;
        let mut ck = ItemCheckpoint {
            path: path.to_path_buf(),
            file,
            done,
            seed: HashMap::new(),
        };
        if keep == 0 {
            ck.append(&json!({ "schema": ITEM_CHECKPOINT_SCHEMA, "key": key, "inputs": inputs }))?;
        }
        Ok(ck)
    }

    /// Number of generations already recorded.
    pub(crate) fn len(&self) -> usize {
        self.done.len()
    }

    fn append(&mut self, line: &Value) -> Result<()> {
        use std::io::Seek;
        self.file.seek(std::io::SeekFrom::End(0))?;
        let mut text = line.to_string();
        text.push('\n');
        self.file
            .write_all(text.as_bytes())
            .and_then(|()| self.file.sync_data())
            .with_context(|| format!("appending to {}", self.path.display()))
    }

    /// The response for `prompt_json`: the recorded one, or `inner`'s, which is
    /// recorded (and fsynced) before it is returned. An `inner` error is
    /// returned and not recorded, so that item is tried again next run.
    pub(crate) fn infer(
        &mut self,
        prompt_json: &str,
        inner: &mut dyn FnMut(&str) -> Result<String>,
    ) -> Result<String> {
        let item = item_hash(prompt_json);
        if let Some(resp) = self.done.get(&item) {
            return Ok(resp.clone());
        }
        let resp = match self.seed.remove(&item) {
            Some(resp) => resp,
            None => inner(prompt_json)?,
        };
        self.record(prompt_json, &resp)?;
        Ok(resp)
    }

    /// Whether `prompt_json` already has a generation here (recorded or seeded),
    /// so [`Self::infer`] would not call its generator for it.
    pub(crate) fn has(&self, prompt_json: &str) -> bool {
        let item = item_hash(prompt_json);
        self.done.contains_key(&item) || self.seed.contains_key(&item)
    }

    /// Return the recorded (or seeded) generation text for `prompt_json` if present.
    pub(crate) fn get(&self, prompt_json: &str) -> Option<&str> {
        let item = item_hash(prompt_json);
        self.done
            .get(&item)
            .or_else(|| self.seed.get(&item))
            .map(String::as_str)
    }

    /// Record (and fsync) `resp` as the generation for `prompt_json`, exactly as
    /// [`Self::infer`] records one it generated.
    pub(crate) fn record(&mut self, prompt_json: &str, resp: &str) -> Result<()> {
        let item = item_hash(prompt_json);
        self.append(&json!({ "item": item, "response": resp }))?;
        self.done.insert(item, resp.to_string());
        Ok(())
    }
}

/// A generation's line key: the BLAKE3 of its exact prompt request text.
fn item_hash(prompt_json: &str) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(prompt_json.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(max_tokens: usize) -> Value {
        json!({ "model": {"snapshot": "/x"}, "max_tokens": max_tokens, "items": "abc" })
    }

    /// An inner generator that answers `resp-<prompt>` and counts its calls.
    fn counting(calls: &mut usize) -> impl FnMut(&str) -> Result<String> + '_ {
        move |p: &str| {
            *calls += 1;
            Ok(format!("resp-{p}"))
        }
    }

    #[test]
    fn a_reopened_checkpoint_skips_items_already_scored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ck.jsonl");
        let mut calls = 0;
        {
            let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
            let mut inner = counting(&mut calls);
            for p in ["a", "b"] {
                ck.infer(p, &mut inner).unwrap();
            }
        }
        assert_eq!(calls, 2);
        let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
        assert_eq!(ck.len(), 2);
        let mut calls2 = 0;
        let mut inner = counting(&mut calls2);
        assert_eq!(ck.infer("a", &mut inner).unwrap(), "resp-a");
        assert_eq!(ck.infer("c", &mut inner).unwrap(), "resp-c");
        drop(inner);
        assert_eq!(calls2, 1, "only the new item is generated");
    }

    #[test]
    fn a_checkpoint_for_other_inputs_is_refused_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ck.jsonl");
        {
            let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
            let mut calls = 0;
            ck.infer("a", &mut counting(&mut calls)).unwrap();
        }
        let err = ItemCheckpoint::open(&path, &inputs(512), false)
            .err()
            .expect("mismatch must be refused");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("different inputs") && msg.contains("--force-restart"),
            "{msg}"
        );
        // The refused file is untouched.
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);

        let ck = ItemCheckpoint::open(&path, &inputs(512), true).unwrap();
        assert_eq!(ck.len(), 0, "force-restart starts from an empty checkpoint");
    }

    #[test]
    fn a_torn_last_line_is_dropped_and_that_item_scored_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ck.jsonl");
        {
            let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
            let mut calls = 0;
            ck.infer("a", &mut counting(&mut calls)).unwrap();
        }
        // A kill mid-write of item "b".
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"{\"item\":\"deadbeef\",\"resp").unwrap();
        drop(f);

        let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
        assert_eq!(ck.len(), 1);
        let mut calls = 0;
        let mut inner = counting(&mut calls);
        ck.infer("b", &mut inner).unwrap();
        drop(inner);
        assert_eq!(calls, 1);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 3, "{text}");
        assert!(
            text.lines()
                .all(|l| serde_json::from_str::<Value>(l).is_ok()),
            "{text}"
        );
    }

    #[test]
    fn a_corrupt_line_before_the_end_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ck.jsonl");
        {
            let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
            let mut calls = 0;
            ck.infer("a", &mut counting(&mut calls)).unwrap();
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines: Vec<&str> = text.lines().collect();
        lines.insert(1, "not json");
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let err = ItemCheckpoint::open(&path, &inputs(2048), false)
            .err()
            .expect("corrupt middle line");
        assert!(format!("{err:#}").contains("line 2 is corrupt"), "{err:#}");
    }

    #[test]
    fn an_inner_error_is_not_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ck.jsonl");
        let mut ck = ItemCheckpoint::open(&path, &inputs(2048), false).unwrap();
        let mut failing = |_: &str| -> Result<String> { anyhow::bail!("gpu fault") };
        assert!(ck.infer("a", &mut failing).is_err());
        assert_eq!(ck.len(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
    }

    fn reasoning_items() -> Vec<vox_eval::reasoning_bench::ReasoningItem> {
        reasoning_items_n(3)
    }

    fn reasoning_items_n(n: usize) -> Vec<vox_eval::reasoning_bench::ReasoningItem> {
        (0..n)
            .map(|i| vox_eval::reasoning_bench::ReasoningItem {
                id: format!("i{i}"),
                question: format!("q{i}"),
                answer: format!("{i}"),
            })
            .collect()
    }

    /// A checkpoint root and an adapter dir.
    fn reasoning_fixture() -> (tempfile::TempDir, tempfile::TempDir) {
        let adapter = tempfile::tempdir().unwrap();
        std::fs::write(
            adapter.path().join("candle_qlora_adapter.safetensors"),
            b"w",
        )
        .unwrap();
        (tempfile::tempdir().unwrap(), adapter)
    }

    fn reasoning_ck(root: &Path, model: &Path, max_tokens: usize, force: bool) -> ItemCheckpoint {
        let settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        reasoning_checkpoint(
            root,
            model,
            "gsm8k_think",
            &reasoning_items(),
            &settings,
            force,
        )
        .unwrap()
    }

    #[test]
    fn a_new_token_cap_gets_its_own_file_and_keeps_the_old_generations() {
        let (root, adapter) = reasoning_fixture();
        let mut calls = 0;
        reasoning_ck(root.path(), adapter.path(), 2048, false)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        // Before files were named by key, this open was refused as "different
        // inputs" and the only way on was --force-restart, deleting the 2048 file.
        let other = reasoning_ck(root.path(), adapter.path(), 4096, false);
        assert_eq!(other.len(), 0);
        drop(other);
        assert_eq!(
            reasoning_ck(root.path(), adapter.path(), 2048, false).len(),
            1
        );
    }

    #[test]
    fn force_restart_discards_only_its_own_settings() {
        let (root, adapter) = reasoning_fixture();
        let mut calls = 0;
        reasoning_ck(root.path(), adapter.path(), 2048, false)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        reasoning_ck(root.path(), adapter.path(), 4096, false)
            .infer("b", &mut counting(&mut calls))
            .unwrap();
        assert_eq!(
            reasoning_ck(root.path(), adapter.path(), 4096, true).len(),
            0
        );
        assert_eq!(
            reasoning_ck(root.path(), adapter.path(), 2048, false).len(),
            1,
            "a 4096 restart must not discard the 2048 generations"
        );
    }

    #[test]
    fn a_legacy_file_with_a_matching_key_is_reused_in_place() {
        let (root, adapter) = reasoning_fixture();
        let mut calls = 0;
        reasoning_ck(root.path(), adapter.path(), 2048, false)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        // Rename the keyed file to the pre-335 name.
        let model_dir = std::fs::read_dir(root.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let keyed = std::fs::read_dir(&model_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let legacy = model_dir.join("gsm8k_think.jsonl");
        std::fs::rename(&keyed, &legacy).unwrap();

        assert_eq!(
            reasoning_ck(root.path(), adapter.path(), 2048, false).len(),
            1
        );
        assert_eq!(
            std::fs::read_dir(&model_dir).unwrap().count(),
            1,
            "legacy used in place"
        );
        // A legacy file for other settings is neither used nor touched.
        let before = std::fs::read(&legacy).unwrap();
        assert_eq!(
            reasoning_ck(root.path(), adapter.path(), 4096, true).len(),
            0
        );
        assert_eq!(std::fs::read(&legacy).unwrap(), before);
    }

    fn grown_ck(
        root: &Path,
        model: &Path,
        n: usize,
        max_tokens: usize,
        force: bool,
    ) -> ItemCheckpoint {
        let settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        reasoning_checkpoint(
            root,
            model,
            "gsm8k_think",
            &reasoning_items_n(n),
            &settings,
            force,
        )
        .unwrap()
    }

    /// Phase 335: growing a slice from 32 to 128 items must not regenerate the 32.
    #[test]
    fn a_grown_item_set_reuses_the_generations_of_its_prefix() {
        let (root, adapter) = reasoning_fixture();
        let mut calls = 0;
        {
            let mut ck = grown_ck(root.path(), adapter.path(), 2, 2048, false);
            let mut inner = counting(&mut calls);
            for p in ["a", "b"] {
                ck.infer(p, &mut inner).unwrap();
            }
        }
        let mut calls2 = 0;
        {
            let mut ck = grown_ck(root.path(), adapter.path(), 3, 2048, false);
            let mut inner = counting(&mut calls2);
            for p in ["a", "b", "c"] {
                assert_eq!(ck.infer(p, &mut inner).unwrap(), format!("resp-{p}"));
            }
        }
        assert_eq!(calls2, 1, "only the new item is generated");
        // The grown file is self-contained, and the prefix file is untouched.
        assert_eq!(
            grown_ck(root.path(), adapter.path(), 3, 2048, false).len(),
            3
        );
        assert_eq!(
            grown_ck(root.path(), adapter.path(), 2, 2048, false).len(),
            2
        );
    }

    #[test]
    fn generations_for_other_settings_or_a_forced_restart_are_never_reused() {
        let (root, adapter) = reasoning_fixture();
        let mut calls = 0;
        grown_ck(root.path(), adapter.path(), 2, 4096, false)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        grown_ck(root.path(), adapter.path(), 3, 2048, false)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        assert_eq!(calls, 2, "a 4096 generation must not answer a 2048 request");
        grown_ck(root.path(), adapter.path(), 4, 2048, true)
            .infer("a", &mut counting(&mut calls))
            .unwrap();
        assert_eq!(calls, 3, "--force-restart regenerates instead of reusing");
    }

    #[test]
    fn an_adapter_is_identified_by_its_weights_not_its_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"v1").unwrap();
        let a = model_identity(dir.path()).unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"v2").unwrap();
        let b = model_identity(dir.path()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn an_adapter_identity_names_how_the_adapter_is_served() {
        // Generations made before the NF4(W)+LoRA serving fix (they served
        // NF4(W+ΔW)) must never be reused: the serving path is part of the key.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"v1").unwrap();
        let id = model_identity(dir.path()).unwrap();
        assert_eq!(id["adapter_serving"], ADAPTER_SERVING);
        assert_eq!(ADAPTER_SERVING, "nf4_base_plus_lora_arm v2");
    }

    #[test]
    fn the_f32_checkpoint_key_is_unchanged() {
        let model = json!({ "snapshot": "/fixed/qwen3" });
        let items = vec![
            vox_eval::reasoning_bench::ReasoningItem {
                id: "item-1".into(),
                question: "What is 2+2?".into(),
                answer: "4".into(),
            },
            vox_eval::reasoning_bench::ReasoningItem {
                id: "item-2".into(),
                question: "What is 3*5?".into(),
                answer: "15".into(),
            },
            vox_eval::reasoning_bench::ReasoningItem {
                id: "item-3".into(),
                question: "Solve x: x - 1 = 0".into(),
                answer: "1".into(),
            },
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let inputs = reasoning_inputs(&model, "gsm8k_think", &items, &settings);
        let key = hash_json(&inputs);
        assert_eq!(
            key,
            "febfd9768573f761620373d72b53778b31a3ecd8fe8bb1944c7b4fd2a277e675"
        );
    }

    #[test]
    fn a_bf16_checkpoint_gets_its_own_file() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("model");
        std::fs::create_dir_all(&model_dir).unwrap();
        let model = json!({ "snapshot": "/fixed/qwen3" });
        let items_f32 = vec![vox_eval::reasoning_bench::ReasoningItem {
            id: "1".into(),
            question: "q1".into(),
            answer: "a1".into(),
        }];
        let items_bf16 = vec![
            vox_eval::reasoning_bench::ReasoningItem {
                id: "1".into(),
                question: "q1".into(),
                answer: "a1".into(),
            },
            vox_eval::reasoning_bench::ReasoningItem {
                id: "2".into(),
                question: "q2".into(),
                answer: "a2".into(),
            },
        ];
        let f32_settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let bf16_settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::Bf16 { gemm_rows: 32 },
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };

        let f32_inputs = reasoning_inputs(&model, "gsm8k_think", &items_f32, &f32_settings);
        let bf16_inputs = reasoning_inputs(&model, "gsm8k_think", &items_bf16, &bf16_settings);

        let keys: Vec<_> = bf16_inputs
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            &keys[keys.len() - 2..],
            &["decode_protocol", "bf16_gemm_rows"]
        );
        assert_eq!(bf16_inputs["decode_protocol"], "bf16");
        assert_eq!(bf16_inputs["bf16_gemm_rows"], 32);

        let f32_path = reasoning_checkpoint_path(&model_dir, "gsm8k_think", &f32_inputs);
        let bf16_path = reasoning_checkpoint_path(&model_dir, "gsm8k_think", &bf16_inputs);
        assert_ne!(f32_path, bf16_path);

        let mut f32_ck = ItemCheckpoint::open(&f32_path, &f32_inputs, false).unwrap();
        f32_ck.record("prompt-1", "resp-1").unwrap();

        let bf16_ck = ItemCheckpoint::open(&bf16_path, &bf16_inputs, false).unwrap();
        assert_eq!(bf16_ck.len(), 0);
        let seed = generations_for_other_item_sets(&model_dir, &bf16_inputs, &bf16_path);
        assert!(
            seed.is_empty(),
            "f32 generations must not seed a bf16 checkpoint"
        );
    }

    #[test]
    fn force_restart_targets_only_its_side() {
        let adapter_only = ForceRestart {
            adapter: true,
            base: false,
        };
        assert!(adapter_only.for_adapter());
        assert!(!adapter_only.for_base());

        let base_only = ForceRestart {
            adapter: false,
            base: true,
        };
        assert!(!base_only.for_adapter());
        assert!(base_only.for_base());

        let both = ForceRestart {
            adapter: true,
            base: true,
        };
        assert!(both.for_adapter());
        assert!(both.for_base());

        let neither = ForceRestart::default();
        assert!(!neither.for_adapter());
        assert!(!neither.for_base());
    }

    #[test]
    fn an_adapter_restart_never_touches_the_base_file() {
        let root = tempfile::tempdir().unwrap();
        let base_dir = tempfile::tempdir().unwrap();
        let adapter_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            adapter_dir.path().join("candle_qlora_adapter.safetensors"),
            b"adapter-bytes",
        )
        .unwrap();

        let items = vec![vox_eval::reasoning_bench::ReasoningItem {
            id: "1".into(),
            question: "q1".into(),
            answer: "a1".into(),
        }];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let mut base_ck = reasoning_checkpoint(
            root.path(),
            base_dir.path(),
            "gsm8k_think",
            &items,
            &settings,
            false,
        )
        .unwrap();
        base_ck.record("prompt-base", "resp-base").unwrap();
        let base_path = base_ck.path.clone();
        drop(base_ck);

        let base_bytes_before = std::fs::read(&base_path).unwrap();

        let mut adapter_ck = reasoning_checkpoint(
            root.path(),
            adapter_dir.path(),
            "gsm8k_think",
            &items,
            &settings,
            true,
        )
        .unwrap();
        adapter_ck.record("prompt-adapt", "resp-adapt").unwrap();
        drop(adapter_ck);

        let base_bytes_after = std::fs::read(&base_path).unwrap();
        assert_eq!(
            base_bytes_before, base_bytes_after,
            "an adapter restart must leave the base checkpoint file untouched"
        );
    }

    #[test]
    fn a_second_adapter_decodes_no_base_items() {
        let root = tempfile::tempdir().unwrap();
        let base_dir = tempfile::tempdir().unwrap();
        let items = vec![
            vox_eval::reasoning_bench::ReasoningItem {
                id: "1".into(),
                question: "q1".into(),
                answer: "a1".into(),
            },
            vox_eval::reasoning_bench::ReasoningItem {
                id: "2".into(),
                question: "q2".into(),
                answer: "a2".into(),
            },
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let infer_calls = std::sync::atomic::AtomicUsize::new(0);
        let mut counting_infer = |prompt: &str| -> Result<String> {
            infer_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(format!("response for {prompt}"))
        };

        let score_base =
            |force: ForceRestart, infer: &mut dyn FnMut(&str) -> Result<String>| -> Result<usize> {
                let mut ck = reasoning_checkpoint(
                    root.path(),
                    base_dir.path(),
                    "gsm8k_think",
                    &items,
                    &settings,
                    force.for_base(),
                )?;
                let mut count = 0;
                for item in &items {
                    let prompt = format!("prompt-{}", item.id);
                    ck.infer(&prompt, infer)?;
                    count += 1;
                }
                Ok(count)
            };

        let run_a_force = ForceRestart {
            adapter: false,
            base: false,
        };
        let gen_a = score_base(run_a_force, &mut counting_infer).unwrap();
        assert_eq!(gen_a, 2);
        assert_eq!(infer_calls.load(std::sync::atomic::Ordering::Relaxed), 2);

        let run_b_force = ForceRestart {
            adapter: true,
            base: false,
        };
        let gen_b = score_base(run_b_force, &mut counting_infer).unwrap();
        assert_eq!(gen_b, 2);
        assert_eq!(
            infer_calls.load(std::sync::atomic::Ordering::Relaxed),
            2,
            "second adapter run must call infer zero times on base"
        );

        let restart_base_force = ForceRestart {
            adapter: false,
            base: true,
        };
        let gen_c = score_base(restart_base_force, &mut counting_infer).unwrap();
        assert_eq!(gen_c, 2);
        assert_eq!(
            infer_calls.load(std::sync::atomic::Ordering::Relaxed),
            4,
            "explicit base restart must call infer for all items"
        );
    }
}
