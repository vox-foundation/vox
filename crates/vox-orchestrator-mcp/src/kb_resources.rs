//! `kb://doc/{path}` and `kb://chunk/{hash}` MCP resources over the curated Library.
//!
//! Reads go through the daemon's `vox_kb_get` (one code path for tools and resources, AP-15):
//! this process opens no DB and reads no doc file. A URI is validated by the same
//! `validate_doc_path` / `validate_chunk_hash` the tool uses, before any daemon call.

use vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::OrchestratorDaemonEnsure;

use crate::kb_tools::library::{validate_chunk_hash, validate_doc_path};

const DOC_PREFIX: &str = "kb://doc/";
const CHUNK_PREFIX: &str = "kb://chunk/";
const MIME: &str = "text/markdown";

/// A validated `kb://` resource address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KbUri {
    /// Repo-relative corpus path, e.g. `docs/src/guide.md`.
    Doc(String),
    /// 64 lowercase hex chars.
    Chunk(String),
}

/// Parse and validate a `kb://` URI; anything else is an error.
pub fn parse_kb_uri(uri: &str) -> Result<KbUri, String> {
    if let Some(path) = uri.strip_prefix(DOC_PREFIX) {
        validate_doc_path(path).map(|p| KbUri::Doc(p.to_string()))
    } else if let Some(hash) = uri.strip_prefix(CHUNK_PREFIX) {
        validate_chunk_hash(hash).map(|h| KbUri::Chunk(h.to_string()))
    } else {
        Err(format!("not a kb:// resource uri: {uri}"))
    }
}

/// Body text of a `vox_kb_get` tool envelope for `uri`, or the envelope's error message.
fn body_from_envelope(uri: &KbUri, envelope: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(envelope).map_err(|e| format!("bad tool envelope: {e}"))?;
    if v.get("success").and_then(|s| s.as_bool()) != Some(true) {
        let msg = v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("vox_kb_get failed");
        return Err(msg.to_string());
    }
    let body = match uri {
        KbUri::Doc(_) => v.pointer("/data/markdown"),
        KbUri::Chunk(_) => v.pointer("/data/chunk/body"),
    };
    body.and_then(|b| b.as_str())
        .map(str::to_string)
        .ok_or_else(|| "vox_kb_get result had no body".to_string())
}

/// Read a `kb://` resource through the daemon; returns `(mime, text)`.
pub async fn read_kb_resource(
    daemon: &OrchestratorDaemonEnsure,
    uri: &str,
) -> Result<(String, String), String> {
    let parsed = parse_kb_uri(uri)?;
    let args = match &parsed {
        KbUri::Doc(p) => serde_json::json!({ "path": p }),
        KbUri::Chunk(h) => serde_json::json!({ "chunk_hash": h }),
    };
    let envelope = crate::daemon_route::call_tool_via_daemon(daemon, "vox_kb_get", args)
        .await
        .map_err(|e| format!("{e}"))?;
    body_from_envelope(&parsed, &envelope).map(|text| (MIME.to_string(), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn parse_kb_uri_accepts_doc_and_chunk() {
        assert_eq!(
            parse_kb_uri("kb://doc/docs/src/library/index.md"),
            Ok(KbUri::Doc("docs/src/library/index.md".into()))
        );
        assert_eq!(
            parse_kb_uri(&format!("kb://chunk/{HASH}")),
            Ok(KbUri::Chunk(HASH.into()))
        );
    }

    #[test]
    fn body_from_envelope_unwraps_doc_chunk_and_errors() {
        let doc = KbUri::Doc("docs/src/a.md".into());
        let chunk = KbUri::Chunk(HASH.into());
        let ok_doc = r##"{"success":true,"data":{"markdown":"# A"}}"##;
        let ok_chunk = r#"{"success":true,"data":{"chunk":{"body":"body text"}}}"#;
        let err = r#"{"success":false,"error":"not indexed: docs/src/a.md"}"#;
        assert_eq!(body_from_envelope(&doc, ok_doc), Ok("# A".into()));
        assert_eq!(body_from_envelope(&chunk, ok_chunk), Ok("body text".into()));
        assert!(
            body_from_envelope(&doc, err)
                .unwrap_err()
                .contains("not indexed")
        );
        assert!(body_from_envelope(&doc, ok_chunk).is_err());
        assert!(body_from_envelope(&doc, "not json").is_err());
    }
}
