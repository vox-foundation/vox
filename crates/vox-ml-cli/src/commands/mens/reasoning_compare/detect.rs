//! Model-free input detection for `vox mens eval-reasoning-compare`.
//!
//! Only the exact `<hub>/models--<owner>--<name>/snapshots/<pinned sha>/`
//! directory of the requested base counts; no other revision is listed or
//! matched. Detection uses `is_dir`/`is_file` only and never opens a weight file
//! or loads a model.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use vox_eval::reasoning_bench::{AbsenceReason, AbsenceRecord, BaseLabel};

/// Adapter weights file the candle QLoRA trainer writes.
pub(crate) const ADAPTER_WEIGHTS: &str = "candle_qlora_adapter.safetensors";
/// Adapter manifest the candle QLoRA trainer writes.
pub(crate) const ADAPTER_MANIFEST: &str = "adapter_manifest.json";

/// What detection found.
#[derive(Debug)]
pub(crate) enum Detected {
    /// The pinned snapshot and a matching adapter are both on disk.
    Ready { snapshot: PathBuf, adapter: PathBuf },
    /// Something is missing; this record is written instead of any score.
    NotCompared(AbsenceRecord),
}

/// Hub cache root: `--hub-root`, then `HF_HUB_CACHE`, then `HF_HOME/hub`, then
/// `<home>/.cache/huggingface/hub`. Values are passed in so tests never touch
/// the process environment; empty values are skipped.
pub(crate) fn resolve_hub_root(
    explicit: Option<&Path>,
    hf_hub_cache: Option<OsString>,
    hf_home: Option<OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let set = |v: Option<OsString>| v.filter(|v| !v.is_empty()).map(PathBuf::from);
    explicit
        .map(Path::to_path_buf)
        .or_else(|| set(hf_hub_cache))
        .or_else(|| set(hf_home).map(|h| h.join("hub")))
        .or_else(|| home.map(|h| h.join(".cache/huggingface/hub")))
}

/// The exact snapshot directory for the pinned revision; `None` without one.
pub(crate) fn snapshot_dir(hub_root: &Path, label: &BaseLabel) -> Option<PathBuf> {
    let revision = label.revision.as_ref()?;
    Some(
        hub_root
            .join(format!("models--{}", label.repo_id.replace('/', "--")))
            .join("snapshots")
            .join(revision),
    )
}

