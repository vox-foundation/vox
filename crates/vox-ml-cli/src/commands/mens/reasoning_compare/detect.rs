//! Model-free input detection for `vox mens eval-reasoning-compare`.
//!
//! Only the exact `<hub>/models--<owner>--<name>/snapshots/<pinned sha>/`
//! directory of the requested base counts; no other revision is listed or
//! matched. Detection uses `is_dir`/`is_file` only and never opens a weight file
//! or loads a model.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use vox_eval::reasoning_bench::{AbsenceReason, AbsenceRecord, BaseLabel};
use vox_populi::mens::tensor::spoke_base_resolver::TrainBase;

/// Adapter weights file the candle QLoRA trainer writes.
pub(crate) const ADAPTER_WEIGHTS: &str = "candle_qlora_adapter.safetensors";
/// Adapter manifest the candle QLoRA trainer writes.
pub(crate) const ADAPTER_MANIFEST: &str = "adapter_manifest.json";

/// What detection found.
#[derive(Debug)]
pub(crate) enum Detected {
    /// The pinned snapshot and a matching adapter are both on disk.
    Ready {
        base: BaseLabel,
        snapshot: PathBuf,
        adapter: PathBuf,
    },
    /// Something is missing; this record is written instead of any score.
    NotCompared(AbsenceRecord),
}

/// The variables hf-hub 1.0 reads for its cache dir, captured once so tests
/// never touch the process environment.
#[derive(Debug, Clone, Default)]
pub(crate) struct HubEnv {
    pub hf_hub_cache: Option<OsString>,
    pub huggingface_hub_cache: Option<OsString>,
    pub hf_home: Option<OsString>,
    pub xdg_cache_home: Option<OsString>,
    pub home: Option<PathBuf>,
}

impl HubEnv {
    /// The current process environment.
    pub(crate) fn from_process() -> Self {
        Self {
            hf_hub_cache: std::env::var_os("HF_HUB_CACHE"),
            huggingface_hub_cache: std::env::var_os("HUGGINGFACE_HUB_CACHE"),
            hf_home: std::env::var_os("HF_HOME"),
            xdg_cache_home: std::env::var_os("XDG_CACHE_HOME"),
            home: dirs::home_dir(),
        }
    }
}

/// Hub cache root: `--hub-root`, else hf-hub 1.0 `resolve_cache_dir` order (the
/// client `vox mens download` writes through): `HF_HUB_CACHE`,
/// `HUGGINGFACE_HUB_CACHE`, then `<hf home>/hub` where hf home is `HF_HOME`,
/// `XDG_CACHE_HOME/huggingface`, or `<home>/.cache/huggingface`. Empty values
/// are skipped.
pub(crate) fn resolve_hub_root(explicit: Option<&Path>, env: HubEnv) -> Option<PathBuf> {
    let set = |v: Option<OsString>| v.filter(|v| !v.is_empty()).map(PathBuf::from);
    let HubEnv {
        hf_hub_cache,
        huggingface_hub_cache,
        hf_home,
        xdg_cache_home,
        home,
    } = env;
    explicit
        .map(Path::to_path_buf)
        .or_else(|| set(hf_hub_cache))
        .or_else(|| set(huggingface_hub_cache))
        .or_else(|| {
            set(hf_home)
                .or_else(|| set(xdg_cache_home).map(|x| x.join("huggingface")))
                .or_else(|| home.map(|h| h.join(".cache/huggingface")))
                .map(|h| h.join("hub"))
        })
}

