//! One-time layout migrations for Unified Vox workspace paths (`.vox/sessions`, `.vox/memory`).

use std::path::Path;

/// Move `.sessions/<repository_id>` → `.vox/sessions/<repository_id>` when the modern path is absent.
pub fn migrate_legacy_sessions_into_vox(repo_root: &Path, repository_id: &str) {
    let legacy = repo_root.join(".sessions").join(repository_id);
    let modern_parent = repo_root.join(".vox").join("sessions");
    let modern = modern_parent.join(repository_id);
    if modern.exists() {
        return;
    }
    if !legacy.is_dir() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&modern_parent) {
        tracing::warn!(
            target: "vox_repository::workspace_path_migration",
            error = %e,
            path = %modern_parent.display(),
            "failed to create .vox/sessions parent"
        );
        return;
    }
    if std::fs::rename(&legacy, &modern).is_ok() {
        tracing::info!(
            target: "vox_repository::workspace_path_migration",
            from = %legacy.display(),
            to = %modern.display(),
            "migrated session JSONL directory to .vox/sessions"
        );
        return;
    }
    // Cross-device or non-empty parent: best-effort directory copy
    if let Err(e) = copy_dir_all_best_effort(&legacy, &modern) {
        tracing::warn!(
            target: "vox_repository::workspace_path_migration",
            error = %e,
            from = %legacy.display(),
            to = %modern.display(),
            "session directory copy failed after rename failed"
        );
    } else {
        tracing::info!(
            target: "vox_repository::workspace_path_migration",
            from = %legacy.display(),
            to = %modern.display(),
            "copied session JSONL directory to .vox/sessions"
        );
    }
}

/// Copy shard `.vox/cache/repos/<repository_id>/memory` → `.vox/memory` when canonical memory is missing.
pub fn migrate_legacy_memory_shard_into_vox_memory(repo_root: &Path, repository_id: &str) {
    let legacy_dir = repo_root
        .join(".vox")
        .join("cache")
        .join("repos")
        .join(repository_id)
        .join("memory");
    let modern_dir = repo_root.join(".vox").join("memory");
    let modern_md = modern_dir.join("MEMORY.md");
    if modern_md.is_file() {
        return;
    }
    let legacy_md = legacy_dir.join("MEMORY.md");
    if !legacy_md.is_file() && !legacy_dir.is_dir() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&modern_dir) {
        tracing::warn!(
            target: "vox_repository::workspace_path_migration",
            error = %e,
            path = %modern_dir.display(),
            "failed to create .vox/memory"
        );
        return;
    }
    if legacy_md.is_file() {
        let _ = std::fs::copy(&legacy_md, &modern_md);
    }
    // Daily logs under legacy_dir
    if let Ok(read) = std::fs::read_dir(&legacy_dir) {
        for ent in read.flatten() {
            let name = ent.file_name();
            let n = name.to_string_lossy();
            if n.ends_with(".md") && n != "MEMORY.md" {
                let dest = modern_dir.join(&name);
                if !dest.exists() {
                    let _ = std::fs::copy(ent.path(), &dest);
                }
            }
        }
    }
    tracing::info!(
        target: "vox_repository::workspace_path_migration",
        legacy = %legacy_dir.display(),
        modern = %modern_dir.display(),
        "migrated memory shard into .vox/memory"
    );
}

fn copy_dir_all_best_effort(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for ent in std::fs::read_dir(src)? {
        let ent = ent?;
        let ty = ent.file_type()?;
        let dest_path = dst.join(ent.file_name());
        if ty.is_dir() {
            copy_dir_all_best_effort(&ent.path(), &dest_path)?;
        } else {
            let _ = std::fs::copy(ent.path(), &dest_path)?;
        }
    }
    Ok(())
}
