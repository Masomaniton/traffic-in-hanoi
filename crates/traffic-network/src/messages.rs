use serde::{Deserialize, Serialize};
use traffic_core::{GameCreated, GameDelta, GameEvent, GameSequence};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ClientMessage {
    SubmitDelta {
        known_sequence: GameSequence,
        delta: GameDelta,
    },
    CatchUpAfter {
        sequence: GameSequence,
    },
    RequestReplayBootstrap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ServerMessage {
    ReplayBootstrap {
        created: GameCreated,
        deltas: Vec<GameDelta>,
    },
    Events {
        events: Vec<GameEvent>,
    },
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
