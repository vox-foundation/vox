//! Card-only push for `vox mens hub sync --cards-only` (phase 339, CARD-12).
//!
//! The README is inside the hashed bundle manifest, so a card change makes a published size
//! `Diverged`, and the ordinary `--push` would re-publish the whole weights bundle under a new tag.
//! This path uploads only `README.md`, `MODEL_CARD.md` and (when the repo has one) a rewritten
//! `hf_sync_manifest.json`. A folder upload adds or overwrites only the staged paths and deletes
//! nothing without `delete_patterns` (hf-hub 1.0.0 `upload_folder_impl`), which
//! [`upload_model_folder`] never sets.
//!
//! The rewritten manifest differs from the remote one in the two card entries and `gate_status`.
//! `gate_status` is the label the local inspection computes (the `stage_publish_bundle(.., true, ..)`
//! call `sync_all_sizes` makes), so an otherwise unchanged size classifies `InSync` afterwards.
//!
//! No weights, no `v<N>` tag, no ungated publish: sizes whose gate is current go through the
//! gated `vox mens hub sync --push --size <S>` path. No card wording lives here.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::mens::hub::upload_model_folder;
use crate::mens::hub_card::{GateState, RunRecords, gate_of};
use crate::mens::hub_repo::{self, NAMESPACE_ENV};
use crate::mens::hub_sync::{
    FileHashEntry, HF_SYNC_MANIFEST_FILENAME, HfSyncManifest, SizeSyncRow, SizeSyncStatus,
    classify_size_sync, fetch_remote_sync_manifest, refresh_ladder_index, require_clavis_hf_token,
    size_in_scope, stage_publish_bundle, sync_all_sizes,
};

/// The only files a card-only push uploads besides the rewritten manifest.
pub const CARD_FILES: [&str; 2] = ["README.md", "MODEL_CARD.md"];

/// True when `rel_path` is a card or card-visual asset (e.g. SVG dashboard) that participates in card-only sync.
pub fn is_card_collateral(rel_path: &str) -> bool {
    CARD_FILES.contains(&rel_path)
        || (rel_path.starts_with("assets/") && rel_path.ends_with(".svg"))
}

/// True when `entry` is a card/visual asset OR a non-weight metadata file (`!*.safetensors`)
/// whose local digest differs from `remote`.
pub fn is_card_or_changed_metadata(entry: &FileHashEntry, remote: Option<&HfSyncManifest>) -> bool {
    if is_card_collateral(&entry.path) {
        return true;
    }
    if entry.path.ends_with(".safetensors") || entry.path == "tokenizer.json" {
        return false;
    }
    remote.is_some_and(|r| r.files.iter().find(|f| f.path == entry.path) != Some(entry))
}

/// True when `remote` has the exact same `.safetensors` weight file entries as `local`.
pub fn remote_weights_match(remote: Option<&HfSyncManifest>, local: &HfSyncManifest) -> bool {
    let Some(r) = remote else {
        return false;
    };
    let r_weights: Vec<&FileHashEntry> = r
        .files
        .iter()
        .filter(|f| f.path.ends_with(".safetensors"))
        .collect();
    let l_weights: Vec<&FileHashEntry> = local
        .files
        .iter()
        .filter(|f| f.path.ends_with(".safetensors"))
        .collect();
    !l_weights.is_empty() && r_weights == l_weights
}

/// What `--cards-only` does for one ladder size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardPushPlan {
    SkipNotTrained,
    SkipHubOnly,
    /// The Hub already carries these card bytes and this gate label.
    Unchanged,
    /// The gate is current: weights go through the gated publish path.
    GatedWeights,
    CardOnly {
        with_manifest: bool,
    },
}

impl CardPushPlan {
    /// The `action_taken` text; `pushed` is the commit id once the upload happened.
    pub fn action(&self, size: &str, pushed: Option<&str>) -> String {
        match (self, pushed) {
            (Self::SkipNotTrained, _) => "skipped (not trained)".to_string(),
            (Self::SkipHubOnly, _) => "skipped (hub only)".to_string(),
            (Self::Unchanged, _) => "cards unchanged".to_string(),
            (Self::GatedWeights, _) => {
                format!("gate current, use: vox mens hub sync --push --size {size}")
            }
            (Self::CardOnly { .. }, Some(oid)) => format!("cards pushed ({oid})"),
            (Self::CardOnly { with_manifest }, None) => format!(
                "would push cards only (no weights, no tag{})",
                if *with_manifest {
                    ", manifest rewritten"
                } else {
                    ""
                }
            ),
        }
    }
}

fn card_entries<'a>(m: &'a HfSyncManifest) -> Vec<&'a FileHashEntry> {
    let mut entries: Vec<&'a FileHashEntry> = m
        .files
        .iter()
        .filter(|f| is_card_collateral(&f.path))
        .collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

