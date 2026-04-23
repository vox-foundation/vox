//! Process-wide cache for in-memory BM25 indices keyed by memory paths + mtimes.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::context::SearchRuntimeContext;
use crate::memory_hybrid::MemorySearchEngine;

struct CacheEntry {
    log_dir: PathBuf,
    md_path: PathBuf,
    log_stamp_nanos: u128,
    md_stamp_nanos: u128,
}

fn mtime_nanos(path: &std::path::Path) -> u128 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Returns a [`MemorySearchEngine`] indexed for `ctx`'s log dir + standalone md path.
///
/// Reuses the last build when paths and directory/file mtimes are unchanged (best-effort).
pub(crate) fn cached_memory_engine(ctx: &SearchRuntimeContext) -> MemorySearchEngine {
    static CACHE: Mutex<Option<(CacheEntry, MemorySearchEngine)>> = Mutex::new(None);
    let log_stamp = mtime_nanos(&ctx.memory_log_dir);
    let md_stamp = mtime_nanos(&ctx.memory_md_path);
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache_hit = guard.as_ref().is_some_and(|(e, _)| {
        e.log_dir == ctx.memory_log_dir
            && e.md_path == ctx.memory_md_path
            && e.log_stamp_nanos == log_stamp
            && e.md_stamp_nanos == md_stamp
    });
    if cache_hit {
        return guard.as_ref().expect("cache_hit implies Some").1.clone();
    }
    let mut engine = MemorySearchEngine::new();
    engine.index_dir(&ctx.memory_log_dir);
    if !ctx.memory_md_path.starts_with(&ctx.memory_log_dir) {
        engine.index_file(&ctx.memory_md_path);
    }
    *guard = Some((
        CacheEntry {
            log_dir: ctx.memory_log_dir.clone(),
            md_path: ctx.memory_md_path.clone(),
            log_stamp_nanos: log_stamp,
            md_stamp_nanos: md_stamp,
        },
        engine.clone(),
    ));
    engine
}
