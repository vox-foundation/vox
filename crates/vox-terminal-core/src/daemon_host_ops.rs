//! Extended `term.*` JSON-line RPC constants and [`DaemonHost`] client operations
//! for custom block streaming, `/clear`, `/compact`, MENS feedback, and `control_epoch`.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, OutputChunk};
use crate::daemon_host::DaemonHost;
use crate::host::BlockFeedbackVerdict;

pub const CUSTOM_OPEN: &str = "term.custom_open";
pub const CUSTOM_CHUNK: &str = "term.custom_chunk";
pub const CUSTOM_FINISH: &str = "term.custom_finish";
pub const GET_BLOCK: &str = "term.get_block";
pub const CLEAR: &str = "term.clear";
pub const COMPACT: &str = "term.compact";
pub const FEEDBACK: &str = "term.feedback";
pub const CONTROL_EPOCH: &str = "term.control_epoch";

impl DaemonHost {
    pub(crate) async fn rpc_begin_custom_block(
        &self,
        id: &str,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
    ) -> Result<BlockId> {
        let r = self
            .client
            .call(
                CUSTOM_OPEN,
                json!({
                    "session_id": id,
                    "kind": kind,
                    "block_origin": origin,
                    "input": input,
                }),
            )
            .await?;
        r.get("block_id")
            .and_then(Value::as_u64)
            .map(BlockId)
            .ok_or_else(|| anyhow!("term.custom_open: missing block_id"))
    }

    pub(crate) async fn rpc_append_block_chunk(
        &self,
        id: &str,
        block_id: BlockId,
        chunk: OutputChunk,
    ) -> Result<()> {
        self.client
            .call(
                CUSTOM_CHUNK,
                json!({
                    "session_id": id,
                    "block_id": block_id.0,
                    "chunk": chunk,
                }),
            )
            .await
            .map(drop)
    }

    pub(crate) async fn rpc_finish_custom_block(
        &self,
        id: &str,
        block_id: BlockId,
        exit_code: i32,
        cancelled: bool,
    ) -> Result<Option<Block>> {
        let r = self
            .client
            .call(
                CUSTOM_FINISH,
                json!({
                    "session_id": id,
                    "block_id": block_id.0,
                    "exit_code": exit_code,
                    "cancelled": cancelled,
                }),
            )
            .await?;
        match r.get("block") {
            Some(v) if !v.is_null() => Ok(Some(serde_json::from_value(v.clone())?)),
            _ => Ok(None),
        }
    }

    pub(crate) async fn rpc_get_block(&self, id: &str, block_id: BlockId) -> Result<Option<Block>> {
        let r = self
            .client
            .call(
                GET_BLOCK,
                json!({
                    "session_id": id,
                    "block_id": block_id.0,
                }),
            )
            .await?;
        match r.get("block") {
            Some(v) if !v.is_null() => Ok(Some(serde_json::from_value(v.clone())?)),
            _ => Ok(None),
        }
    }

    pub(crate) async fn rpc_clear_unpinned_blocks(&self, id: &str) -> Result<usize> {
        let r = self.client.call(CLEAR, json!({"session_id": id})).await?;
        let rem = r
            .get("remaining_blocks")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        Ok(rem as usize)
    }

    pub(crate) async fn rpc_compact_blocks(
        &self,
        id: &str,
        summary_markdown: String,
        keep_recent: usize,
    ) -> Result<Option<BlockId>> {
        let r = self
            .client
            .call(
                COMPACT,
                json!({
                    "session_id": id,
                    "summary_markdown": summary_markdown,
                    "keep_recent": keep_recent,
                }),
            )
            .await?;
        Ok(r.get("digest_block_id")
            .and_then(Value::as_u64)
            .map(BlockId))
    }

    pub(crate) async fn rpc_record_block_feedback(
        &self,
        id: &str,
        block_id: BlockId,
        verdict: BlockFeedbackVerdict,
    ) -> Result<bool> {
        let r = self
            .client
            .call(
                FEEDBACK,
                json!({
                    "session_id": id,
                    "block_id": block_id.0,
                    "verdict": verdict,
                }),
            )
            .await?;
        Ok(r.get("recorded").and_then(Value::as_bool).unwrap_or(false))
    }

    pub(crate) async fn rpc_control_epoch(&self, id: &str) -> Result<u64> {
        let r = self
            .client
            .call(CONTROL_EPOCH, json!({"session_id": id}))
            .await?;
        Ok(r.get("control_epoch").and_then(Value::as_u64).unwrap_or(0))
    }
}
