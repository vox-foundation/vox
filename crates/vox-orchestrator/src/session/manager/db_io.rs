//! Blocking bridge for session DB I/O from sync code.

use super::super::errors::SessionError;

/// Run Codex session writes synchronously from non-async code (MCP / orchestrator hooks).
pub(super) fn run_session_db_io(
    fut: impl std::future::Future<Output = Result<(), vox_db::StoreError>> + Send,
) -> Result<(), SessionError> {
    use tokio::runtime::Handle;
    use tokio::task::block_in_place;
    match Handle::try_current() {
        Ok(handle) => block_in_place(|| handle.block_on(fut))
            .map_err(|e| SessionError::Io(std::io::Error::other(e.to_string()))),
        Err(_) => Err(SessionError::Io(std::io::Error::other(
            "run_session_db_io requires an active Tokio runtime",
        ))),
    }
}
