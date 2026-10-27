//! External AI session scanner & importer (`~/.claude`, `~/.codex`, `~/.cursor/chats/.../store.db`)
//! + portable `VoxSessionExportBundleV1` (`T3-WEBHOOK-IMPORT-03`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedExternalSession {
    pub source_harness: String,
    pub external_session_id: String,
    pub title: String,
    pub messages: Vec<ImportedSessionMessage>,
}

/// Portable JSON session export bundle (`T3-WEBHOOK-IMPORT-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxSessionExportBundleV1 {
    pub schema_version: u32,
    pub session_id: String,
    pub title: String,
    pub messages: Vec<ImportedSessionMessage>,
}

impl VoxSessionExportBundleV1 {
    pub fn new(
        session_id: impl Into<String>,
        title: impl Into<String>,
        messages: Vec<ImportedSessionMessage>,
    ) -> Self {
        Self {
            schema_version: 1,
            session_id: session_id.into(),
            title: title.into(),
            messages,
        }
    }
}

/// Decodes a hex-encoded root blob from Cursor's `store.db` SQLite `blobs` table into UTF-8 JSON (`T3-WEBHOOK-IMPORT-03`).
pub fn decode_cursor_root_blob_hex(hex_str: &str) -> Result<serde_json::Value, String> {
    let clean = hex_str.trim();
    if clean.len() % 2 != 0 {
        return Err("hex string must have even length".to_string());
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    let chars: Vec<u8> = clean.bytes().collect();
    for pair in chars.chunks_exact(2) {
        let s = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(s, 16).map_err(|e| format!("invalid hex byte '{s}': {e}"))?;
        bytes.push(b);
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid root blob JSON: {e}"))
}

/// Parses Claude Code or Codex CLI JSONL transcript lines into an `ImportedExternalSession` (`T3-WEBHOOK-IMPORT-03`).
pub fn parse_jsonl_external_session(
    source_harness: &str,
    external_session_id: &str,
    jsonl: &str,
) -> ImportedExternalSession {
    let mut messages = Vec::new();
    for line in jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let role = val
                .get("role")
                .or_else(|| val.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let content = val
                .get("content")
                .or_else(|| val.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !content.is_empty() {
                messages.push(ImportedSessionMessage { role, content });
            }
        }
    }
    let title = messages
        .first()
        .map(|m| m.content.chars().take(48).collect::<String>())
        .unwrap_or_else(|| format!("{source_harness}:{external_session_id}"));
    ImportedExternalSession {
        source_harness: source_harness.to_string(),
        external_session_id: external_session_id.to_string(),
        title,
        messages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_cursor_hex_blob_and_parses_claude_codex_jsonl() {
        let raw_json = br#"{"title":"Cursor Chat","messages":[{"role":"user","content":"hi"}]}"#;
        let hex_encoded = vox_crypto::hex_encode(raw_json);
        let decoded = decode_cursor_root_blob_hex(&hex_encoded).expect("decode hex");
        assert_eq!(decoded["title"], "Cursor Chat");

        let jsonl = "{\"role\":\"user\",\"content\":\"Implement router\"}\n{\"role\":\"assistant\",\"content\":\"Done\"}";
        let imported = parse_jsonl_external_session("claude", "sess-1", jsonl);
        assert_eq!(imported.messages.len(), 2);
        assert_eq!(imported.title, "Implement router");

        let bundle = VoxSessionExportBundleV1::new("v-1", imported.title, imported.messages);
        assert_eq!(bundle.schema_version, 1);
    }
}
