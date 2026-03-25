use std::env;
use std::fs;

use crate::types::AgentId;

use super::super::config::SessionConfig;
use super::super::errors::SessionError;
use super::super::state::{SessionState, now_secs};
use super::SessionManager;

fn temp_sessions_dir() -> std::path::PathBuf {
    static DIR_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let c = DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = env::temp_dir().join(format!("vox_sessions_{}_{c}", now_secs()));
    fs::create_dir_all(&d).ok();
    d
}

fn test_config() -> SessionConfig {
    SessionConfig {
        sessions_dir: temp_sessions_dir(),
        repository_id: None,
        idle_timeout_secs: 30,
        archive_timeout_secs: 60,
        max_sessions: 4,
        persist: true,
    }
}

#[test]
fn create_and_retrieve_session() {
    let mut mgr = SessionManager::new(test_config()).expect("create manager");
    let id = mgr.create(AgentId(1)).expect("create session");
    let session = mgr.get(&id).expect("get session");
    assert_eq!(session.agent_id, AgentId(1));
    assert_eq!(session.state, SessionState::Active);
    assert_eq!(session.turns.len(), 0);
}

#[test]
fn add_turn_and_check_tokens() {
    let mut mgr = SessionManager::new(test_config()).expect("create manager");
    let id = mgr.create(AgentId(1)).expect("create");
    mgr.add_turn(&id, "user", "hello world", 3)
        .expect("add turn");
    let s = mgr.get(&id).expect("get");
    assert_eq!(s.turns.len(), 1);
    assert_eq!(s.current_tokens(), 3);
    assert_eq!(s.turn_count, 1);
    assert_eq!(s.total_tokens, 3);
}

#[test]
fn reset_clears_history() {
    let mut mgr = SessionManager::new(test_config()).expect("create manager");
    let id = mgr.create(AgentId(1)).expect("create");
    mgr.add_turn(&id, "user", "hello", 2).expect("add");
    mgr.add_turn(&id, "assistant", "hi", 1).expect("add");
    let cleared = mgr.reset(&id).expect("reset");
    assert_eq!(cleared, 2);
    assert_eq!(mgr.get(&id).expect("get").turns.len(), 0);
}

#[test]
fn compact_replaces_with_summary() {
    let mut mgr = SessionManager::new(test_config()).expect("create manager");
    let id = mgr.create(AgentId(1)).expect("create");
    mgr.add_turn(&id, "user", "lots of content", 100)
        .expect("add");
    mgr.add_turn(&id, "assistant", "response", 50).expect("add");
    let removed = mgr
        .compact(&id, "Session summary: fixed parser")
        .expect("compact");
    assert_eq!(removed, 1); // 2 turns → replace with 1 summary → removed = 2-1
    assert_eq!(mgr.get(&id).expect("get").turns.len(), 1);
    assert_eq!(mgr.get(&id).expect("get").state, SessionState::Compacted);
}

#[test]
fn set_meta_persisted() {
    let mut mgr = SessionManager::new(test_config()).expect("create manager");
    let id = mgr.create(AgentId(1)).expect("create");
    mgr.set_meta(&id, "model", "claude-sonnet-4")
        .expect("set meta");
    let val = mgr.get(&id).expect("get").meta.get("model").cloned();
    assert_eq!(val.as_deref(), Some("claude-sonnet-4"));
}

#[tokio::test]
async fn session_persistence_roundtrip() {
    let cfg = test_config();
    let dir = cfg.sessions_dir.clone();
    let session_id;

    {
        let mut mgr = SessionManager::new(cfg.clone()).expect("create");
        session_id = mgr.create(AgentId(2)).expect("create session");
        mgr.add_turn(&session_id, "user", "fix parser", 10)
            .expect("add");
        mgr.set_meta(&session_id, "crate", "vox-parser")
            .expect("meta");
    }

    // Reload into fresh manager
    let mut mgr2 = SessionManager::new(SessionConfig {
        sessions_dir: dir,
        ..cfg
    })
    .expect("create");
    mgr2.load(&session_id).await.expect("load");
    let s = mgr2.get(&session_id).expect("get");
    assert_eq!(s.agent_id, AgentId(2));
    assert_eq!(s.turns.len(), 1);
    assert_eq!(s.meta.get("crate").map(|s| s.as_str()), Some("vox-parser"));
}

#[test]
fn max_sessions_limit() {
    let cfg = SessionConfig {
        max_sessions: 2,
        ..test_config()
    };
    let mut mgr = SessionManager::new(cfg).expect("create");
    mgr.create(AgentId(1)).expect("1st");
    mgr.create(AgentId(2)).expect("2nd");
    let err = mgr.create(AgentId(3));
    assert!(matches!(err, Err(SessionError::MaxSessions(2))));
}

#[test]
fn lifecycle_tick_marks_idle_then_archives() {
    let cfg = SessionConfig {
        idle_timeout_secs: 10,
        archive_timeout_secs: 10,
        ..test_config()
    };
    let mut mgr = SessionManager::new(cfg).expect("create");
    let id = mgr.create(AgentId(1)).expect("create");
    // Force last_active to be far in the past
    if let Some(s) = mgr.get_mut(&id) {
        s.last_active = now_secs().saturating_sub(20);
    }
    mgr.tick_lifecycle();
    mgr.tick_lifecycle();
    assert_eq!(mgr.get(&id).expect("get").state, SessionState::Archived);
}

#[test]
fn cleanup_removes_archived_sessions() {
    let cfg = SessionConfig {
        idle_timeout_secs: 1,
        archive_timeout_secs: 1,
        ..test_config()
    };
    let mut mgr = SessionManager::new(cfg).expect("create");
    let id = mgr.create(AgentId(1)).expect("create");
    if let Some(s) = mgr.get_mut(&id) {
        s.state = SessionState::Archived;
    }
    let removed = mgr.cleanup().expect("cleanup");
    assert_eq!(removed, 1);
    assert!(mgr.get(&id).is_none());
}

#[tokio::test]
async fn plugin_state_persistence_roundtrip() {
    let cfg = test_config();
    let dir = cfg.sessions_dir.clone();
    let session_id;

    {
        let mut mgr = SessionManager::new(cfg.clone()).expect("create");
        session_id = mgr.create(AgentId(3)).expect("create");
        mgr.set_plugin_state(
            &session_id,
            "weather",
            serde_json::json!({"city": "London"}),
        )
        .expect("set");
    }

    let mut mgr2 = SessionManager::new(SessionConfig {
        sessions_dir: dir,
        ..cfg
    })
    .expect("create");
    mgr2.load(&session_id).await.expect("load");
    let s = mgr2.get(&session_id).expect("get");
    assert_eq!(s.plugin_state.get("weather").unwrap()["city"], "London");
}
