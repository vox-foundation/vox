//! Host-Moderated Live Guest Share (`GuestShareService` & `GuestShareDecider`, `T3-STREAM-GUEST-03..04`).
//!
//! Adapted from `pingdotgg/t3code` `#24104`:
//! - `apps/server/src/guest/GuestShareService.ts` (`GUEST_SHARE_TTL_MS = 24h`, single-browser redemption)
//! - `apps/server/src/orchestration-v2/Decider.guest.ts` (`GUEST_PENDING_MESSAGE_LIMIT = 3`, host approve/dismiss moderation)
//!
//! Uses `vox_crypto::keyed_hash` (BLAKE3 MAC) and `vox_secrets::resolve_secret(SecretId::WebhookSigningSecret)`
//! in strict compliance with `AGENTS.md §Cryptography Policy` & `§Secret Management`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default time-to-live for a guest share invite token (`24` hours in milliseconds).
pub const GUEST_SHARE_TTL_MS: u64 = 24 * 60 * 60 * 1_000;

/// Maximum number of unmoderated pending messages a single guest may queue at once.
pub const GUEST_PENDING_MESSAGE_LIMIT: usize = 3;

/// Errors returned by `GuestShareService` and `GuestShareDecider`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GuestShareError {
    /// Invite token does not exist or belongs to another thread.
    #[error("guest share invite not found")]
    InviteNotFound,
    /// Invite token signature verification failed.
    #[error("guest share invite signature is invalid")]
    InvalidSignature,
    /// Invite token has expired (`now_ms > expires_at_ms`).
    #[error("guest share invite has expired")]
    InviteExpired,
    /// Invite token was already redeemed by another browser session.
    #[error("guest share invite has already been redeemed by another browser session")]
    AlreadyRedeemed,
    /// Guest session is not authorized for this thread.
    #[error("guest session is not authorized")]
    UnauthorizedSession,
    /// Guest already has `GUEST_PENDING_MESSAGE_LIMIT` (`3`) pending messages awaiting host approval.
    #[error("guest pending message limit ({GUEST_PENDING_MESSAGE_LIMIT}) reached")]
    PendingLimitReached,
    /// Pending guest message ID was not found.
    #[error("pending guest message not found")]
    PendingMessageNotFound,
}

fn default_guest_scope() -> String {
    "suggest_prompt".to_string()
}

/// Signed guest invite token record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestInviteRecord {
    /// Invite ID (`invite_id`).
    pub invite_id: String,
    /// Target thread ID.
    pub thread_id: String,
    /// Permission scope (`"suggest_prompt"` or `"read_only"`).
    #[serde(default = "default_guest_scope")]
    pub scope: String,
    /// Expiration timestamp in Unix milliseconds.
    pub expires_at_ms: u64,
    /// Hex-encoded BLAKE3 `keyed_hash` MAC over `invite_id:thread_id:expires_at_ms`.
    pub signature_hex: String,
    /// Browser session ID that redeemed this invite (`None` until first redemption).
    pub redeemed_by_session_id: Option<String>,
    /// Whether the invite has been revoked by the host.
    #[serde(default)]
    pub revoked: bool,
}

/// Moderation state of a guest-submitted prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuestMessageModerationStatus {
    /// Awaiting host owner approval.
    Pending,
    /// Approved by host owner and promoted into an active turn.
    Approved,
    /// Dismissed by host owner.
    Dismissed,
}

/// A guest-submitted prompt awaiting or having completed host moderation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestPendingMessage {
    /// Unique message ID.
    pub message_id: String,
    /// Thread ID.
    pub thread_id: String,
    /// Guest browser session ID.
    pub guest_session_id: String,
    /// Display name of the guest.
    pub guest_display_name: String,
    /// Submitted prompt text.
    pub prompt: String,
    /// Current moderation status.
    pub status: GuestMessageModerationStatus,
}

