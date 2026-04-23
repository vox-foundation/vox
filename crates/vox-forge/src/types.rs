//! Forge-neutral types used across all `GitForgeProvider` implementations.
//!
//! All types here are Forge-agnostic. The key naming convention:
//! - GitHub calls these "Pull Requests"
//! - GitLab calls these "Merge Requests"
//! - Vox calls them all "ChangeRequests" (internal term)

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ChangeRequest (PR/MR abstraction)
// ---------------------------------------------------------------------------

/// Forge-neutral identifier for a ChangeRequest (distinct from the human-facing `number`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeRequestId(
    /// Raw numeric id returned by the forge API.
    pub u64,
);

/// State of a ChangeRequest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeRequestState {
    /// Open for review and merge.
    Open,
    /// Closed without merge.
    Closed,
    /// Merged into the target branch.
    Merged,
    /// Work-in-progress / not yet ready for merge (when the forge models drafts separately).
    Draft,
}

/// CI/merge status of a ChangeRequest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeRequestStatus {
    /// Checks still running or not reported.
    Pending,
    /// Required checks passed.
    Success,
    /// One or more checks failed.
    Failure,
    /// Check system reported an error.
    Error,
    /// Status could not be mapped from the forge payload.
    Unknown,
}

/// A forge-neutral change request (e.g. GitHub pull request or GitLab merge request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    /// Forge-internal numeric ID.
    pub id: ChangeRequestId,
    /// Short human-readable number (e.g., #42).
    pub number: u64,
    /// Title of the change request.
    pub title: String,
    /// Description / body (markdown).
    pub body: String,
    /// Source branch (the branch being merged).
    pub source_branch: String,
    /// Target branch (the branch being merged into).
    pub target_branch: String,
    /// Current state.
    pub state: ChangeRequestState,
    /// CI/merge status.
    pub status: ChangeRequestStatus,
    /// Author login.
    pub author: String,
    /// Assignees.
    pub assignees: Vec<String>,
    /// Labels attached to this CR.
    pub labels: Vec<Label>,
    /// URL on the forge.
    pub web_url: String,
    /// When created (ISO 8601).
    pub created_at: String,
    /// When last updated (ISO 8601).
    pub updated_at: String,
    /// Whether this CR is a draft.
    pub is_draft: bool,
    /// Whether it is currently mergeable.
    pub mergeable: Option<bool>,
}

impl ChangeRequest {
    /// True if this CR is in an open, non-draft state.
    pub fn is_actionable(&self) -> bool {
        self.state == ChangeRequestState::Open && !self.is_draft
    }
}

/// Arguments for `GitForgeProvider::create_change_request`.
#[derive(Debug, Clone, Copy)]
pub struct NewChangeRequest<'a> {
    /// Change request title.
    pub title: &'a str,
    /// Markdown body / description.
    pub body: &'a str,
    /// Head branch (contains the commits to merge).
    pub source_branch: &'a str,
    /// Base branch to merge into.
    pub target_branch: &'a str,
    /// Open as a draft when supported by the forge.
    pub draft: bool,
}

// ---------------------------------------------------------------------------
// Release
// ---------------------------------------------------------------------------

/// Arguments for `GitForgeProvider::create_release`.
#[derive(Debug, Clone, Copy)]
pub struct NewRelease<'a> {
    /// Tag name for the release.
    pub tag_name: &'a str,
    /// Release name/title.
    pub name: &'a str,
    /// Markdown body.
    pub body: &'a str,
    /// Whether to create as a draft.
    pub draft: bool,
}

// ---------------------------------------------------------------------------
// Discussion/Issue
// ---------------------------------------------------------------------------

/// Arguments for `GitForgeProvider::create_discussion_or_issue`.
#[derive(Debug, Clone, Copy)]
pub struct NewDiscussionOrIssue<'a> {
    /// Title of the discussion/issue.
    pub title: &'a str,
    /// Markdown body.
    pub body: &'a str,
    /// Category (used by GitHub Discussions; ignored by Issues/GitLab).
    pub category: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Label
// ---------------------------------------------------------------------------

/// A label on a ChangeRequest or issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// Short label text as shown in the forge UI.
    pub name: String,
    /// Hex color string from the forge API (often without leading `#`).
    pub color: String,
    /// Optional longer description for the label.
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Review
// ---------------------------------------------------------------------------

/// State of a code review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewState {
    /// Reviewer approved the change.
    Approved,
    /// Reviewer requested changes before merge.
    ChangesRequested,
    /// Comment-only review.
    Commented,
    /// Review was dismissed.
    Dismissed,
    /// Review is pending / not yet submitted.
    Pending,
}