/// Pure per-size decision. `local` is the manifest of the locally staged bundle.
pub fn plan_card_push(
    status: SizeSyncStatus,
    gate_current: bool,
    remote: Option<&HfSyncManifest>,
    local: Option<&HfSyncManifest>,
) -> CardPushPlan {
    match status {
        SizeSyncStatus::NotTrained => return CardPushPlan::SkipNotTrained,
        SizeSyncStatus::HubOnly => return CardPushPlan::SkipHubOnly,
        _ => {}
    }
    let Some(local) = local else {
        return CardPushPlan::SkipNotTrained;
    };
    if let Some(r) = remote {
        let metadata_changed = local
            .files
            .iter()
            .any(|e| !is_card_collateral(&e.path) && is_card_or_changed_metadata(e, Some(r)));
        if card_entries(r) == card_entries(local)
            && r.gate_status == local.gate_status
            && !metadata_changed
        {
            return CardPushPlan::Unchanged;
        }
    }
    if gate_current {
        return CardPushPlan::GatedWeights;
    }
    CardPushPlan::CardOnly {
        with_manifest: remote.is_some(),
    }
}

/// The remote manifest with the card entries, changed non-weight metadata entries, and `gate_status`
/// taken from `local`. Every weight entry is preserved from `remote`, so `vox mens hub pull` re-hash checks pass.
pub fn card_only_manifest(remote: &HfSyncManifest, local: &HfSyncManifest) -> HfSyncManifest {
    let mut m = remote.clone();
    for entry in &local.files {
        if is_card_or_changed_metadata(entry, Some(remote)) {
            m.files.retain(|f| f.path != entry.path);
            m.files.push(entry.clone());
        }
    }
    m.files.retain(|f| {
        f.path.ends_with(".safetensors")
            || f.path == "tokenizer.json"
            || local.files.iter().any(|lf| lf.path == f.path)
    });
    m.files.sort_by(|a, b| a.path.cmp(&b.path));
    m.gate_status = local.gate_status.clone();
    m
}

/// Copy the card files, visual assets, and changed non-weight metadata from `local_stage` into the empty dir `out`
/// (checking each against the local manifest) and, when the repo has a manifest, write the merged one beside them.
pub fn stage_card_only(
    local_stage: &Path,
    out: &Path,
    remote: Option<&HfSyncManifest>,
    local: &HfSyncManifest,
) -> Result<Option<HfSyncManifest>> {
    std::fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;
    if std::fs::read_dir(out)?.next().is_some() {
        bail!("card-only stage dir {} is not empty", out.display());
    }
    for entry in &local.files {
        if !is_card_or_changed_metadata(entry, remote) {
            continue;
        }
        let src = local_stage.join(&entry.path);
        let bytes = std::fs::read(&src)
            .with_context(|| format!("read staged {} in {}", entry.path, local_stage.display()))?;
        if entry.blake3 != vox_crypto::hash_fast_hex(&bytes)
            || entry.size_bytes != bytes.len() as u64
        {
            bail!(
                "staged {} does not match the local manifest entry",
                entry.path
            );
        }
        let dst = out.join(&entry.path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dst, bytes)?;
    }
    let Some(remote) = remote else {
        return Ok(None);
    };
    let merged = card_only_manifest(remote, local);
    std::fs::write(
        out.join(HF_SYNC_MANIFEST_FILENAME),
        format!("{}\n", serde_json::to_string_pretty(&merged)?),
    )?;
    Ok(Some(merged))
}

/// Set the row's `action_taken` to the plan text (never an upload) and return the plan.
pub fn decide_row(
    row: &mut SizeSyncRow,
    gate_current: bool,
    remote: Option<&HfSyncManifest>,
    local: Option<&HfSyncManifest>,
) -> CardPushPlan {
    let plan = plan_card_push(row.sync_status, gate_current, remote, local);
    row.action_taken = Some(plan.action(&row.size, None));
    plan
}

/// Fresh private temp dir (random name, exclusive create, mode 0700 on unix), removed when dropped.
/// tempfile's default mode follows the umask, so 0700 is requested explicitly.
pub(crate) fn scratch(tag: &str, size: &str) -> Result<tempfile::TempDir> {
    let prefix = format!("vox-mens-cards-{tag}-{size}-");
    let mut b = tempfile::Builder::new();
    b.prefix(&prefix);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        b.permissions(std::fs::Permissions::from_mode(0o700));
    }
    b.tempdir()
        .with_context(|| format!("create {tag} staging dir for {size}"))
}

