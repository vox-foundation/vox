use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::types::AgentId;

use super::super::errors::SessionError;
use super::super::state::{Session, SessionEvent, SessionState, SessionTurn, now_secs};
use super::SessionManager;

impl SessionManager {
    /// Load a session by replaying events from JSONL.
    #[deprecated(note = "JSONL persistence is being replaced by VoxDb SSOT")]
    pub fn load_from_jsonl(&mut self, session_id: &str) -> Result<(), SessionError> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Err(SessionError::NotFound(session_id.to_string()));
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        let mut session: Option<Session> = None;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim().trim_start_matches('\u{feff}');
            if line.is_empty() {
                continue;
            }
            // One logical JSONL row can contain multiple concatenated objects if writers interleave
            // before newline (stress / coverage); stream-parse every value on the line.
            let iter = serde_json::Deserializer::from_str(line).into_iter::<SessionEvent>();
            for ev in iter {
                let event: SessionEvent = ev.map_err(SessionError::Serialize)?;
                match event {
                    SessionEvent::Created {
                        session_id: sid,
                        agent_id,
                        created_at,
                    } => {
                        let now = now_secs();
                        session = Some(Session {
                            id: sid,
                            agent_id: AgentId(agent_id),
                            state: SessionState::Active,
                            created_at,
                            last_active: now,
                            last_expensive_op_at: None,
                            turns: Vec::new(),
                            meta: HashMap::new(),
                            plugin_state: HashMap::new(),
                            turn_count: 0,
                            total_tokens: 0,
                        });
                    }
                    SessionEvent::TurnAdded {
                        role,
                        content,
                        tokens,
                        at,
                    } => {
                        if let Some(ref mut s) = session {
                            s.turns.push(SessionTurn {
                                role,
                                content,
                                tokens,
                                at,
                            });
                            s.turn_count += 1;
                            s.total_tokens += tokens;
                        }
                    }
                    SessionEvent::StateChanged { to, .. } => {
                        if let Some(ref mut s) = session {
                            s.state = to;
                        }
                    }
                    SessionEvent::MetaUpdated { key, value, .. } => {
                        if let Some(ref mut s) = session {
                            s.meta.insert(key, value);
                        }
                    }
                    SessionEvent::PluginStateUpdated {
                        plugin_id, state, ..
                    } => {
                        if let Some(ref mut s) = session {
                            s.plugin_state.insert(plugin_id, state);
                        }
                    }
                    SessionEvent::Reset { .. } => {
                        if let Some(ref mut s) = session {
                            s.turns.clear();
                            s.state = SessionState::Active;
                        }
                    }
                    SessionEvent::Compacted {
                        summary,
                        turns_removed: _,
                        at,
                    } => {
                        if let Some(ref mut s) = session {
                            let tokens =
                                crate::compaction::CompactionEngine::estimate_tokens(&summary);
                            s.turns.clear();
                            s.turns.push(SessionTurn {
                                role: "system".to_string(),
                                content: format!("[compacted summary]\n{summary}"),
                                tokens,
                                at,
                            });
                            s.state = SessionState::Compacted;
                        }
                    }
                    SessionEvent::ExpensiveOpRecorded { at } => {
                        if let Some(ref mut s) = session {
                            s.last_expensive_op_at = Some(at);
                        }
                    }
                }
            }
        }

        if let Some(s) = session {
            self.sessions.insert(s.id.clone(), s);
        }
        Ok(())
    }

    /// Load a session by ID. Checks VoxDb first, falls back to JSONL.
    pub async fn load(&mut self, session_id: &str) -> Result<(), SessionError> {
        if let Some(db) = &self.db {
            let db = db.clone();
            let session_rows =
                db.list_active_sessions()
                    .await
                    .map_err(|e: vox_db::StoreError| {
                        SessionError::Io(std::io::Error::other(e.to_string()))
                    })?;

            if let Some((_, agent_id_str, _)) =
                session_rows.iter().find(|(sid, _, _)| sid == session_id)
            {
                let aid = agent_id_str.parse::<AgentId>().unwrap_or(AgentId(0));
                let mut session = Session {
                    id: session_id.to_string(),
                    agent_id: aid,
                    state: SessionState::Active,
                    created_at: now_secs(), // Replaced by Create event if found
                    last_active: now_secs(),
                    last_expensive_op_at: None,
                    turns: Vec::new(),
                    meta: HashMap::new(),
                    plugin_state: HashMap::new(),
                    turn_count: 0,
                    total_tokens: 0,
                };

                let events =
                    db.load_session_events(session_id)
                        .await
                        .map_err(|e: vox_db::StoreError| {
                            SessionError::Io(std::io::Error::other(e.to_string()))
                        })?;

                for (_etype, payload_json) in events {
                    let event: SessionEvent =
                        serde_json::from_str(&payload_json).map_err(SessionError::Serialize)?;
                    self.apply_event_to_session(&mut session, event);
                }

                self.sessions.insert(session_id.to_string(), session);
                return Ok(());
            }
        }

        #[allow(deprecated)]
        self.load_from_jsonl(session_id)
    }

    /// Helper to apply an event to a session object.
    fn apply_event_to_session(&self, s: &mut Session, event: SessionEvent) {
        match event {
            SessionEvent::Created {
                created_at,
                agent_id,
                ..
            } => {
                s.created_at = created_at;
                s.agent_id = AgentId(agent_id);
            }
            SessionEvent::TurnAdded {
                role,
                content,
                tokens,
                at,
            } => {
                s.turns.push(SessionTurn {
                    role,
                    content,
                    tokens,
                    at,
                });
                s.turn_count += 1;
                s.total_tokens += tokens;
                s.last_active = at;
            }
            SessionEvent::StateChanged { to, at, .. } => {
                s.state = to;
                s.last_active = at;
            }
            SessionEvent::MetaUpdated { key, value, at } => {
                s.meta.insert(key, value);
                s.last_active = at;
            }
            SessionEvent::PluginStateUpdated {
                plugin_id,
                state,
                at,
            } => {
                s.plugin_state.insert(plugin_id, state);
                s.last_active = at;
            }
            SessionEvent::Reset { at } => {
                s.turns.clear();
                s.state = SessionState::Active;
                s.last_active = at;
            }
            SessionEvent::Compacted { summary, at, .. } => {
                let tokens = crate::compaction::CompactionEngine::estimate_tokens(&summary);
                s.turns.clear();
                s.turns.push(SessionTurn {
                    role: "system".to_string(),
                    content: format!("[compacted summary]\n{summary}"),
                    tokens,
                    at,
                });
                s.state = SessionState::Compacted;
                s.last_active = at;
            }
            SessionEvent::ExpensiveOpRecorded { at } => {
                s.last_expensive_op_at = Some(at);
                s.last_active = at;
            }
        }
    }

    /// Scan the sessions directory and load all JSONL files (non-authoritative; prefer [`Self::load`] from DB).
    pub async fn load_all(&mut self) -> Result<usize, SessionError> {
        if !self.config.sessions_dir.exists() {
            return Ok(0);
        }
        let entries = fs::read_dir(&self.config.sessions_dir)?;
        let mut loaded = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let _ = self.load(stem).await;
                loaded += 1;
            }
        }
        Ok(loaded)
    }

    /// Session file path for a given ID.
    pub(super) fn session_path(&self, session_id: &str) -> PathBuf {
        self.config.sessions_dir.join(format!("{session_id}.jsonl"))
    }

    /// Append a JSONL event to the session's file.
    pub(super) fn append_event(
        &self,
        session_id: &str,
        event: &SessionEvent,
    ) -> Result<(), SessionError> {
        let path = self.session_path(session_id);
        let mut json = serde_json::to_string(event).map_err(SessionError::Serialize)?;
        json.push('\n');
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }
}
