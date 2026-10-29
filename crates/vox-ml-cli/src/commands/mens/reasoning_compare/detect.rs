//! Model-free detection of a pinned base snapshot and its adapter.
//! Never opens a weight file and never loads a model.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use vox_eval::reasoning_bench::{AbsenceReason, AbsenceRecord, BaseLabel};

/// A pinned snapshot plus a matching adapter, or the absence record to write.
pub(crate) enum Detected {
    /// Both inputs are on disk at the exact pin. Scoring is filled in later.
    Ready { snapshot: PathBuf, adapter: PathBuf },
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
    let Some(adapter_dir) = adapter else {
        return absent(label, AbsenceReason::AdapterAbsent, searched, None, false);
    };
    let weights = adapter_dir.join("candle_qlora_adapter.safetensors");
    let manifest = adapter_dir.join("adapter_manifest.json");
    match (weights.is_file(), manifest.is_file()) {
        (false, false) => absent(
            label,
            AbsenceReason::AdapterAbsent,
            searched,
            Some(adapter_dir),
            false,
        ),
        (false, true) | (true, false) => absent(
            label,
            AbsenceReason::AdapterIncomplete,
            searched,
            Some(adapter_dir),
            false,
        ),
        (true, true) => Detected::Ready {
            snapshot,
            adapter: adapter_dir.to_path_buf(),
        },
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

    fn write_snapshot(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        for name in ["config.json", "tokenizer.json", "model.safetensors"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
    }

    fn place(hub: &Path, repo: &str, revision: &str) -> PathBuf {
        let dir = hub
            .join(format!("models--{}", repo.replace('/', "--")))
            .join("snapshots")
            .join(revision);
        write_snapshot(&dir);
        dir
    }

    fn reason_of(detected: Detected) -> AbsenceReason {
        match detected {
            Detected::NotCompared(record) => record.reason,
            Detected::Ready { .. } => panic!("expected an absence, got Ready"),
        }
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
            resolve_hub_root(
                None,
                Some(cache.clone()),
                Some(hf_home.clone()),
                Some(home.clone())
            ),
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
}