/// The pin to compare. `None` is [`vox_populi::mens::DEFAULT_MODEL_ID`]; a
/// `repo@<40 hex>` pin passes through; a size name (`Qwen3-8B`) or bare repo id
/// (`Qwen/Qwen3-8B`) resolves to the single distinct pinned `hf_id` across all
/// `train_bases` tags. Unknown or ambiguous input comes back unchanged, so
/// detection records `pin_unverifiable`; there is no fallback revision.
pub(crate) fn resolve_base_pin(
    requested: Option<&str>,
    overlay: &HashMap<String, Vec<TrainBase>>,
) -> String {
    let Some(requested) = requested else {
        return vox_populi::mens::DEFAULT_MODEL_ID.to_string();
    };
    if BaseLabel::from_pin(requested).revision.is_some() {
        return requested.to_string();
    }
    let suffix = format!("/{requested}");
    let pins: std::collections::BTreeSet<&str> = overlay
        .values()
        .flatten()
        .map(|b| b.hf_id.as_str())
        .filter(|id| {
            let label = BaseLabel::from_pin(id);
            label.revision.is_some()
                && (label.repo_id == requested || label.repo_id.ends_with(&suffix))
        })
        .collect();
    match pins.into_iter().collect::<Vec<_>>().as_slice() {
        [only] => (*only).to_string(),
        _ => requested.to_string(),
    }
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
    if !snapshot_complete(snap) {
        return absent(AbsenceReason::BaseSnapshotIncomplete);
    }
    let (Some(adapter), Some(pair)) = (adapter, pair) else {
        return absent(AbsenceReason::AdapterAbsent);
    };
    match pair {
        (false, false) => return absent(AbsenceReason::AdapterAbsent),
        (true, true) => {}
        _ => return absent(AbsenceReason::AdapterIncomplete),
    }
    let Some(manifest) = read_json(&adapter.join(ADAPTER_MANIFEST)) else {
        return absent(AbsenceReason::AdapterIncomplete);
    };
    let base_model = manifest
        .get("base_model")
        .and_then(serde_json::Value::as_str)
        .map(Path::new);
    if !base_model.is_some_and(|b| b.is_dir() && same_dir(b, snap)) {
        return absent(AbsenceReason::AdapterBaseNotPinnedSnapshot);
    }
    Detected::Ready {
        base: label.clone(),
        snapshot: snap.clone(),
        adapter: adapter.to_path_buf(),
    }
}

