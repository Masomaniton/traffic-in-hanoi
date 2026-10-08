//! Shared, transport-neutral messages for the Traffic in Hanoi live protocol.
//!
//! These types deliberately contain no sockets, authentication, or game rules.

use serde::{Deserialize, Serialize};
use traffic_core::{GameCreated, GameDelta, GameEvent, GameSequence};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ClientMessage {
    /// Propose one delta from the state ending at `known_sequence`.
    SubmitDelta {
        known_sequence: GameSequence,
        delta: GameDelta,
    },
    /// Continue an existing local replica with the contiguous event tail.
    CatchUpAfter { sequence: GameSequence },
    /// Build a new local replica from the creation record and complete log.
    RequestReplayBootstrap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ServerMessage {
    /// Used only when the client has no retained local game replica.
    ReplayBootstrap {
        created: GameCreated,
        events: Vec<GameEvent>,
    },
    /// A contiguous portion of the canonical append-only event log.
    Events { events: Vec<GameEvent> },
    /// A submitted delta was not appended to the canonical log.
    Rejected {
        current_sequence: GameSequence,
        reason: RejectionReason,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RejectionReason {
    StaleSequence,
    Unauthorized,
    InvalidDelta,
}
