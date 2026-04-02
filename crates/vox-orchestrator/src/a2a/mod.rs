//! Agent-to-agent messaging types and helpers.

mod bus;
mod dispatch;
mod envelope;
mod remote_poller;
mod remote_worker;

pub use crate::types::{A2AMessage, A2AMessageType, MessageId};

pub use bus::MessageBus;
pub use dispatch::{
    acknowledge_db_message, drain_populi_remote_task_results, poll_inbox_from_db,
    prune_old_a2a_messages, relay_remote_task_cancel, relay_remote_task_envelope, relay_to_mesh,
    send_to_db, send_to_db_with_breaker,
};
pub use envelope::{
    A2ADeliveryPlane, A2AInboxPlane, A2ARoute, DbA2AMessage, REMOTE_TASK_ACK_TYPE,
    REMOTE_TASK_CANCEL_TYPE, REMOTE_TASK_ENVELOPE_TYPE, REMOTE_TASK_RESULT_TYPE, RemoteTaskAck,
    RemoteTaskCancel, RemoteTaskEnvelope, RemoteTaskResult,
};
pub use remote_poller::{populi_remote_result_poll_once, spawn_populi_remote_result_poller};
pub use remote_worker::{populi_remote_worker_tick_once, spawn_populi_remote_worker_poller};

pub use vox_search::{A2ARetrievalRefinement, A2ARetrievalRequest, A2ARetrievalResponse};