/// Size-capped read and parse; any failure is `None`.
fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = vox_bounded_fs::read_utf8_path_capped(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Both paths canonicalize to the same directory.
fn same_dir(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

/// `config.json`, `tokenizer.json`, and every weight shard are files. With a
/// `model.safetensors.index.json`, every distinct `weight_map` value must be a
/// plain file name present in the snapshot; otherwise `model.safetensors` must be.
/// A dangling symlink is not a file, so a partial download is incomplete.
fn snapshot_complete(snap: &Path) -> bool {
    if !(snap.join("config.json").is_file() && snap.join("tokenizer.json").is_file()) {
        return false;
    }
    let index = snap.join("model.safetensors.index.json");
    if std::fs::symlink_metadata(&index).is_err() {
        return snap.join("model.safetensors").is_file();
    }
    let Some(v) = read_json(&index) else {
        return false;
    };
    let Some(map) = v.get("weight_map").and_then(serde_json::Value::as_object) else {
        return false;
    };
    let Some(shards) = map
        .values()
        .map(serde_json::Value::as_str)
        .collect::<Option<std::collections::BTreeSet<&str>>>()
    else {
        return false;
    };
    let plain = |s: &str| {
        matches!(
            Path::new(s).components().collect::<Vec<_>>().as_slice(),
            [std::path::Component::Normal(_)]
        )
    };
    !shards.is_empty() && shards.iter().all(|s| plain(s) && snap.join(s).is_file())
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

    /// Pinned 8B snapshot plus an adapter whose manifest points at it.
    fn present(hub: &Path) -> (PathBuf, PathBuf) {
        let snap = fake_snapshot(hub, PIN_8B);
        let adapter = hub.join("adapter");
        fake_adapter(&adapter, &snap);
        (snap, adapter)
    }

    fn detect_8b(hub: &Path, adapter: &Path) -> Option<AbsenceReason> {
        reason(&detect_inputs(
            hub,
            &BaseLabel::from_pin(PIN_8B),
            Some(adapter),
        ))
    }

    fn write_index(snap: &Path, shards: &[&str]) {
        let map: serde_json::Map<String, serde_json::Value> = shards
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("layer.{i}.weight"), serde_json::json!(s)))
            .collect();
        let index = serde_json::json!({ "metadata": {}, "weight_map": map });
        std::fs::write(snap.join("model.safetensors.index.json"), index.to_string())
            .expect("write index");
        std::fs::remove_file(snap.join("model.safetensors")).expect("drop single file");
    }

    const SHARDS: [&str; 2] = [
        "model-00001-of-00002.safetensors",
        "model-00002-of-00002.safetensors",
    ];

    #[test]
    fn snapshot_missing_tokenizer_is_incomplete() {
        let hub = tempfile::tempdir().expect("tempdir");
        let (snap, adapter) = present(hub.path());
        std::fs::remove_file(snap.join("tokenizer.json")).expect("rm tokenizer");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::BaseSnapshotIncomplete)
        );
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_with_dangling_shard_link_is_incomplete() {
        let hub = tempfile::tempdir().expect("tempdir");
        let (snap, adapter) = present(hub.path());
        write_index(&snap, &SHARDS);
        std::fs::write(snap.join(SHARDS[0]), b"").expect("shard 1");
        std::os::unix::fs::symlink(hub.path().join("blobs/missing"), snap.join(SHARDS[1]))
            .expect("dangling link");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::BaseSnapshotIncomplete)
        );
    }

    #[test]
    fn snapshot_with_all_indexed_shards_is_present() {
        let hub = tempfile::tempdir().expect("tempdir");
        let (snap, adapter) = present(hub.path());
        write_index(&snap, &[SHARDS[0], SHARDS[1], SHARDS[1]]);
        std::fs::write(snap.join(SHARDS[0]), b"").expect("shard 1");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::BaseSnapshotIncomplete),
            "one shard still missing"
        );
        std::fs::write(snap.join(SHARDS[1]), b"").expect("shard 2");
        assert_eq!(detect_8b(hub.path(), &adapter), None);
        // An index entry that escapes the snapshot never counts.
        write_index_keep(&snap, &["../escape.safetensors"]);
        std::fs::write(snap.join("../escape.safetensors"), b"").expect("outside file");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::BaseSnapshotIncomplete)
        );
    }

    fn write_index_keep(snap: &Path, shards: &[&str]) {
        std::fs::write(snap.join("model.safetensors"), b"").expect("restore single file");
        write_index(snap, shards);
    }

    #[test]
    fn snapshot_with_single_safetensors_is_present() {
        let hub = tempfile::tempdir().expect("tempdir");
        let (snap, adapter) = present(hub.path());
        match detect_inputs(hub.path(), &BaseLabel::from_pin(PIN_8B), Some(&adapter)) {
            Detected::Ready {
                base,
                snapshot,
                adapter: a,
            } => {
                assert_eq!(base, BaseLabel::from_pin(PIN_8B));
                assert_eq!(snapshot, snap);
                assert_eq!(a, adapter);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
        std::fs::remove_file(snap.join("model.safetensors")).expect("rm weights");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::BaseSnapshotIncomplete)
        );
    }

    #[test]
    fn adapter_manifest_base_must_be_the_pinned_snapshot() {
        let hub = tempfile::tempdir().expect("tempdir");
        let (snap, adapter) = present(hub.path());
        let manifest = adapter.join(ADAPTER_MANIFEST);
        let other = hub.path().join("other-base");
        std::fs::create_dir_all(&other).expect("mkdir other");
        for body in [
            serde_json::json!({ "base_model": other.display().to_string() }).to_string(),
            serde_json::json!({ "base_model": "Qwen/Qwen3-8B" }).to_string(),
            serde_json::json!({ "base_model": 7 }).to_string(),
            serde_json::json!({ "rank": 16 }).to_string(),
        ] {
            std::fs::write(&manifest, &body).expect("write manifest");
            assert_eq!(
                detect_8b(hub.path(), &adapter),
                Some(AbsenceReason::AdapterBaseNotPinnedSnapshot),
                "{body}"
            );
        }
        std::fs::write(&manifest, "{not json").expect("write manifest");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::AdapterIncomplete)
        );
        let sha = snap
            .file_name()
            .expect("sha")
            .to_string_lossy()
            .into_owned();
        let roundabout = snap.join("..").join("..").join("snapshots").join(sha);
        fake_adapter(&adapter, &roundabout);
        assert_eq!(detect_8b(hub.path(), &adapter), None);
        std::fs::remove_file(adapter.join(ADAPTER_WEIGHTS)).expect("rm weights");
        assert_eq!(
            detect_8b(hub.path(), &adapter),
            Some(AbsenceReason::AdapterIncomplete)
        );
    }

    // ---- Task 3: base resolution ----

    fn repo_overlay() -> HashMap<String, Vec<TrainBase>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&root).expect("overlay")
    }

    fn base(hf_id: &str) -> TrainBase {
        TrainBase {
            hf_id: hf_id.to_string(),
            floor_mb: 1,
            methods: Vec::new(),
            license: None,
            attribution_required: false,
        }
    }

    #[test]
    fn base_resolution_default_is_the_pinned_8b() {
        assert_eq!(resolve_base_pin(None, &repo_overlay()), PIN_8B);
        assert_eq!(
            PIN_8B,
            "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"
        );
    }

    #[test]
    fn base_resolution_full_pin_passes_through() {
        let overlay = repo_overlay();
        for pin in [
            PIN_0_6B,
            "someone/Other-Model@0123456789abcdef0123456789abcdef01234567",
        ] {
            assert_eq!(resolve_base_pin(Some(pin), &overlay), pin);
        }
    }

    #[test]
    fn base_resolution_size_name_uses_train_bases() {
        let overlay = repo_overlay();
        assert_eq!(resolve_base_pin(Some("Qwen3-8B"), &overlay), PIN_8B);
        assert_eq!(resolve_base_pin(Some("Qwen/Qwen3-8B"), &overlay), PIN_8B);
        assert_eq!(resolve_base_pin(Some("Qwen3-0.6B"), &overlay), PIN_0_6B);
    }

    #[test]
    fn base_resolution_unknown_or_ambiguous_is_unverifiable() {
        let hub = tempfile::tempdir().expect("tempdir");
        let unknown = resolve_base_pin(Some("Qwen3-1234B"), &repo_overlay());
        assert_eq!(unknown, "Qwen3-1234B");
        let mut overlay = HashMap::new();
        overlay.insert(
            "a".to_string(),
            vec![base(
                "Qwen/Qwen3-8B@0123456789abcdef0123456789abcdef01234567",
            )],
        );
        overlay.insert("b".to_string(), vec![base(PIN_8B)]);
        let ambiguous = resolve_base_pin(Some("Qwen3-8B"), &overlay);
        assert_eq!(ambiguous, "Qwen3-8B");
        for pin in [unknown, ambiguous] {
            let d = detect_inputs(hub.path(), &BaseLabel::from_pin(&pin), None);
            assert_eq!(reason(&d), Some(AbsenceReason::PinUnverifiable), "{pin}");
        }
    }

    #[test]
    fn base_resolution_size_is_detected_at_its_own_snapshot() {
        let hub = tempfile::tempdir().expect("tempdir");
        let snap = fake_snapshot(hub.path(), PIN_0_6B);
        let adapter = hub.path().join("adapter");
        fake_adapter(&adapter, &snap);
        let small = BaseLabel::from_pin(&resolve_base_pin(Some("Qwen3-0.6B"), &repo_overlay()));
        assert!(matches!(
            detect_inputs(hub.path(), &small, Some(&adapter)),
            Detected::Ready { .. }
        ));
        let default = BaseLabel::from_pin(&resolve_base_pin(None, &repo_overlay()));
        assert_eq!(
            reason(&detect_inputs(hub.path(), &default, Some(&adapter))),
            Some(AbsenceReason::BaseSnapshotAbsent)
        );
    }

    /// Same order as hf-hub 1.0 `resolve_cache_dir`, which `vox mens download` uses.
    #[test]
    fn hub_root_order() {
        let os = |s: &str| Some(OsString::from(s));
        let full = || HubEnv {
            hf_hub_cache: os("/c"),
            huggingface_hub_cache: os("/legacy"),
            hf_home: os("/h"),
            xdg_cache_home: os("/xdg"),
            home: Some(PathBuf::from("/home/u")),
        };
        let root = |explicit: Option<&str>, env: HubEnv| {
            resolve_hub_root(explicit.map(Path::new), env).map(|p| p.display().to_string())
        };
        assert_eq!(root(Some("/x"), full()).as_deref(), Some("/x"));
        assert_eq!(root(None, full()).as_deref(), Some("/c"));
        let mut env = full();
        env.hf_hub_cache = os("");
        assert_eq!(root(None, env.clone()).as_deref(), Some("/legacy"));
        env.huggingface_hub_cache = None;
        assert_eq!(root(None, env.clone()).as_deref(), Some("/h/hub"));
        env.hf_home = None;
        assert_eq!(
            root(None, env.clone()).as_deref(),
            Some("/xdg/huggingface/hub")
        );
        env.xdg_cache_home = os("");
        assert_eq!(
            root(None, env).as_deref(),
            Some("/home/u/.cache/huggingface/hub")
        );
        assert_eq!(root(None, HubEnv::default()), None);
    }
}
