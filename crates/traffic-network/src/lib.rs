//! Shared, transport-neutral messages for the Traffic in Hanoi live protocol.
//!
//! These types deliberately contain no sockets, authentication, or game rules.

mod messages;

pub use messages::{ClientMessage, RejectionReason, ServerMessage};