/// Host-moderated Live Guest Share service (`GuestShareService` + `GuestShareDecider`).
#[derive(Debug, Clone)]
pub struct GuestShareService {
    signing_key: [u8; 32],
    invites: HashMap<String, GuestInviteRecord>,
    pending_messages: Vec<GuestPendingMessage>,
}

impl Default for GuestShareService {
    fn default() -> Self {
        Self::new_from_clavis()
    }
}

impl GuestShareService {
    /// Construct a `GuestShareService` resolving its signing key via `vox_secrets::resolve_secret`
    /// and deriving a 32-byte key with `vox_crypto::secure_hash`.
    pub fn new_from_clavis() -> Self {
        let resolved = vox_secrets::resolve_secret(vox_secrets::SecretId::WebhookSigningSecret);
        let seed = resolved
            .expose()
            .unwrap_or("vox-default-ephemeral-guest-share-key");
        let signing_key = vox_crypto::secure_hash(seed.as_bytes());
        Self {
            signing_key,
            invites: HashMap::new(),
            pending_messages: Vec::new(),
        }
    }

    /// Construct with an explicit 32-byte key for deterministic unit tests.
    pub fn with_signing_key(signing_key: [u8; 32]) -> Self {
        Self {
            signing_key,
            invites: HashMap::new(),
            pending_messages: Vec::new(),
        }
    }

    fn compute_mac_hex(&self, invite_id: &str, thread_id: &str, expires_at_ms: u64) -> String {
        let payload = format!("{invite_id}:{thread_id}:{expires_at_ms}");
        let mac = vox_crypto::keyed_hash(&self.signing_key, payload.as_bytes());
        hex::encode(&mac[..16])
    }

    /// Mint a 128-bit BLAKE3-MAC guest invite token valid for `GUEST_SHARE_TTL_MS` (`24h`).
    pub fn create_invite(
        &mut self,
        invite_id: impl Into<String>,
        thread_id: impl Into<String>,
        now_ms: u64,
    ) -> GuestInviteRecord {
        self.create_invite_with_options(
            invite_id,
            thread_id,
            default_guest_scope(),
            now_ms,
            GUEST_SHARE_TTL_MS,
        )
    }

