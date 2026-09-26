//! WebhookInbox extension point — poll-style drain of events accepted by a webhook
//! listener plugin (D-10).
//!
//! Each item is one JSON-serialized webhook event with the fields `id`, `source`,
//! `event_type`, `payload` and `received_at`. The host polls on an interval; there is no
//! push callback across the ABI boundary.

use abi_stable::{sabi_trait, std_types::*};

pub const WEBHOOK_INBOX_REVISION: u32 = 1;

#[sabi_trait]
pub trait WebhookInbox: Send + Sync {
    fn revision(&self) -> u32 {
        WEBHOOK_INBOX_REVISION
    }
    /// Returns at most `max` pending events in arrival order, an empty vector when none
    /// are pending, and `RErr` when the listener has not been started.
    fn poll_events(&self, max: u32) -> RResult<RVec<RString>, RBoxError>;
}
