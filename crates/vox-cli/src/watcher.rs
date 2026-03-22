//! Notify-based file watch for rebuild loops (`vox-compilerd` `dev`, future `--watch` flags).

use anyhow::Context;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;

/// For each filesystem modify event that targets `file_canon`, call `on_hit`.
///
/// Runs until the notify channel closes (normally never while the watcher lives).
pub(crate) async fn each_modify_hit<F, Fut>(
    file_canon: PathBuf,
    watch_dir: PathBuf,
    mut on_hit: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<notify::Event>();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                let _ = tx.send(ev);
            }
        },
        Config::default(),
    )
    .context("failed to create notify watcher")?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("failed to watch {}", watch_dir.display()))?;

    let _keep_watcher_alive = watcher;

    while let Some(event) = rx.recv().await {
        if !matches!(event.kind, EventKind::Modify(_)) {
            continue;
        }
        let mut hit = false;
        for path in &event.paths {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            if abs == file_canon {
                hit = true;
                break;
            }
        }
        if hit {
            on_hit().await;
        }
    }

    Ok(())
}