    /// Mint a 128-bit BLAKE3-MAC guest invite token with explicit permission scope and TTL.
    pub fn create_invite_with_options(
        &mut self,
        invite_id: impl Into<String>,
        thread_id: impl Into<String>,
        scope: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> GuestInviteRecord {
        let invite_id = invite_id.into();
        let thread_id = thread_id.into();
        let scope = scope.into();
        let expires_at_ms = now_ms.saturating_add(ttl_ms);
        let signature_hex = self.compute_mac_hex(&invite_id, &thread_id, expires_at_ms);
        let record = GuestInviteRecord {
            invite_id: invite_id.clone(),
            thread_id,
            scope,
            expires_at_ms,
            signature_hex,
            redeemed_by_session_id: None,
            revoked: false,
        };
        self.invites.insert(invite_id, record.clone());
        record
    }

    /// Redeem an invite token for `browser_session_id`.
    /// Single-browser lock: the first browser session binds the invite; any different browser session
    /// attempting to redeem the same invite receives `GuestShareError::AlreadyRedeemed`.
    pub fn redeem_invite(
        &mut self,
        invite_id: &str,
        signature_hex: &str,
        browser_session_id: &str,
        now_ms: u64,
    ) -> Result<GuestInviteRecord, GuestShareError> {
        let Some(existing) = self.invites.get(invite_id).cloned() else {
            return Err(GuestShareError::InviteNotFound);
        };
        let expected_sig = self.compute_mac_hex(
            &existing.invite_id,
            &existing.thread_id,
            existing.expires_at_ms,
        );
        if expected_sig != signature_hex {
            return Err(GuestShareError::InvalidSignature);
        }
        if now_ms > existing.expires_at_ms {
            return Err(GuestShareError::InviteExpired);
        }
        match &existing.redeemed_by_session_id {
            Some(bound_session) if bound_session != browser_session_id => {
                return Err(GuestShareError::AlreadyRedeemed);
            }
            _ => {}
        }
        let record = self
            .invites
            .get_mut(invite_id)
            .expect("invite verified above");
        record.redeemed_by_session_id = Some(browser_session_id.to_string());
        Ok(record.clone())
    }

    /// Enqueue a guest message for host moderation (`GUEST_PENDING_MESSAGE_LIMIT = 3`).
    pub fn submit_guest_message(
        &mut self,
        message_id: impl Into<String>,
        thread_id: &str,
        guest_session_id: &str,
        guest_display_name: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<GuestPendingMessage, GuestShareError> {
        let authorized = self.invites.values().any(|inv| {
            inv.thread_id == thread_id
                && inv.redeemed_by_session_id.as_deref() == Some(guest_session_id)
        });
        if !authorized {
            return Err(GuestShareError::UnauthorizedSession);
        }
        let active_pending = self
            .pending_messages
            .iter()
            .filter(|m| {
                m.thread_id == thread_id
                    && m.guest_session_id == guest_session_id
                    && m.status == GuestMessageModerationStatus::Pending
            })
            .count();
        if active_pending >= GUEST_PENDING_MESSAGE_LIMIT {
            return Err(GuestShareError::PendingLimitReached);
        }
        let msg = GuestPendingMessage {
            message_id: message_id.into(),
            thread_id: thread_id.to_string(),
            guest_session_id: guest_session_id.to_string(),
            guest_display_name: guest_display_name.into(),
            prompt: prompt.into(),
            status: GuestMessageModerationStatus::Pending,
        };
        self.pending_messages.push(msg.clone());
        Ok(msg)
    }

    /// Host owner approves a pending guest message so it can be dispatched as a turn.
    pub fn has_pending_message(&self, message_id: &str) -> bool {
        self.pending_messages
            .iter()
            .any(|m| m.message_id == message_id)
    }

    pub fn approve_guest_message(
        &mut self,
        message_id: &str,
    ) -> Result<GuestPendingMessage, GuestShareError> {
        let Some(msg) = self
            .pending_messages
            .iter_mut()
            .find(|m| m.message_id == message_id)
        else {
            return Err(GuestShareError::PendingMessageNotFound);
        };
        msg.status = GuestMessageModerationStatus::Approved;
        Ok(msg.clone())
    }

    /// Host owner dismisses a pending guest message.
    pub fn dismiss_guest_message(
        &mut self,
        message_id: &str,
    ) -> Result<GuestPendingMessage, GuestShareError> {
        let Some(msg) = self
            .pending_messages
            .iter_mut()
            .find(|m| m.message_id == message_id)
        else {
            return Err(GuestShareError::PendingMessageNotFound);
        };
        msg.status = GuestMessageModerationStatus::Dismissed;
        Ok(msg.clone())
    }

    /// List invites, optionally filtered by `thread_id`, sorted deterministically by `invite_id`.
    #[must_use]
    pub fn list_invites(&self, thread_id: Option<&str>) -> Vec<GuestInviteRecord> {
        let mut out: Vec<GuestInviteRecord> = self
            .invites
            .values()
            .filter(|inv| thread_id.is_none_or(|tid| inv.thread_id == tid))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.invite_id.cmp(&b.invite_id));
        out
    }

    /// List guest messages, optionally filtered by `thread_id`, sorted deterministically by `message_id`.
    #[must_use]
    pub fn list_pending_messages(&self, thread_id: Option<&str>) -> Vec<GuestPendingMessage> {
        let mut out: Vec<GuestPendingMessage> = self
            .pending_messages
            .iter()
            .filter(|msg| thread_id.is_none_or(|tid| msg.thread_id == tid))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.message_id.cmp(&b.message_id));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GUEST_PENDING_MESSAGE_LIMIT, GUEST_SHARE_TTL_MS, GuestMessageModerationStatus,
        GuestShareError, GuestShareService,
    };

