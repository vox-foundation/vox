//! Workspace source, manifest, contract, and directory modification-time scanner
//! used to invalidate stale coalesced build receipts after edits or deletions.

use std::path::Path;

pub(crate) fn file_mtime_ms(p: &Path) -> u128 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn max_mtime_walk(dir: &Path, acc: &mut u128) {
    *acc = (*acc).max(file_mtime_ms(dir));
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if matches!(
            name_str,
            "target"
                | ".git"
                | "node_modules"
                | ".sccache"
                | ".planning"
                | "docs"
                | ".vox"
                | ".claude"
                | ".cursor"
        ) {
            continue;
        }
        let Ok(ft) = entry.file_type() else { continue };
        let p = entry.path();
        if ft.is_dir() {
            *acc = (*acc).max(file_mtime_ms(&p));
            max_mtime_walk(&p, acc);
        } else if ft.is_file() {
            *acc = (*acc).max(file_mtime_ms(&p));
        }
    }
}

/// Maximum source/manifest/contract/directory modification timestamp (epoch ms) in `worktree`.
pub fn latest_source_mtime_ms(worktree: &Path) -> u128 {
    let mut max_ms = [
        worktree.to_path_buf(),
        worktree.join("Cargo.toml"),
        worktree.join("Cargo.lock"),
        worktree.join(".cargo"),
        worktree.join(".cargo").join("config.toml"),
        worktree.join("build.rs"),
        worktree.join("rust-toolchain.toml"),
        worktree.join("rust-toolchain"),
        worktree.join("lefthook.yml"),
    ]
    .iter()
    .map(|p| file_mtime_ms(p))
    .max()
    .unwrap_or(0);

    let crates_dir = worktree.join("crates");
    if crates_dir.is_dir() {
        for sub in ["crates", "src", "contracts", ".agents"] {
            let tree = worktree.join(sub);
            if tree.is_dir() {
                max_mtime_walk(&tree, &mut max_ms);
            }
        }
    } else {
        max_mtime_walk(worktree, &mut max_ms);
    }
    max_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn tracks_edits_deletions_contracts_agents_toolchain_and_ignores_target() {
        let tmp = tempfile::tempdir().unwrap();
        let wt = tmp.path();
        std::fs::create_dir_all(wt.join("crates/demo/src")).unwrap();
        std::fs::create_dir_all(wt.join("contracts")).unwrap();
        std::fs::create_dir_all(wt.join(".agents")).unwrap();
        std::fs::create_dir_all(wt.join("target/debug")).unwrap();
        std::fs::write(wt.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(wt.join("crates/demo/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(wt.join("crates/demo/src/extra.rs"), "pub fn x() {}\n").unwrap();

        let mut prev = latest_source_mtime_ms(wt);
        assert!(prev > 0);

        std::thread::sleep(Duration::from_millis(25));
        std::fs::write(wt.join("target/debug/junk"), "ignored").unwrap();
        assert_eq!(prev, latest_source_mtime_ms(wt));

        std::thread::sleep(Duration::from_millis(25));
        std::fs::remove_file(wt.join("crates/demo/src/extra.rs")).unwrap();
        let after_del = latest_source_mtime_ms(wt);
        assert!(after_del > prev, "file deletion must advance dir mtime");
        prev = after_del;

        for rel in [
            "contracts/policy.yaml",
            ".agents/cargo-safety.md",
            "rust-toolchain.toml",
            "lefthook.yml",
        ] {
            std::thread::sleep(Duration::from_millis(25));
            std::fs::write(wt.join(rel), "updated\n").unwrap();
            let next = latest_source_mtime_ms(wt);
            assert!(
                next > prev,
                "{rel} edit must advance latest_source_mtime_ms"
            );
            prev = next;
        }
    }
}
