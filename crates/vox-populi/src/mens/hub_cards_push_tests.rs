use super::*;
use crate::mens::hub_sync::{classify_size_sync, stage_publish_bundle};
use serde_json::json;
use std::path::PathBuf;

const PIN: &str = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";

/// A run dir with weights, a manifest and a trained record (no receipts, so the local label is ungated).
fn run_dir(root: &Path) -> PathBuf {
    let dir = root.join("qwen3-1.7b");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("candle_qlora_adapter.safetensors"), b"weights").unwrap();
    let manifest = json!({"base_model": PIN, "finetune_contract_digest": "digest"});
    std::fs::write(dir.join("training_manifest.json"), manifest.to_string()).unwrap();
    let record = json!({"schema": "vox_mens_train_record_v1", "status": "trained", "size": "1.7B"});
    std::fs::write(dir.join("train-record.json"), record.to_string()).unwrap();
    dir
}

/// The local bundle (staged and hashed) and the remote manifest that carries an older card and label.
fn local_and_remote(root: &Path) -> (PathBuf, HfSyncManifest, HfSyncManifest) {
    let run = run_dir(root);
    let stage = root.join("stage");
    let local = stage_publish_bundle(&run, &stage, true, "ns").unwrap();
    let mut remote = local.clone();
    remote.gate_status = "passed".to_string();
    for e in remote
        .files
        .iter_mut()
        .filter(|e| is_card_collateral(&e.path))
    {
        e.blake3 = "0".repeat(64);
        e.size_bytes += 1;
    }
    (stage, local, remote)
}

fn row(status: SizeSyncStatus) -> SizeSyncRow {
    SizeSyncRow {
        size: "1.7B".to_string(),
        base_pin: PIN.to_string(),
        repo_id: "ns/vox-mens-qwen3-1.7b".to_string(),
        local_dir: "mens/runs/qwen3-1.7b".to_string(),
        sync_status: status,
        gate_status: None,
        final_loss: None,
        measured_peak_mb: None,
        action_taken: None,
        latest_version: None,
    }
}

#[test]
fn card_only_push_leaves_size_in_sync() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, remote) = local_and_remote(tmp.path());
    assert_eq!(
        classify_size_sync(Some(&local), Some(&remote)),
        SizeSyncStatus::Diverged
    );
    let merged = card_only_manifest(&remote, &local);
    assert_eq!(
        classify_size_sync(Some(&local), Some(&merged)),
        SizeSyncStatus::InSync
    );
}

#[test]
fn card_only_manifest_prunes_removed_metadata_and_keeps_diverged_weights_diverged() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, mut remote) = local_and_remote(tmp.path());
    for e in remote
        .files
        .iter_mut()
        .filter(|e| e.path.ends_with(".safetensors"))
    {
        e.blake3 = "f".repeat(64);
    }
    remote.files.push(FileHashEntry {
        path: "stale_meta.json".to_string(),
        size_bytes: 12,
        blake3: "1".repeat(64),
    });
    assert!(!remote_weights_match(Some(&remote), &local));
    let merged = card_only_manifest(&remote, &local);
    assert!(!merged.files.iter().any(|f| f.path == "stale_meta.json"));
    assert_eq!(
        classify_size_sync(Some(&local), Some(&merged)),
        SizeSyncStatus::Diverged
    );
}

#[test]
fn card_only_manifest_matches_staged_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (stage, local, remote) = local_and_remote(tmp.path());
    let out = tmp.path().join("out");
    let merged = stage_card_only(&stage, &out, Some(&remote), &local)
        .unwrap()
        .unwrap();
    for name in CARD_FILES {
        let bytes = std::fs::read(out.join(name)).unwrap();
        let e = merged.files.iter().find(|f| f.path == name).unwrap();
        assert_eq!(e.blake3, vox_crypto::hash_fast_hex(&bytes), "{name}");
        assert_eq!(e.size_bytes, bytes.len() as u64, "{name}");
    }
    let svg_bytes = std::fs::read(out.join("assets/scorecard.svg")).unwrap();
    let svg_entry = merged
        .files
        .iter()
        .find(|f| f.path == "assets/scorecard.svg")
        .unwrap();
    assert_eq!(svg_entry.blake3, vox_crypto::hash_fast_hex(&svg_bytes));
    let staged: HfSyncManifest = serde_json::from_str(
        &std::fs::read_to_string(out.join(HF_SYNC_MANIFEST_FILENAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(staged, merged);
    for e in remote.files.iter().filter(|e| !is_card_collateral(&e.path)) {
        assert!(
            merged.files.contains(e),
            "non-card entry changed: {}",
            e.path
        );
    }
    assert_eq!(merged.files.len(), remote.files.len());
}

#[test]
fn card_only_stage_dir_holds_only_card_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (stage, local, remote) = local_and_remote(tmp.path());
    let names = |dir: &Path| {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };
    let with = tmp.path().join("with");
    stage_card_only(&stage, &with, Some(&remote), &local).unwrap();
    assert_eq!(
        names(&with),
        [
            "MODEL_CARD.md",
            "README.md",
            "assets",
            "hf_sync_manifest.json"
        ]
    );
    assert!(with.join("assets/scorecard.svg").is_file());
    let without = tmp.path().join("without");
    assert!(
        stage_card_only(&stage, &without, None, &local)
            .unwrap()
            .is_none()
    );
    assert_eq!(names(&without), ["MODEL_CARD.md", "README.md", "assets"]);
    assert!(without.join("assets/scorecard.svg").is_file());
    // A non-empty out dir is refused.
    assert!(stage_card_only(&stage, &with, None, &local).is_err());
}

#[test]
fn stage_card_only_copies_scorecard_svg_and_creates_assets_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let (stage, local, remote) = local_and_remote(tmp.path());
    let out = tmp.path().join("out_test");
    stage_card_only(&stage, &out, Some(&remote), &local).unwrap();
    assert!(out.join("assets").is_dir());
    assert!(out.join("assets/scorecard.svg").is_file());
}