    #[test]
    fn guest_share_enforces_single_browser_redemption_and_pending_limit() {
        let mut svc = GuestShareService::with_signing_key([7u8; 32]);
        let now = 1_700_000_000_000;
        let inv = svc.create_invite("inv-1", "thread-1", now);
        assert_eq!(inv.expires_at_ms, now + GUEST_SHARE_TTL_MS);

        // First browser redeems successfully (and can re-redeem idempotently)
        svc.redeem_invite("inv-1", &inv.signature_hex, "browser-A", now + 100)
            .expect("first browser redeem");
        svc.redeem_invite("inv-1", &inv.signature_hex, "browser-A", now + 200)
            .expect("same browser re-redeem");

        // Second browser is rejected with AlreadyRedeemed
        let second = svc.redeem_invite("inv-1", &inv.signature_hex, "browser-B", now + 300);
        assert_eq!(second, Err(GuestShareError::AlreadyRedeemed));

        // Enqueue up to GUEST_PENDING_MESSAGE_LIMIT = 3 messages
        for i in 1..=GUEST_PENDING_MESSAGE_LIMIT {
            svc.submit_guest_message(
                format!("msg-{i}"),
                "thread-1",
                "browser-A",
                "Alice",
                format!("Guest question {i}"),
            )
            .expect("within limit");
        }
        let fourth =
            svc.submit_guest_message("msg-4", "thread-1", "browser-A", "Alice", "Over limit");
        assert_eq!(fourth, Err(GuestShareError::PendingLimitReached));

        // Host approves msg-1, freeing a pending slot
        let approved = svc.approve_guest_message("msg-1").expect("approve msg-1");
        assert_eq!(approved.status, GuestMessageModerationStatus::Approved);

        svc.submit_guest_message(
            "msg-4",
            "thread-1",
            "browser-A",
            "Alice",
            "Now allowed after approval",
        )
        .expect("slot freed after approval");
    }

    #[test]
    fn list_invites_and_pending_messages_filter_and_sort_deterministically() {
        let mut svc = GuestShareService::with_signing_key([9u8; 32]);
        let now = 1_700_000_000_000;
        let inv_b = svc.create_invite("inv-b", "thread-2", now);
        let inv_a = svc.create_invite("inv-a", "thread-1", now);

        svc.redeem_invite("inv-b", &inv_b.signature_hex, "browser-2", now + 10)
            .expect("redeem inv-b");
        svc.redeem_invite("inv-a", &inv_a.signature_hex, "browser-1", now + 20)
            .expect("redeem inv-a");

        svc.submit_guest_message("msg-z", "thread-2", "browser-2", "Bob", "Prompt Z")
            .expect("submit msg-z");
        svc.submit_guest_message("msg-a", "thread-1", "browser-1", "Alice", "Prompt A")
            .expect("submit msg-a");

        let all_invites = svc.list_invites(None);
        assert_eq!(all_invites.len(), 2);
        assert_eq!(all_invites[0].invite_id, "inv-a");
        assert_eq!(all_invites[1].invite_id, "inv-b");

        let t1_invites = svc.list_invites(Some("thread-1"));
        assert_eq!(t1_invites.len(), 1);
        assert_eq!(t1_invites[0].invite_id, "inv-a");

        let all_msgs = svc.list_pending_messages(None);
        assert_eq!(all_msgs.len(), 2);
        assert_eq!(all_msgs[0].message_id, "msg-a");
        assert_eq!(all_msgs[1].message_id, "msg-z");

        let t2_msgs = svc.list_pending_messages(Some("thread-2"));
        assert_eq!(t2_msgs.len(), 1);
        assert_eq!(t2_msgs[0].message_id, "msg-z");
    }
}
