//! Rule-violation values reported for blocked operations.

use std::fmt;

use crate::{Card, Square, Support};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Violation {
    RootOwner,
    RootAlreadySelected,
    ActiveEviction(Card),
    NotEvicted(Card),
    CoveredMover(Card),
    NonActiveOwner(Card),
    NonAdjacent,
    UnknownTargetSquare(Square),
    ForeignSpecialSquare,
    LargerOnSmaller,
    CannotPushEqualOrLarger,
    TargetCollision(Support),
    SupportCollision(Card),
    NotIntended(Card),
    OccupiedTarget(Support),
    PendingCards,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootOwner => f.write_str("root card must belong to the active player"),
            Self::RootAlreadySelected => f.write_str("a root card is already selected"),
            Self::ActiveEviction(card) | Self::NonActiveOwner(card) => {
                write!(f, "{card} is owned by the active player but is not active")
            }
            Self::NotEvicted(card) => write!(f, "{card} is not evicted"),
            Self::CoveredMover(card) => write!(f, "{card} has a card on top"),
            Self::NonAdjacent => f.write_str("source and target are not adjacent"),
            Self::UnknownTargetSquare(square) => {
                write!(f, "target square {square} does not belong to the layout")
            }
            Self::ForeignSpecialSquare => {
                f.write_str("card cannot enter the opponent's special square")
            }
            Self::LargerOnSmaller => f.write_str("card cannot rest on a lower-ranked card"),
            Self::CannotPushEqualOrLarger => {
                f.write_str("card cannot push an equal- or greater-ranked card")
            }
            Self::TargetCollision(support) => {
                write!(f, "another intention already targets {support}")
            }
            Self::SupportCollision(card) => write!(f, "target support {card} is not idle"),
            Self::NotIntended(card) => write!(f, "{card} is not intended"),
            Self::OccupiedTarget(support) => write!(f, "target support {support} is occupied"),
            Self::PendingCards => f.write_str("cannot end while cards are evicted or intended"),
        }
    }
}
