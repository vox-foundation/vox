//! Model-free detection of a pinned base snapshot and its adapter.
//! Never opens a weight file and never loads a model.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use vox_eval::reasoning_bench::{AbsenceReason, AbsenceRecord, BaseLabel};

/// A pinned snapshot plus a matching adapter, or the absence record to write.
pub(crate) enum Detected {
    /// Both inputs are on disk at the exact pin.
    Ready {
        snapshot: PathBuf,
        adapter: PathBuf,
        base: BaseLabel,
    },
    /// Detection stopped. The record is the only thing the command may write.
    NotCompared(AbsenceRecord),
}

/// `explicit`, then `HF_HUB_CACHE`, then `HF_HOME/hub`, then `<home>/.cache/huggingface/hub`.
pub(crate) fn resolve_hub_root(
    explicit: Option<&Path>,
    hf_hub_cache: Option<OsString>,
    hf_home: Option<OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    if let Some(cache) = hf_hub_cache {
        return Some(PathBuf::from(cache));
    }
    if let Some(hf_home) = hf_home {
        return Some(PathBuf::from(hf_home).join("hub"));
    }
    home.map(|dir| dir.join(".cache/huggingface/hub"))
}

/// Exact `models--<owner>--<name>/snapshots/<sha>/` path. `None` when the pin has no revision.
pub(crate) fn snapshot_dir(hub_root: &Path, label: &BaseLabel) -> Option<PathBuf> {
    let revision = label.revision.as_ref()?;
    let repo = label.repo_id.replace('/', "--");
    Some(
        hub_root
            .join(format!("models--{repo}"))
            .join("snapshots")
            .join(revision),
    )
}

/// Classify the hub tree and adapter. Looks only at the exact pinned snapshot path.
pub(crate) fn detect_inputs(
    hub_root: &Path,
    label: &BaseLabel,
    adapter: Option<&Path>,
) -> Detected {
    let Some(snapshot) = snapshot_dir(hub_root, label) else {
        return absent(label, AbsenceReason::PinUnverifiable, None, adapter, false);
    };
    let searched = Some(snapshot.display().to_string());
    if !snapshot.is_dir() {
        return absent(
            label,
            AbsenceReason::BaseSnapshotAbsent,
            searched,
            adapter,
            false,
        );
    }
    if !snapshot_complete(&snapshot) {
        return absent(
            label,
            AbsenceReason::BaseSnapshotIncomplete,
            searched,
            adapter,
            false,
        );
    }
    let Some(adapter_dir) = adapter else {
        return absent(label, AbsenceReason::AdapterAbsent, searched, None, false);
    };
    let weights = adapter_dir.join("candle_qlora_adapter.safetensors");
    let manifest = adapter_dir.join("adapter_manifest.json");
    let pair_on_disk = weights.is_file() && manifest.is_file();
    match (weights.is_file(), manifest.is_file()) {
        (false, false) => {
            return absent(
                label,
                AbsenceReason::AdapterAbsent,
                searched,
                Some(adapter_dir),
                false,
            );
        }
        (false, true) | (true, false) => {
            return absent(
                label,
                AbsenceReason::AdapterIncomplete,
                searched,
                Some(adapter_dir),
                false,
            );
        }
        (true, true) => {}
    }
    match manifest_matches(&manifest, &snapshot) {
        ManifestMatch::Matches => Detected::Ready {
            snapshot,
            adapter: adapter_dir.to_path_buf(),
            base: label.clone(),
        },
        ManifestMatch::Incomplete => absent(
            label,
            AbsenceReason::AdapterIncomplete,
            searched,
            Some(adapter_dir),
            pair_on_disk,
        ),
        ManifestMatch::NotPinned => absent(
            label,
            AbsenceReason::AdapterBaseNotPinnedSnapshot,
            searched,
            Some(adapter_dir),
            pair_on_disk,
        ),
    }
}

fn snapshot_complete(snapshot: &Path) -> bool {
    if !snapshot.join("config.json").is_file() || !snapshot.join("tokenizer.json").is_file() {
        return false;
    }
    let index = snapshot.join("model.safetensors.index.json");
    if index.is_file() {
        let Ok(text) = vox_bounded_fs::read_utf8_path_capped(&index) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return false;
        };
        let Some(map) = value
            .get("weight_map")
            .and_then(serde_json::Value::as_object)
        else {
            return false;
        };
        let mut shards: Vec<&str> = map.values().filter_map(serde_json::Value::as_str).collect();
        shards.sort_unstable();
        shards.dedup();
        !shards.is_empty() && shards.iter().all(|shard| shard_is_file(snapshot, shard))
    } else {
        snapshot.join("model.safetensors").is_file()
    }
}

/// A shard counts only as a file inside the snapshot. `..` and absolute names do not.
fn shard_is_file(snapshot: &Path, shard: &str) -> bool {
    let rel = Path::new(shard);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    snapshot.join(rel).is_file()
}

enum ManifestMatch {
    Matches,
    Incomplete,
    NotPinned,
}

