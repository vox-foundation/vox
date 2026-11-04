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

/// The checkpoint for one model on one reasoning slice under `root`.
pub(crate) fn reasoning_checkpoint(
    root: &Path,
    model_dir: &Path,
    bench: &str,
    items: &[vox_eval::reasoning_bench::ReasoningItem],
    max_tokens: usize,
    force_restart: bool,
) -> Result<ItemCheckpoint> {
    let model = model_identity(model_dir)?;
    let item_set: Vec<Value> = items
        .iter()
        .map(|i| json!([i.id, i.question, i.answer]))
        .collect();
    let inputs = json!({
        "scorer": REASONING_SCORER,
        "model": model,
        "bench": bench,
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "seed": 0,
        "items_blake3": hash_json(&Value::Array(item_set)),
    });
    let path = root
        .join(&hash_json(&model)[..16])
        .join(format!("{bench}.jsonl"));
    ItemCheckpoint::open(&path, &inputs, force_restart)
}

/// BLAKE3 hex of the compact JSON text of `v` (object keys sorted by serde_json).
pub(crate) fn hash_json(v: &Value) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(v.to_string().as_bytes()))
}

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
        let item = vox_crypto::hex_encode(&vox_crypto::secure_hash(prompt_json.as_bytes()));
        if let Some(resp) = self.done.get(&item) {
            return Ok(resp.clone());
        }
        let resp = inner(prompt_json)?;
        self.append(&json!({ "item": item, "response": resp }))?;
        self.done.insert(item, resp.clone());
        Ok(resp)
    }
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

    #[test]
    fn an_adapter_is_identified_by_its_weights_not_its_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"v1").unwrap();
        let a = model_identity(dir.path()).unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"v2").unwrap();
        let b = model_identity(dir.path()).unwrap();
        assert_ne!(a, b);
    }
}