/// Decide whether the comparison can run, without loading anything.
pub(crate) fn detect_inputs(
    hub_root: &Path,
    label: &BaseLabel,
    adapter: Option<&Path>,
) -> Detected {
    let pair = adapter.map(|a| {
        (
            a.join(ADAPTER_WEIGHTS).is_file(),
            a.join(ADAPTER_MANIFEST).is_file(),
        )
    });
    let snapshot = snapshot_dir(hub_root, label);
    let absent = |reason: AbsenceReason| {
        Detected::NotCompared(AbsenceRecord {
            base: label.clone(),
            reason,
            searched: snapshot.as_ref().map(|p| p.display().to_string()),
            adapter_given: adapter.is_some(),
            adapter_on_disk: pair == Some((true, true)),
        })
    };
    let Some(snap) = snapshot.as_ref() else {
        return absent(AbsenceReason::PinUnverifiable);
    };
    if !snap.is_dir() {
        return absent(AbsenceReason::BaseSnapshotAbsent);
    }
    let (Some(adapter), Some(pair)) = (adapter, pair) else {
        return absent(AbsenceReason::AdapterAbsent);
    };
    match pair {
        (false, false) => absent(AbsenceReason::AdapterAbsent),
        (true, true) => Detected::Ready {
            snapshot: snap.clone(),
            adapter: adapter.to_path_buf(),
        },
        _ => absent(AbsenceReason::AdapterIncomplete),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const PIN_8B: &str = vox_populi::mens::DEFAULT_MODEL_ID;
    pub(crate) const PIN_0_6B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";

    /// A complete fake snapshot: empty config, tokenizer, and single weights file.
    pub(crate) fn fake_snapshot(hub: &Path, pin: &str) -> PathBuf {
        let dir = snapshot_dir(hub, &BaseLabel::from_pin(pin)).expect("pinned");
        std::fs::create_dir_all(&dir).expect("mkdir snapshot");
        for f in ["config.json", "tokenizer.json", "model.safetensors"] {
            std::fs::write(dir.join(f), b"").expect("write snapshot file");
        }
        dir
    }

    /// An adapter dir with both files; the manifest names `base_model`.
    pub(crate) fn fake_adapter(dir: &Path, base_model: &Path) {
        std::fs::create_dir_all(dir).expect("mkdir adapter");
        std::fs::write(dir.join(ADAPTER_WEIGHTS), b"").expect("write weights");
        let manifest = serde_json::json!({ "base_model": base_model.display().to_string() });
        std::fs::write(dir.join(ADAPTER_MANIFEST), manifest.to_string()).expect("write manifest");
    }

    fn reason(d: &Detected) -> Option<AbsenceReason> {
        match d {
            Detected::NotCompared(r) => Some(r.reason),
            Detected::Ready { .. } => None,
        }
    }

    #[test]
    fn empty_hub_root_is_base_snapshot_absent() {
        let hub = tempfile::tempdir().expect("tempdir");
        let d = detect_inputs(hub.path(), &BaseLabel::from_pin(PIN_8B), None);
        assert_eq!(reason(&d), Some(AbsenceReason::BaseSnapshotAbsent));
        let Detected::NotCompared(r) = d else {
            unreachable!()
        };
        let searched = r.searched.expect("searched path recorded");
        assert!(
            searched.ends_with(
                "models--Qwen--Qwen3-8B/snapshots/b968826d9c46dd6066d109eabc6255188de91218"
            ),
            "{searched}"
        );
    }

    #[test]
    fn hub_with_only_qwen3_0_6b_is_absent_for_the_8b_pin() {
        let hub = tempfile::tempdir().expect("tempdir");
        let small = fake_snapshot(hub.path(), PIN_0_6B);
        let adapter = hub.path().join("adapter");
        fake_adapter(&adapter, &small);
        let d = detect_inputs(hub.path(), &BaseLabel::from_pin(PIN_8B), Some(&adapter));
        assert_eq!(reason(&d), Some(AbsenceReason::BaseSnapshotAbsent));
    }

    #[test]
    fn other_revision_is_not_the_pinned_snapshot() {
        let hub = tempfile::tempdir().expect("tempdir");
        let other = fake_snapshot(
            hub.path(),
            "Qwen/Qwen3-8B@0123456789abcdef0123456789abcdef01234567",
        );
        let adapter = hub.path().join("adapter");
        fake_adapter(&adapter, &other);
        let d = detect_inputs(hub.path(), &BaseLabel::from_pin(PIN_8B), Some(&adapter));
        assert_eq!(reason(&d), Some(AbsenceReason::BaseSnapshotAbsent));
    }

    #[test]
    fn pin_without_40_hex_revision_is_unverifiable() {
        let hub = tempfile::tempdir().expect("tempdir");
        for pin in [
            "Qwen/Qwen3-8B",
            "Qwen/Qwen3-8B@main",
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de9121",
            "Qwen/Qwen3-8B@B968826D9C46DD6066D109EABC6255188DE91218",
        ] {
            let d = detect_inputs(hub.path(), &BaseLabel::from_pin(pin), None);
            assert_eq!(reason(&d), Some(AbsenceReason::PinUnverifiable), "{pin}");
        }
    }

    #[test]
    fn hub_root_order() {
        let os = |s: &str| Some(OsString::from(s));
        let home = Some(PathBuf::from("/home/u"));
        assert_eq!(
            resolve_hub_root(Some(Path::new("/x")), os("/c"), os("/h"), home.clone()),
            Some(PathBuf::from("/x"))
        );
        assert_eq!(
            resolve_hub_root(None, os("/c"), os("/h"), home.clone()),
            Some(PathBuf::from("/c"))
        );
        assert_eq!(
            resolve_hub_root(None, None, os("/h"), home.clone()),
            Some(PathBuf::from("/h/hub"))
        );
        assert_eq!(
            resolve_hub_root(None, os(""), os(""), home.clone()),
            Some(PathBuf::from("/home/u/.cache/huggingface/hub"))
        );
        assert_eq!(resolve_hub_root(None, None, None, None), None);
    }
}