fn manifest_matches(manifest: &Path, snapshot: &Path) -> ManifestMatch {
    let Ok(text) = vox_bounded_fs::read_utf8_path_capped(manifest) else {
        return ManifestMatch::Incomplete;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return ManifestMatch::Incomplete;
    };
    let Some(base) = value.get("base_model").and_then(serde_json::Value::as_str) else {
        return ManifestMatch::NotPinned;
    };
    let path = Path::new(base);
    if !path.is_dir() {
        return ManifestMatch::NotPinned;
    }
    match (path.canonicalize(), snapshot.canonicalize()) {
        (Ok(left), Ok(right)) if left == right => ManifestMatch::Matches,
        _ => ManifestMatch::NotPinned,
    }
}

fn absent(
    label: &BaseLabel,
    reason: AbsenceReason,
    searched: Option<String>,
    adapter: Option<&Path>,
    adapter_on_disk: bool,
) -> Detected {
    Detected::NotCompared(AbsenceRecord {
        base: label.clone(),
        reason,
        searched,
        adapter_given: adapter.is_some(),
        adapter_on_disk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";
    const PIN_06B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";

    fn write_files(dir: &Path, names: &[&str]) {
        std::fs::create_dir_all(dir).unwrap();
        for name in names {
            std::fs::write(dir.join(name), b"").unwrap();
        }
    }

    fn place(hub: &Path, repo: &str, revision: &str) -> PathBuf {
        let dir = hub
            .join(format!("models--{}", repo.replace('/', "--")))
            .join("snapshots")
            .join(revision);
        write_files(
            &dir,
            &["config.json", "tokenizer.json", "model.safetensors"],
        );
        dir
    }

    fn reason_of(detected: Detected) -> AbsenceReason {
        match detected {
            Detected::NotCompared(record) => record.reason,
            Detected::Ready { .. } => panic!("expected an absence, got Ready"),
        }
    }

    fn write_adapter(dir: &Path, base_model: &str) {
        write_files(dir, &["candle_qlora_adapter.safetensors"]);
        std::fs::write(
            dir.join("adapter_manifest.json"),
            serde_json::json!({ "base_model": base_model }).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn empty_hub_root_is_base_snapshot_absent() {
        let hub = tempfile::tempdir().unwrap();
        let label = BaseLabel::from_pin(PIN_8B);
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, None)),
            AbsenceReason::BaseSnapshotAbsent
        );
    }

    #[test]
    fn hub_with_only_qwen3_0_6b_is_absent_for_the_8b_pin() {
        let hub = tempfile::tempdir().unwrap();
        let rev = PIN_06B.rsplit('@').next().unwrap();
        place(hub.path(), "Qwen/Qwen3-0.6B", rev);
        let label = BaseLabel::from_pin(PIN_8B);
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, None)),
            AbsenceReason::BaseSnapshotAbsent
        );
    }

    #[test]
    fn other_revision_is_not_the_pinned_snapshot() {
        let hub = tempfile::tempdir().unwrap();
        place(
            hub.path(),
            "Qwen/Qwen3-8B",
            "0123456789abcdef0123456789abcdef01234567",
        );
        let label = BaseLabel::from_pin(PIN_8B);
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, None)),
            AbsenceReason::BaseSnapshotAbsent
        );
    }

    #[test]
    fn pin_without_40_hex_revision_is_unverifiable() {
        let hub = tempfile::tempdir().unwrap();
        let short = "b968826d9c46dd6066d109eabc6255188de9121";
        assert_eq!(short.len(), 39);
        for pin in [
            "Qwen/Qwen3-8B",
            "Qwen/Qwen3-8B@main",
            &format!("Qwen/Qwen3-8B@{short}"),
        ] {
            let label = BaseLabel::from_pin(pin);
            assert_eq!(
                reason_of(detect_inputs(hub.path(), &label, None)),
                AbsenceReason::PinUnverifiable,
                "{pin}"
            );
        }
    }

    #[test]
    fn hub_root_order() {
        let explicit = Path::new("/explicit");
        let cache = OsString::from("/cache");
        let hf_home = OsString::from("/hfhome");
        let home = PathBuf::from("/home/user");
        assert_eq!(
            resolve_hub_root(
                Some(explicit),
                Some(cache.clone()),
                Some(hf_home.clone()),
                Some(home.clone())
            ),
            Some(explicit.to_path_buf())
        );
        assert_eq!(
            resolve_hub_root(None, Some(cache), Some(hf_home.clone()), Some(home.clone())),
            Some(PathBuf::from("/cache"))
        );
        assert_eq!(
            resolve_hub_root(None, None, Some(hf_home), Some(home.clone())),
            Some(PathBuf::from("/hfhome/hub"))
        );
        assert_eq!(
            resolve_hub_root(None, None, None, Some(home)),
            Some(PathBuf::from("/home/user/.cache/huggingface/hub"))
        );
        assert_eq!(resolve_hub_root(None, None, None, None), None);
    }

    fn pinned_dir(hub: &Path) -> PathBuf {
        let label = BaseLabel::from_pin(PIN_8B);
        hub.join("models--Qwen--Qwen3-8B")
            .join("snapshots")
            .join(label.revision.expect("pin"))
    }

    #[test]
    fn snapshot_missing_tokenizer_is_incomplete() {
        let hub = tempfile::tempdir().unwrap();
        let dir = pinned_dir(hub.path());
        write_files(&dir, &["config.json", "model.safetensors"]);
        let label = BaseLabel::from_pin(PIN_8B);
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, None)),
            AbsenceReason::BaseSnapshotIncomplete
        );
    }

    #[test]
    fn snapshot_with_dangling_shard_link_is_incomplete() {
        let hub = tempfile::tempdir().unwrap();
        let dir = pinned_dir(hub.path());
        write_files(
            &dir,
            &[
                "config.json",
                "tokenizer.json",
                "model-00001-of-00002.safetensors",
            ],
        );
        let index = serde_json::json!({
            "weight_map": {
                "a": "model-00001-of-00002.safetensors",
                "b": "model-00002-of-00002.safetensors"
            }
        });
        std::fs::write(dir.join("model.safetensors.index.json"), index.to_string()).unwrap();
        std::os::unix::fs::symlink("missing-blob", dir.join("model-00002-of-00002.safetensors"))
            .unwrap();
        let label = BaseLabel::from_pin(PIN_8B);
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, None)),
            AbsenceReason::BaseSnapshotIncomplete
        );
    }

    #[test]
    fn snapshot_with_all_indexed_shards_is_present() {
        let hub = tempfile::tempdir().unwrap();
        let dir = pinned_dir(hub.path());
        write_files(
            &dir,
            &[
                "config.json",
                "tokenizer.json",
                "model-00001-of-00002.safetensors",
                "model-00002-of-00002.safetensors",
            ],
        );
        let index = serde_json::json!({
            "weight_map": {
                "a": "model-00001-of-00002.safetensors",
                "b": "model-00002-of-00002.safetensors"
            }
        });
        std::fs::write(dir.join("model.safetensors.index.json"), index.to_string()).unwrap();
        let adapter = tempfile::tempdir().unwrap();
        write_adapter(adapter.path(), &dir.display().to_string());
        let label = BaseLabel::from_pin(PIN_8B);
        assert!(matches!(
            detect_inputs(hub.path(), &label, Some(adapter.path())),
            Detected::Ready { .. }
        ));
    }

    #[test]
    fn snapshot_with_single_safetensors_is_present() {
        let hub = tempfile::tempdir().unwrap();
        let dir = place(
            hub.path(),
            "Qwen/Qwen3-8B",
            PIN_8B.rsplit('@').next().unwrap(),
        );
        let adapter = tempfile::tempdir().unwrap();
        write_adapter(adapter.path(), &dir.display().to_string());
        let label = BaseLabel::from_pin(PIN_8B);
        assert!(matches!(
            detect_inputs(hub.path(), &label, Some(adapter.path())),
            Detected::Ready { .. }
        ));
    }

    #[test]
    fn adapter_manifest_base_must_be_the_pinned_snapshot() {
        let hub = tempfile::tempdir().unwrap();
        let dir = place(
            hub.path(),
            "Qwen/Qwen3-8B",
            PIN_8B.rsplit('@').next().unwrap(),
        );
        let label = BaseLabel::from_pin(PIN_8B);
        let other = tempfile::tempdir().unwrap();

        let mismatch = tempfile::tempdir().unwrap();
        write_adapter(mismatch.path(), &other.path().display().to_string());
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, Some(mismatch.path()))),
            AbsenceReason::AdapterBaseNotPinnedSnapshot
        );

        let hub_id = tempfile::tempdir().unwrap();
        write_adapter(hub_id.path(), "Qwen/Qwen3-8B");
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, Some(hub_id.path()))),
            AbsenceReason::AdapterBaseNotPinnedSnapshot
        );

        let missing = tempfile::tempdir().unwrap();
        write_files(missing.path(), &["candle_qlora_adapter.safetensors"]);
        std::fs::write(missing.path().join("adapter_manifest.json"), "{}").unwrap();
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, Some(missing.path()))),
            AbsenceReason::AdapterBaseNotPinnedSnapshot
        );

        let malformed = tempfile::tempdir().unwrap();
        write_files(malformed.path(), &["candle_qlora_adapter.safetensors"]);
        std::fs::write(malformed.path().join("adapter_manifest.json"), "not-json").unwrap();
        assert_eq!(
            reason_of(detect_inputs(hub.path(), &label, Some(malformed.path()))),
            AbsenceReason::AdapterIncomplete
        );

        let via = dir.join("..").join(dir.file_name().unwrap());
        let alias = tempfile::tempdir().unwrap();
        write_adapter(alias.path(), &via.display().to_string());
        assert!(matches!(
            detect_inputs(hub.path(), &label, Some(alias.path())),
            Detected::Ready { .. }
        ));
    }
}
