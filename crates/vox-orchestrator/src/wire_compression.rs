//! Wire Snapshot & Event Stream Compression (`WireIdTableInterner`, `LiveBodyDeduper`, and `omit_idle_subagents`).
//!
//! Adapted from:
//! - `packages/contracts/src/orchestrationV2IdTable.ts` (`compactOrchestrationIdTable` / `expandOrchestrationIdTable`)
//! - `packages/shared/src/orchestrationV2DedupedBodies.ts` (`LIVE_DEDUPE_MEMORY = 32`)
//! - `packages/client-runtime/src/state/threadFamilies.ts` (`omitIdleSubagents`)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, VecDeque};

/// Number of recent large text bodies retained in the sliding deduplication window.
pub const LIVE_DEDUPE_MEMORY: usize = 32;

/// Minimum UTF-8 byte length before a body field (`text`, `body`, `output`, `markdown_summary`) is deduplicated.
pub const MIN_DEDUPE_BODY_BYTES: usize = 64;

/// Compact wire envelope carrying an interned ID table (`ids`) and the compacted JSON payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactWireEnvelope {
    /// Interned ID strings indexed by `0..ids.len()`.
    pub ids: Vec<String>,
    ///Deduplicated large text bodies indexed by `0..bodies.len()`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<String>,
    /// Compacted payload where `*Id` / `*_id` fields hold integer indices into `ids`.
    pub payload: Value,
}

/// Returns `true` if `key` is an entity ID field eligible for response-level ID-table interning.
pub fn is_id_field_key(key: &str) -> bool {
    key == "id"
        || key.ends_with("Id")
        || key.ends_with("_id")
        || key.ends_with("Ids")
        || key.ends_with("_ids")
}

/// Returns `true` if `key` is an array-of-IDs field (`*Ids` or `*_ids`).
pub fn is_id_array_field_key(key: &str) -> bool {
    key.ends_with("Ids") || key.ends_with("_ids")
}

/// Returns `true` if `key` is a large text body field eligible for sliding-window deduplication.
pub fn is_body_field_key(key: &str) -> bool {
    matches!(
        key,
        "text" | "body" | "output" | "content" | "markdown_summary"
    )
}

/// Response-level ID table interner (`orchestrationV2IdTable` equivalent).
#[derive(Debug, Default, Clone)]
pub struct WireIdTableInterner {
    ids: Vec<String>,
    index_by_id: HashMap<String, usize>,
}

impl WireIdTableInterner {
    /// Create an empty ID table interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a single ID string and return its integer table index.
    pub fn intern(&mut self, id: &str) -> usize {
        if let Some(&idx) = self.index_by_id.get(id) {
            return idx;
        }
        let idx = self.ids.len();
        self.ids.push(id.to_string());
        self.index_by_id.insert(id.to_string(), idx);
        idx
    }

