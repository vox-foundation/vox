//! Corpus mix + primary-source sync for Mens training (single prep path for CLI / schola / pipeline).

use std::path::{Path, PathBuf};

use crate::corpus::{self, MixConfigSchema};

/// Relative path from workspace root to mix configuration.
pub const MIX_CONFIG_REL: &str = "mens/config/mix.yaml";

/// Whether two paths refer to the same existing file (handles relative vs absolute and `\\?\` on Windows).
fn same_existing_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    if !a.is_file() || !b.is_file() {
        return false;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// `VOX_TRAIN_SKIP_CORPUS_MIX=1|true` skips mix entirely (operators / tests).
#[must_use]
pub fn corpus_mix_skip_from_env() -> bool {
    std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Resolve mix YAML path: prefer workspace root; fall back to `cwd/mens/config/mix.yaml` (pipeline legacy).
pub fn resolve_mix_config_path(workspace_root: Option<&Path>) -> PathBuf {
    if let Some(root) = workspace_root {
        root.join(MIX_CONFIG_REL)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(MIX_CONFIG_REL)
    }
}

/// When the active `data_dir/train.jsonl` differs from mix’s primary source path, copy train into the
/// primary path so `run_mix` sees the latest pairs (workspace-relative primary paths).
pub fn sync_mix_primary_with_train_jsonl(
    workspace_root: Option<&Path>,
    data_dir: &Path,
    mix_yaml: &Path,
) -> anyhow::Result<()> {
    let Some(ws) = workspace_root else {
        return Ok(());
    };
    let Ok(cfg) = MixConfigSchema::load(mix_yaml) else {
        return Ok(());
    };
    let Some(primary) = cfg.sources.first() else {
        return Ok(());
    };
    let primary_resolved = ws.join(&primary.path);
    let train_jsonl = data_dir.join(super::preflight::PRIMARY_TRAIN_FILE);
    if same_existing_file(&primary_resolved, &train_jsonl) {
        tracing::debug!(
            path = %train_jsonl.display(),
            "mix primary path matches data_dir train.jsonl; skip redundant copy"
        );
        return Ok(());
    }
    if !train_jsonl.is_file() {
        tracing::warn!(
            active_train_jsonl = %train_jsonl.display(),
            "active train.jsonl missing; skip mix primary sync"
        );
        return Ok(());
    }
    if let Some(parent) = primary_resolved.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&train_jsonl, &primary_resolved).map_err(|e| {
        anyhow::anyhow!(
            "mix primary sync failed ({} -> {}): {e}",
            train_jsonl.display(),
            primary_resolved.display()
        )
    })?;
    Ok(())
}

/// Run corpus mix (when not skipped) and return the mixed JSONL path for [`super::preflight::validate_train_preflight`].
///
/// Mix output is always resolved relative to **`workspace_root`** when known (SSOT); avoids CWD drift.
pub fn refresh_train_contract_override_from_mix(
    workspace_root: Option<&Path>,
    data_dir: &Path,
    skip_mix: bool,
    sync_primary_with_data_dir_train: bool,
) -> anyhow::Result<Option<PathBuf>> {
    if skip_mix {
        return Ok(None);
    }
    let mix_yaml = resolve_mix_config_path(workspace_root);
    if !mix_yaml.is_file() {
        return Ok(None);
    }
    if sync_primary_with_data_dir_train {
        sync_mix_primary_with_train_jsonl(workspace_root, data_dir, &mix_yaml)?;
    }
    if let Err(e) = corpus::run_mix(&mix_yaml) {
        tracing::warn!(error = %e, mix_yaml = %mix_yaml.display(), "corpus mix failed; continuing with existing train files");
        return Ok(None);
    }
    let Ok(mix_cfg) = MixConfigSchema::load(&mix_yaml) else {
        return Ok(None);
    };
    let mix_output = match workspace_root {
        Some(ws) => ws.join(&mix_cfg.output),
        None => std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&mix_cfg.output),
    };
    if mix_output.is_file() {
        Ok(Some(mix_output))
    } else {
        Ok(None)
    }
}

