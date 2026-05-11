//! Sealed-trait facade for capability minting.
//!
//! [`MintToken`] is the only intended implementation of
//! `vox_orchestrator_types::vcs_capability::sealed::MintWitness`.
//! The real compile-time guard is `pub(crate)` on `WorkingTreeWrite::mint` and
//! `BranchCreate::mint` — external crates cannot call those constructors directly
//! and must go through [`mint_working_tree_write`] / [`mint_branch_create`] here.

use vox_orchestrator_types::vcs_capability::{
    BranchCreate, BranchName, WorkingTreeWrite, WorkspaceId,
};

/// The single in-process token that proves construction went through this facade.
#[derive(Debug, Copy, Clone)]
pub struct MintToken(());

impl vox_orchestrator_types::vcs_capability::sealed::MintWitness for MintToken {}

/// Mint a [`WorkingTreeWrite`] capability for `workspace`/`branch`.
///
/// Authorization (lock-leader check, affinity, signature) is the caller's
/// responsibility — `vox_orchestrator::authorize_*` wrappers are the only
/// intended callers.
pub fn mint_working_tree_write(workspace: WorkspaceId, branch: BranchName) -> WorkingTreeWrite {
    let token = MintToken(());
    vox_orchestrator_types::vcs_capability::sealed::__mint_working_tree_write(
        workspace, branch, &token,
    )
}

/// Mint a [`BranchCreate`] capability for `workspace`/`parent`.
pub fn mint_branch_create(workspace: WorkspaceId, parent: BranchName) -> BranchCreate {
    let token = MintToken(());
    vox_orchestrator_types::vcs_capability::sealed::__mint_branch_create(workspace, parent, &token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_working_tree_write_round_trip() {
        let cap = mint_working_tree_write(WorkspaceId(1), BranchName::parse("agent/test").unwrap());
        assert_eq!(cap.workspace(), WorkspaceId(1));
        assert_eq!(cap.branch().as_str(), "agent/test");
    }

    #[test]
    fn mint_branch_create_round_trip() {
        let cap = mint_branch_create(WorkspaceId(2), BranchName::parse("main").unwrap());
        assert_eq!(cap.workspace(), WorkspaceId(2));
        assert_eq!(cap.parent().as_str(), "main");
    }
}