#[test]
fn staged_card_that_disagrees_with_the_manifest_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (stage, local, _) = local_and_remote(tmp.path());
    std::fs::write(stage.join("README.md"), b"edited after hashing").unwrap();
    assert!(stage_card_only(&stage, &tmp.path().join("out"), None, &local).is_err());
}

#[test]
fn plan_card_push_cases() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, remote) = local_and_remote(tmp.path());
    let (l, r) = (Some(&local), Some(&remote));
    use SizeSyncStatus as S;
    assert_eq!(
        plan_card_push(S::NotTrained, false, None, None),
        CardPushPlan::SkipNotTrained
    );
    assert_eq!(
        plan_card_push(S::HubOnly, true, r, l),
        CardPushPlan::SkipHubOnly
    );
    assert_eq!(
        plan_card_push(S::InSync, true, l, l),
        CardPushPlan::Unchanged
    );
    assert_eq!(
        plan_card_push(S::Diverged, true, r, l),
        CardPushPlan::GatedWeights
    );
    assert_eq!(
        plan_card_push(S::Diverged, false, r, l),
        CardPushPlan::CardOnly {
            with_manifest: true
        }
    );
    assert_eq!(
        plan_card_push(S::LocalOnly, false, None, l),
        CardPushPlan::CardOnly {
            with_manifest: false
        }
    );
    // Same card bytes and label: unchanged even if the card entries differ elsewhere in the manifest.
    let mut other = local.clone();
    other.corpus_hash = "other".to_string();
    assert_eq!(
        plan_card_push(S::Diverged, false, Some(&other), l),
        CardPushPlan::Unchanged
    );
    // A different gate label alone is a change.
    other.gate_status = "passed".to_string();
    assert_eq!(
        plan_card_push(S::Diverged, false, Some(&other), l),
        CardPushPlan::CardOnly {
            with_manifest: true
        }
    );
}

#[test]
fn decide_rows_dry_run() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, remote) = local_and_remote(tmp.path());
    let mut r = row(SizeSyncStatus::Diverged);
    let plan = decide_row(&mut r, false, Some(&remote), Some(&local));
    assert_eq!(
        plan,
        CardPushPlan::CardOnly {
            with_manifest: true
        }
    );
    let text = r.action_taken.clone().unwrap();
    assert!(text.starts_with("would push"), "{text}");
    assert!(!text.contains("pushed"), "{text}");
    let mut r = row(SizeSyncStatus::InSync);
    assert_eq!(
        decide_row(&mut r, false, Some(&local), Some(&local)),
        CardPushPlan::Unchanged
    );
    assert_eq!(r.action_taken.as_deref(), Some("cards unchanged"));
    // The upload text only appears when a commit id is supplied.
    assert!(plan.action("1.7B", Some("abc")).contains("(abc)"));
}

#[test]
fn gated_current_is_left_to_the_weights_path() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, remote) = local_and_remote(tmp.path());
    let mut r = row(SizeSyncStatus::Diverged);
    let plan = decide_row(&mut r, true, Some(&remote), Some(&local));
    assert_eq!(plan, CardPushPlan::GatedWeights);
    assert!(
        r.action_taken
            .unwrap()
            .contains("vox mens hub sync --push --size 1.7B")
    );
}

#[test]
fn not_trained_row_is_never_uploaded() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, local, remote) = local_and_remote(tmp.path());
    for status in [SizeSyncStatus::NotTrained, SizeSyncStatus::HubOnly] {
        // Even with a gate that reads current and manifests on both sides.
        let mut r = row(status);
        let plan = decide_row(&mut r, true, Some(&remote), Some(&local));
        assert!(!matches!(
            plan,
            CardPushPlan::CardOnly { .. } | CardPushPlan::GatedWeights
        ));
        assert!(r.action_taken.unwrap().starts_with("skipped"));
    }
}

#[test]
fn scratch_dirs_are_private_distinct_and_leave_predictable_paths_alone() {
    // The old, predictable name must neither be reused nor deleted.
    let old = std::env::temp_dir().join(format!("vox-mens-cards-test-x-{}", std::process::id()));
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("sentinel"), b"1").unwrap();
    let (a, b) = (scratch("test", "x").unwrap(), scratch("test", "x").unwrap());
    assert_ne!(a.path(), b.path());
    assert_ne!(a.path(), old.as_path());
    assert!(a.path().is_dir() && std::fs::read_dir(a.path()).unwrap().next().is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(a.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "staging dir must be owner-only");
    }
    let path = a.path().to_path_buf();
    drop(a);
    assert!(!path.exists());
    assert!(old.join("sentinel").is_file());
    std::fs::remove_dir_all(&old).unwrap();
}
