//! Shared transport-neutral protocol types.
use serde::{Deserialize, Serialize};
use traffic_core::{GameDesign, ServerEvent};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LogSequence(u64);
impl LogSequence {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SequencedEvent {
    pub sequence: LogSequence,
    pub event: ServerEvent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ClientMessage {
    SubmitEvent {
        known_through: Option<LogSequence>,
        event: ServerEvent,
    },
    CatchUpAfter {
        sequence: Option<LogSequence>,
    },
    RequestReplayBootstrap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ServerMessage {
    ReplayBootstrap {
        design: GameDesign,
        events: Vec<ServerEvent>,
    },
    Events {
        events: Vec<SequencedEvent>,
    },
    Rejected {
        current_through: Option<LogSequence>,
        reason: RejectionReason,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RejectionReason {
    StaleSequence,
    Unauthorized,
    InvalidEvent,
}
