//! Sequence diagram IR types. Phase 86 adds the parser and the `Ir::Sequence` variant
//! additively; nothing here parses.

use crate::span::Location;
use serde::{Deserialize, Serialize};

/// A sequence diagram: declared actors and the ordered events between them.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SequenceIr {
    /// Declared actors in source order.
    pub actors: Vec<Actor>,
    /// Messages and notes in source order.
    pub events: Vec<SeqEvent>,
}

/// A participant lane.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// Stable identifier referenced by events.
    pub id: String,
    /// Display text.
    pub label: String,
    /// Source location of the declaration.
    pub span: Location,
}

/// One event on the timeline.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SeqEvent {
    /// An arrow between two actors; `dashed` marks a return.
    Message {
        /// Sending actor id.
        from: String,
        /// Receiving actor id.
        to: String,
        /// Message text.
        text: String,
        /// Dashed (return) arrow.
        dashed: bool,
        /// Source location.
        span: Location,
    },
    /// A `note over` one or two actors.
    Note {
        /// One or two actor ids.
        over: Vec<String>,
        /// Note text.
        text: String,
        /// Source location.
        span: Location,
    },
}