/// Stage and upload the card-only folder for one row; returns the commit id.
async fn push_one(
    row: &SizeSyncRow,
    stage: &Path,
    remote: Option<&HfSyncManifest>,
    local: &HfSyncManifest,
    private: bool,
) -> Result<String> {
    let out = scratch("out", &row.size)?;
    stage_card_only(stage, out.path(), remote, local)?;
    upload_model_folder(
        &row.repo_id,
        out.path(),
        private,
        Some(&format!(
            "Update VoxMens Qwen3-{} model card (no weights)",
            row.size
        )),
    )
    .await
}

/// Render every card locally (the dry run) and, with `push`, upload card-only folders for each
/// trained size whose gate is not current, then refresh `vox-mens-index` once.
pub async fn sync_cards_only(
    workspace_root: Option<&Path>,
    namespace: Option<&str>,
    private: bool,
    only_size: Option<&str>,
    push: bool,
) -> Result<Vec<SizeSyncRow>> {
    if push {
        let _ = require_clavis_hf_token()?;
    }
    let mut rows = sync_all_sizes(
        workspace_root,
        namespace,
        false,
        false,
        false,
        private,
        only_size,
    )
    .await?;
    let ns_env = std::env::var(NAMESPACE_ENV).ok();
    let ns = hub_repo::resolve_namespace(namespace.or(ns_env.as_deref()))?;
    let root = workspace_root.unwrap_or(Path::new("."));

    for row in rows
        .iter_mut()
        .filter(|r| size_in_scope(&r.size, only_size))
    {
        if matches!(
            row.sync_status,
            SizeSyncStatus::NotTrained | SizeSyncStatus::HubOnly
        ) {
            decide_row(row, false, None, None);
            continue;
        }
        let run_dir = root.join(&row.local_dir);
        let records = RunRecords::load(&run_dir, &row.size, &row.base_pin);
        let gate_current = gate_of(&records).state == GateState::Passed;
        let stage = match scratch("local", &row.size) {
            Ok(d) => d,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: {e}"));
                continue;
            }
        };
        let local = match stage_publish_bundle(&run_dir, stage.path(), true, &ns) {
            Ok(m) => m,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: stage: {e}"));
                continue;
            }
        };
        // ponytail: fetch_remote_sync_manifest reads a failed repo info() as "no manifest";
        // upload would fail the same way. Revisit if it grows an Err path.
        let remote = match fetch_remote_sync_manifest(&row.repo_id).await {
            Ok(m) => m,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: remote manifest: {e}"));
                continue;
            }
        };
        row.sync_status = classify_size_sync(Some(&local), remote.as_ref());
        let plan = decide_row(row, gate_current, remote.as_ref(), Some(&local));
        let CardPushPlan::CardOnly { with_manifest } = plan else {
            continue;
        };
        if !push {
            continue;
        }
        match push_one(row, stage.path(), remote.as_ref(), &local, private).await {
            Ok(oid) => {
                row.action_taken = Some(format!("cards pushed ({oid})"));
                if with_manifest && remote_weights_match(remote.as_ref(), &local) {
                    row.sync_status = SizeSyncStatus::InSync;
                }
            }
            Err(e) => row.action_taken = Some(format!("cards_failed: {e}")),
        }
    }

    if push {
        refresh_ladder_index(workspace_root, Some(&ns), private).await?;
    }
    Ok(rows)
}

/// Push card and non-weight metadata updates for any `Diverged` row whose remote `.safetensors`
/// weights already match local, updating `row.sync_status` to `InSync` in place.
pub async fn sync_diverged_cards_in_place(
    rows: &mut [SizeSyncRow],
    root: &Path,
    ns: &str,
    private: bool,
) {
    for row in rows.iter_mut() {
        if row.sync_status != SizeSyncStatus::Diverged {
            continue;
        }
        let run_dir = root.join(&row.local_dir);
        let stage = match scratch("auto", &row.size) {
            Ok(d) => d,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: {e}"));
                continue;
            }
        };
        let local = match stage_publish_bundle(&run_dir, stage.path(), true, ns) {
            Ok(m) => m,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: stage: {e}"));
                continue;
            }
        };
        let remote = match fetch_remote_sync_manifest(&row.repo_id).await {
            Ok(m) => m,
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: remote manifest: {e}"));
                continue;
            }
        };
        if !remote_weights_match(remote.as_ref(), &local) {
            continue;
        }
        match push_one(row, stage.path(), remote.as_ref(), &local, private).await {
            Ok(oid) => {
                row.action_taken = Some(format!("cards pushed ({oid})"));
                if remote_weights_match(remote.as_ref(), &local) {
                    row.sync_status = SizeSyncStatus::InSync;
                }
            }
            Err(e) => {
                row.action_taken = Some(format!("cards_failed: {e}"));
            }
        }
    }
}

#[cfg(test)]
#[path = "hub_cards_push_tests.rs"]
mod tests;