/// A code review on a ChangeRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Reviewer login on the forge.
    pub reviewer: String,
    /// Outcome of the review.
    pub state: ReviewState,
    /// Optional review comment body.
    pub body: Option<String>,
    /// ISO 8601 timestamp when submitted, if known.
    pub submitted_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Repository info
// ---------------------------------------------------------------------------

/// Forge-neutral repository metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoInfo {
    /// Owner (user or org).
    pub owner: String,
    /// Repository name.
    pub name: String,
    /// Full path (e.g., "owner/repo").
    pub full_name: String,
    /// Clone URL (HTTPS).
    pub clone_url: String,
    /// SSH URL.
    pub ssh_url: Option<String>,
    /// Default branch name.
    pub default_branch: String,
    /// Whether the repo is private.
    pub is_private: bool,
    /// Star count.
    pub stars: u64,
    /// Fork count.
    pub forks: u64,
    /// Open issues count.
    pub open_issues: u64,
    /// Description.
    pub description: Option<String>,
    /// Web URL.
    pub web_url: String,
}

// ---------------------------------------------------------------------------
// User
// ---------------------------------------------------------------------------

/// A forge user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeUser {
    /// Primary handle / login.
    pub login: String,
    /// Display name, if different from login.
    pub display_name: Option<String>,
    /// Public email when exposed by the forge.
    pub email: Option<String>,
    /// Avatar image URL.
    pub avatar_url: Option<String>,
    /// Profile URL on the forge website.
    pub web_url: String,
    /// Whether this account is an app/bot user.
    pub is_bot: bool,
}

// ---------------------------------------------------------------------------
// Webhook events
// ---------------------------------------------------------------------------

/// A webhook event received from a forge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebhookEvent {
    /// A push to a branch.
    Push {
        /// Branch that received commits.
        branch: String,
        /// Commit SHAs included in the push (best-effort; forge-dependent).
        commits: Vec<String>,
        /// Actor that pushed.
        pusher: String,
    },
    /// A ChangeRequest was opened.
    ChangeRequestOpened {
        /// Human-facing CR number (e.g. PR number).
        cr_number: u64,
        /// Login of the author.
        author: String,
    },
    /// A ChangeRequest was merged.
    ChangeRequestMerged {
        /// Human-facing CR number.
        cr_number: u64,
        /// Login of the user who merged.
        merged_by: String,
    },
    /// A ChangeRequest was closed (without merge).
    ChangeRequestClosed {
        /// Human-facing CR number.
        cr_number: u64,
    },
    /// A review was submitted.
    ReviewSubmitted {
        /// Target CR number.
        cr_number: u64,
        /// Reviewer login.
        reviewer: String,
        /// Review outcome.
        state: ReviewState,
    },
    /// A CI check completed.
    CheckCompleted {
        /// Related CR number when the payload associates a check with a CR.
        cr_number: Option<u64>,
        /// Check or workflow name.
        name: String,
        /// Normalized check status.
        status: ChangeRequestStatus,
    },
    /// An unknown event type.
    Unknown {
        /// Raw event type string from the forge.
        event_type: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_request_actionable() {
        let cr = ChangeRequest {
            id: ChangeRequestId(1),
            number: 42,
            title: "Fix parser bug".into(),
            body: String::new(),
            source_branch: "fix/parser".into(),
            target_branch: "main".into(),
            state: ChangeRequestState::Open,
            status: ChangeRequestStatus::Pending,
            author: "alice".into(),
            assignees: vec![],
            labels: vec![],
            web_url: "https://github.com/org/repo/pull/42".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            is_draft: false,
            mergeable: Some(true),
        };
        assert!(cr.is_actionable());
    }

    #[test]
    fn draft_cr_not_actionable() {
        let mut cr = ChangeRequest {
            id: ChangeRequestId(2),
            number: 43,
            title: "WIP: big refactor".into(),
            body: String::new(),
            source_branch: "wip/refactor".into(),
            target_branch: "main".into(),
            state: ChangeRequestState::Open,
            status: ChangeRequestStatus::Pending,
            author: "bob".into(),
            assignees: vec![],
            labels: vec![],
            web_url: "https://example.com/pr/43".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            is_draft: true,
            mergeable: None,
        };
        assert!(!cr.is_actionable());
        cr.is_draft = false;
        assert!(cr.is_actionable());
    }
}