    /// Recursively compact all ID fields inside `value`.
    pub fn compact_value(&mut self, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut out = Map::with_capacity(map.len());
                for (k, v) in map {
                    if is_id_array_field_key(k) {
                        if let Value::Array(items) = v {
                            let compacted_items: Vec<Value> = items
                                .iter()
                                .map(|item| match item.as_str() {
                                    Some(s) if !s.is_empty() => Value::from(self.intern(s)),
                                    _ => self.compact_value(item),
                                })
                                .collect();
                            out.insert(k.clone(), Value::Array(compacted_items));
                            continue;
                        }
                    } else if is_id_field_key(k) {
                        if let Some(s) = v.as_str() {
                            if !s.is_empty() {
                                out.insert(k.clone(), Value::from(self.intern(s)));
                                continue;
                            }
                        }
                    }
                    out.insert(k.clone(), self.compact_value(v));
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| self.compact_value(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Consume the interner and return the ordered `ids` table.
    pub fn into_ids(self) -> Vec<String> {
        self.ids
    }

    /// Expand a compacted JSON value back to full string IDs using `ids`.
    pub fn expand_value(value: &Value, ids: &[String]) -> Value {
        match value {
            Value::Object(map) => {
                let mut out = Map::with_capacity(map.len());
                for (k, v) in map {
                    if is_id_array_field_key(k) {
                        if let Value::Array(items) = v {
                            let expanded: Vec<Value> = items
                                .iter()
                                .map(|item| {
                                    if let Some(idx) = item.as_u64() {
                                        if let Some(id_str) = ids.get(idx as usize) {
                                            return Value::String(id_str.clone());
                                        }
                                    }
                                    Self::expand_value(item, ids)
                                })
                                .collect();
                            out.insert(k.clone(), Value::Array(expanded));
                            continue;
                        }
                    } else if is_id_field_key(k) {
                        if let Some(idx) = v.as_u64() {
                            if let Some(id_str) = ids.get(idx as usize) {
                                out.insert(k.clone(), Value::String(id_str.clone()));
                                continue;
                            }
                        }
                    }
                    out.insert(k.clone(), Self::expand_value(v, ids));
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| Self::expand_value(v, ids)).collect())
            }
            other => other.clone(),
        }
    }
}

/// Sliding-window (`LIVE_DEDUPE_MEMORY = 32`) large text body deduplicator.
#[derive(Debug, Default, Clone)]
pub struct LiveBodyDeduper {
    window: VecDeque<String>,
    bodies: Vec<String>,
    index_by_body: HashMap<String, usize>,
}

impl LiveBodyDeduper {
    /// Create a new body deduplicator bounded by `LIVE_DEDUPE_MEMORY = 32`.
    pub fn new() -> Self {
        Self::default()
    }

    fn record_or_lookup(&mut self, text: &str) -> Option<usize> {
        if text.len() < MIN_DEDUPE_BODY_BYTES {
            return None;
        }
        if let Some(&idx) = self.index_by_body.get(text) {
            return Some(idx);
        }
        if self.window.len() >= LIVE_DEDUPE_MEMORY {
            if let Some(evicted) = self.window.pop_front() {
                self.index_by_body.remove(&evicted);
            }
        }
        let idx = self.bodies.len();
        self.bodies.push(text.to_string());
        self.window.push_back(text.to_string());
        self.index_by_body.insert(text.to_string(), idx);
        None
    }

    /// Replace duplicate large text bodies in `value` with `{"$dedup": idx}` references.
    pub fn dedupe_value(&mut self, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut out = Map::with_capacity(map.len());
                for (k, v) in map {
                    if is_body_field_key(k) {
                        if let Some(s) = v.as_str() {
                            if let Some(idx) = self.record_or_lookup(s) {
                                let mut marker = Map::new();
                                marker.insert("$dedup".to_string(), Value::from(idx));
                                out.insert(k.clone(), Value::Object(marker));
                                continue;
                            }
                        }
                    }
                    out.insert(k.clone(), self.dedupe_value(v));
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| self.dedupe_value(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Consume the deduper and return the recorded `bodies` table.
    pub fn into_bodies(self) -> Vec<String> {
        self.bodies
    }

    /// Expand `{"$dedup": idx}` markers using `bodies`.
    pub fn expand_bodies(value: &Value, bodies: &[String]) -> Value {
        match value {
            Value::Object(map) => {
                if map.len() == 1 {
                    if let Some(idx) = map.get("$dedup").and_then(Value::as_u64) {
                        if let Some(body) = bodies.get(idx as usize) {
                            return Value::String(body.clone());
                        }
                    }
                }
                let mut out = Map::with_capacity(map.len());
                for (k, v) in map {
                    out.insert(k.clone(), Self::expand_bodies(v, bodies));
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|v| Self::expand_bodies(v, bodies))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

/// Compress a JSON snapshot payload with both `WireIdTableInterner` and `LiveBodyDeduper`.
pub fn compress_wire_snapshot(payload: &Value) -> CompactWireEnvelope {
    let mut id_interner = WireIdTableInterner::new();
    let mut body_deduper = LiveBodyDeduper::new();
    let id_compacted = id_interner.compact_value(payload);
    let fully_compacted = body_deduper.dedupe_value(&id_compacted);
    CompactWireEnvelope {
        ids: id_interner.into_ids(),
        bodies: body_deduper.into_bodies(),
        payload: fully_compacted,
    }
}

/// Expand a `CompactWireEnvelope` back into the original uncompressed JSON value.
pub fn expand_wire_snapshot(envelope: &CompactWireEnvelope) -> Value {
    let bodies_expanded = LiveBodyDeduper::expand_bodies(&envelope.payload, &envelope.bodies);
    WireIdTableInterner::expand_value(&bodies_expanded, &envelope.ids)
}

/// Thread summary metadata used for `omit_idle_subagents` cold-connect filtering and `get_thread_family`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadFamilyNode {
    /// Unique thread ID.
    pub thread_id: String,
    /// Optional parent thread ID (`None` for root threads).
    pub parent_thread_id: Option<String>,
    /// Current lifecycle state (`"active"`, `"running"`, `"paused"`, `"waiting"`, `"error"`, `"completed"`, `"idle"`).
    pub status: String,
}

impl ThreadFamilyNode {
    /// Returns `true` if this node is a root thread (`parent_thread_id.is_none()`).
    pub fn is_root(&self) -> bool {
        self.parent_thread_id.is_none()
    }

    /// Returns `true` if this subagent thread is idle/completed and can be omitted on cold shell startup.
    pub fn is_idle_subagent(&self) -> bool {
        self.parent_thread_id.is_some()
            && matches!(
                self.status.as_str(),
                "completed" | "idle" | "done" | "archived"
            )
    }
}

/// Filter a list of threads for cold-connect shell startup (`omit_idle_subagents = true`).
pub fn filter_snapshot_subagent_threads(
    threads: &[ThreadFamilyNode],
    omit_idle_subagents: bool,
) -> Vec<ThreadFamilyNode> {
    if !omit_idle_subagents {
        return threads.to_vec();
    }
    threads
        .iter()
        .filter(|node| !node.is_idle_subagent())
        .cloned()
        .collect()
}

/// Return the full thread family (root thread + all descendant subagent threads) for `root_thread_id`.
pub fn get_thread_family(
    threads: &[ThreadFamilyNode],
    root_thread_id: &str,
) -> Vec<ThreadFamilyNode> {
    let mut family = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut frontier = VecDeque::new();
    frontier.push_back(root_thread_id.to_string());
    visited.insert(root_thread_id.to_string());

    while let Some(current_id) = frontier.pop_front() {
        for node in threads {
            if node.thread_id == current_id {
                family.push(node.clone());
            } else if node.parent_thread_id.as_deref() == Some(current_id.as_str())
                && visited.insert(node.thread_id.clone())
            {
                frontier.push_back(node.thread_id.clone());
            }
        }
    }
    family
}

#[cfg(test)]
mod tests {
    use super::{
        ThreadFamilyNode, compress_wire_snapshot, expand_wire_snapshot,
        filter_snapshot_subagent_threads, get_thread_family,
    };
    use serde_json::json;

    #[test]
    fn wire_id_table_and_body_deduper_roundtrip_losslessly() {
        let large_body = "A".repeat(128);
        let original = json!({
            "threadId": "thread-uuid-1111-2222",
            "turns": [
                {
                    "turnId": "turn-uuid-aaaa",
                    "threadId": "thread-uuid-1111-2222",
                    "itemIds": ["item-1", "item-2"],
                    "text": large_body,
                },
                {
                    "turnId": "turn-uuid-bbbb",
                    "threadId": "thread-uuid-1111-2222",
                    "itemIds": ["item-1", "item-3"],
                    "text": large_body,
                }
            ]
        });

        let compact = compress_wire_snapshot(&original);
        // `thread-uuid-1111-2222` and `item-1` appear only once in `compact.ids`.
        assert_eq!(
            compact
                .ids
                .iter()
                .filter(|s| s.as_str() == "thread-uuid-1111-2222")
                .count(),
            1
        );
        // Second occurrence of `large_body` is replaced with `{"$dedup": 0}`.
        assert_eq!(compact.payload["turns"][1]["text"]["$dedup"], json!(0));

        let restored = expand_wire_snapshot(&compact);
        assert_eq!(restored, original);
    }

    #[test]
    fn omit_idle_subagents_filters_completed_children_and_get_thread_family_restores_tree() {
        let nodes = vec![
            ThreadFamilyNode {
                thread_id: "root-1".to_string(),
                parent_thread_id: None,
                status: "completed".to_string(),
            },
            ThreadFamilyNode {
                thread_id: "sub-idle".to_string(),
                parent_thread_id: Some("root-1".to_string()),
                status: "completed".to_string(),
            },
            ThreadFamilyNode {
                thread_id: "sub-running".to_string(),
                parent_thread_id: Some("root-1".to_string()),
                status: "running".to_string(),
            },
            ThreadFamilyNode {
                thread_id: "sub-grandchild".to_string(),
                parent_thread_id: Some("sub-idle".to_string()),
                status: "idle".to_string(),
            },
        ];

        let cold = filter_snapshot_subagent_threads(&nodes, true);
        assert_eq!(cold.len(), 2);
        assert!(cold.iter().any(|n| n.thread_id == "root-1"));
        assert!(cold.iter().any(|n| n.thread_id == "sub-running"));

        let family = get_thread_family(&nodes, "root-1");
        assert_eq!(family.len(), 4);
    }
}
