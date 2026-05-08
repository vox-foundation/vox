use crate::pid::Pid;
use tokio::sync::{mpsc, oneshot};

/// A message envelope wrapping user messages, requests, and system signals.
///
/// `Envelope` is NOT Clone because `Request` contains a oneshot sender.
/// Use `Message` or `Signal` variants where cloning is needed.
#[derive(Debug)]
pub enum Envelope {
    /// Fire-and-forget message payload.
    Message(Message),
    /// Request-response: caller sends a message and waits for a reply.
    Request(Request),
    /// System signal (link, monitor, exit).
    Signal(Signal),
}

/// An application-level message sent between actors.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sender process id.
    pub from: Pid,
    /// Application payload.
    pub payload: MessagePayload,
}

/// A request expecting a response, carrying a oneshot reply channel.
#[derive(Debug)]
pub struct Request {
    /// Caller process id.
    pub from: Pid,
    /// Request body.
    pub payload: MessagePayload,
    /// Channel the callee uses to send the reply string.
    pub reply_tx: oneshot::Sender<String>,
}

/// Dynamic message payload (typed via serde-like serialization in practice).
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// UTF-8 text payload.
    Text(String),
    /// JSON-serialized string payload.
    Json(String),
    /// Opaque binary payload.
    Binary(Vec<u8>),
}

/// System signals for process lifecycle management.
#[derive(Debug, Clone)]
pub enum Signal {
    /// Process exited normally.
    Exit(Pid, ExitReason),
    /// Link request between two processes.
    Link(Pid),
    /// Unlink request.
    Unlink(Pid),
    /// Monitor notification.
    Down(Pid, ExitReason),
}

/// Reason a process exited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitReason {
    /// Clean exit without error.
    Normal,
    /// Cooperative shutdown requested.
    Shutdown,
    /// Actor failed with a message.
    Error(String),
}

/// A handle to send messages to a process's mailbox.
pub type MailboxSender = mpsc::Sender<Envelope>;
/// The receiving end of a process's mailbox.
pub type MailboxReceiver = mpsc::Receiver<Envelope>;

/// Create a new mailbox with the given buffer capacity.
pub fn new_mailbox(capacity: usize) -> (MailboxSender, MailboxReceiver) {
    mpsc::channel(capacity)
}

/// Trait used to sever GC boundaries when passing messages between actors.
/// The compiler derives this for custom types to deep-copy values outside
/// the GC arena before they cross a mailbox boundary.
pub trait DeepCloneToOwned {
    type Owned;
    fn deep_clone_to_owned(&self) -> Self::Owned;
}
