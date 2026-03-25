use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::types::AgentId;

use super::config::MemoryConfig;
use super::daily_log::DailyLog;
use super::long_term::LongTermMemory;
use super::manager::MemoryManager;
use super::time::{timestamp_hms, unix_secs_to_ymd};

static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = env::temp_dir().join(format!(
        "vox_memory_test_{}_{}",
        timestamp_hms().replace(':', ""),
        n
    ));
    fs::create_dir_all(&d).ok();
    d
}

#[test]
fn daily_log_append_and_read() {
    let dir = temp_dir();
    let log = DailyLog::open(&dir, "2026-02-27").expect("open");
    log.append("compiler fixed").expect("append");
    let content = log.read().expect("read");
    assert!(content.contains("compiler fixed"));
    assert!(content.contains("2026-02-27"));
}

#[test]
fn daily_log_multiple_appends() {
    let dir = temp_dir();
    let log = DailyLog::open(&dir, "2026-02-28").expect("open");
    log.append("first entry").expect("append");
    log.append("second entry").expect("append");
    let content = log.read().expect("read");
    assert!(content.contains("first entry"));
    assert!(content.contains("second entry"));
}

#[test]
fn long_term_memory_set_and_get() {
    let dir = temp_dir();
    let mem = LongTermMemory::open(&dir.join("MEMORY.md")).expect("open");
    mem.set("current_crate", "vox-parser").expect("set");
    let val = mem.get("current_crate").expect("get");
    assert_eq!(val.as_deref(), Some("vox-parser"));
}

#[test]
fn long_term_memory_upsert() {
    let dir = temp_dir();
    let mem = LongTermMemory::open(&dir.join("MEMORY.md")).expect("open");
    mem.set("status", "in progress").expect("set first");
    mem.set("status", "completed").expect("set second (upsert)");
    let val = mem.get("status").expect("get");
    assert_eq!(val.as_deref(), Some("completed"));
}

#[test]
fn long_term_memory_list_keys() {
    let dir = temp_dir();
    let mem = LongTermMemory::open(&dir.join("MEMORY.md")).expect("open");
    mem.set("alpha", "a").expect("set");
    mem.set("beta", "b").expect("set");
    let keys = mem.list_keys().expect("list");
    assert!(keys.contains(&"alpha".to_string()));
    assert!(keys.contains(&"beta".to_string()));
}

#[test]
fn memory_manager_persist_and_recall() {
    let dir = temp_dir();
    let mut mgr = MemoryManager::new(MemoryConfig {
        log_dir: dir.join("logs"),
        memory_md_path: dir.join("MEMORY.md"),
        log_retention_days: 7,
        enabled: true,
    })
    .expect("create");
    mgr.persist_fact(AgentId(1), "last_task", "fix parser", &[], None, None)
        .expect("persist");
    let val = mgr.recall("last_task").expect("recall");
    assert_eq!(val.as_deref(), Some("fix parser"));
}

#[test]
fn memory_manager_bootstrap_context() {
    let dir = temp_dir();
    let mut mgr = MemoryManager::new(MemoryConfig {
        log_dir: dir.join("logs"),
        memory_md_path: dir.join("MEMORY.md"),
        log_retention_days: 7,
        enabled: true,
    })
    .expect("create");
    mgr.persist_fact(AgentId(1), "project", "vox", &[], None, None)
        .expect("persist");
    mgr.log("started session").expect("log");
    let ctx = mgr.bootstrap_context();
    assert!(ctx.contains("project"));
    assert!(ctx.contains("vox"));
}

#[test]
fn memory_manager_search() {
    let dir = temp_dir();
    let mut mgr = MemoryManager::new(MemoryConfig {
        log_dir: dir.join("logs"),
        memory_md_path: dir.join("MEMORY.md"),
        log_retention_days: 7,
        enabled: true,
    })
    .expect("create");
    mgr.log("fixed the parser bug").expect("log");
    mgr.persist_fact(
        AgentId(1),
        "active_branch",
        "feat/parser-fix",
        &[],
        None,
        None,
    )
    .expect("persist");
    let hits = mgr.search("parser").expect("search");
    assert!(!hits.is_empty(), "should find 'parser' in memory");
}

#[test]
fn flush_before_compaction_persists_facts() {
    let dir = temp_dir();
    let mut mgr = MemoryManager::new(MemoryConfig {
        log_dir: dir.join("logs"),
        memory_md_path: dir.join("MEMORY.md"),
        log_retention_days: 7,
        enabled: true,
    })
    .expect("create");
    let mut facts = HashMap::new();
    facts.insert(
        "lock_file".to_string(),
        "crates/vox-parser/src/parser.rs".to_string(),
    );
    facts.insert("agent_state".to_string(), "building".to_string());
    let flushed = mgr
        .flush_before_compaction(AgentId(1), facts)
        .expect("flush");
    assert_eq!(flushed, 2);
    assert!(mgr.recall("lock_file").expect("recall").is_some());
    assert!(mgr.recall("agent_state").expect("recall").is_some());
}

#[test]
fn disabled_memory_manager_returns_empty_context() {
    let dir = temp_dir();
    let mgr = MemoryManager::new(MemoryConfig {
        log_dir: dir.join("logs"),
        memory_md_path: dir.join("MEMORY.md"),
        log_retention_days: 7,
        enabled: false,
    })
    .expect("create");
    let ctx = mgr.bootstrap_context();
    assert!(
        ctx.is_empty(),
        "disabled memory should return empty context"
    );
}

#[test]
fn unix_secs_to_ymd_basic() {
    // 2026-02-27 00:00:00 UTC = 1772150400 secs
    let (y, m, d) = unix_secs_to_ymd(1_772_150_400);
    assert_eq!(y, 2026);
    assert_eq!(m, 2);
    assert_eq!(d, 27);
}