/// After pipeline (or other steps) already produced a mixed file, copy mix output to `data_dir/train.jsonl`.
pub fn copy_mix_output_to_train_jsonl(
    workspace_root: &Path,
    data_dir: &Path,
    mix_yaml: &Path,
) -> anyhow::Result<bool> {
    if !mix_yaml.is_file() {
        return Ok(false);
    }
    let mix_cfg = MixConfigSchema::load(mix_yaml)?;
    let mixed_path = workspace_root.join(&mix_cfg.output);
    let final_train_path = data_dir.join(super::preflight::PRIMARY_TRAIN_FILE);
    if !mixed_path.is_file() {
        return Ok(false);
    }
    if same_existing_file(&mixed_path, &final_train_path) {
        return Ok(true);
    }
    if let Some(parent) = final_train_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&mixed_path, &final_train_path)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sync_copies_train_into_primary_when_paths_differ() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = tmp.path();
        let data = ws.join("target/dogfood");
        std::fs::create_dir_all(&data).expect("data dir");
        let mix_dir = ws.join("mens/config");
        std::fs::create_dir_all(&mix_dir).expect("mix dir");
        let primary = ws.join("mens/data/mix_sources/primary.jsonl");
        std::fs::create_dir_all(primary.parent().unwrap()).expect("parent");
        let train = data.join("train.jsonl");
        std::fs::write(&train, r#"{"a":1}"#).expect("write train");

        let mix_yaml = mix_dir.join("mix.yaml");
        let mut f = std::fs::File::create(&mix_yaml).expect("mix file");
        writeln!(
            f,
            "output: mens/data/mixed_out.jsonl\nsources:\n  - path: mens/data/mix_sources/primary.jsonl\n    weight: 1.0"
        )
        .expect("write mix");

        sync_mix_primary_with_train_jsonl(Some(ws), &data, &mix_yaml).expect("sync");
        let got = std::fs::read_to_string(&primary).expect("read primary");
        assert!(got.contains("\"a\":1"));
    }

    #[test]
    fn sync_skips_when_absolute_paths_differ_but_same_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("target/dogfood")).expect("dirs");
        std::fs::create_dir_all(ws.join("mens/config")).expect("mix dir");
        let train = ws.join("target/dogfood/train.jsonl");
        std::fs::write(&train, r#"{"x":1}"#).expect("train");

        let mix_yaml = ws.join("mens/config/mix.yaml");
        let mut f = std::fs::File::create(&mix_yaml).expect("mix");
        writeln!(
            f,
            "output: target/dogfood/train_mixed.jsonl\nsources:\n  - path: target/dogfood/train.jsonl\n    weight: 1.0"
        )
        .expect("write");

        let data = ws.join("target/dogfood");
        sync_mix_primary_with_train_jsonl(Some(ws), &data, &mix_yaml).expect("no self-copy");

        let got = std::fs::read_to_string(&train).expect("read");
        assert!(got.contains("\"x\":1"));
    }

    #[test]
    fn sync_skips_when_data_dir_is_relative_to_cwd_matching_primary() {
        static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = CWD_LOCK.lock().expect("cwd test lock");

        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("target/dogfood")).expect("dirs");
        std::fs::create_dir_all(ws.join("mens/config")).expect("mix dir");
        let train = ws.join("target/dogfood/train.jsonl");
        std::fs::write(&train, r#"{"y":2}"#).expect("train");

        let mix_yaml = ws.join("mens/config/mix.yaml");
        let mut f = std::fs::File::create(&mix_yaml).expect("mix");
        writeln!(
            f,
            "output: target/dogfood/train_mixed.jsonl\nsources:\n  - path: target/dogfood/train.jsonl\n    weight: 1.0"
        )
        .expect("write");

        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(ws).expect("cd tmp");
        let r = sync_mix_primary_with_train_jsonl(Some(ws), Path::new("target/dogfood"), &mix_yaml);
        std::env::set_current_dir(prev).expect("restore cwd");
        r.expect("relative data-dir must not self-copy on Windows");

        let got = std::fs::read_to_string(&train).expect("read");
        assert!(got.contains("\"y\":2"));
    }
}
