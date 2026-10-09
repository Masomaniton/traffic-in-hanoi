use std::collections::{BTreeMap, BTreeSet};

use traffic_core::{CoreError, Layout, Owner, Square};

pub struct Draft {
    pub rank_count: u16,
    pub squares: BTreeSet<Square>,
    pub starts: BTreeMap<Owner, Square>,
    pub finishes: BTreeMap<Owner, Square>,
}

impl Draft {
    pub fn standard(rank_count: u16) -> Self {
        let layout = Layout::standard(rank_count).expect("standard layout is valid");
        Self {
            rank_count,
            squares: layout.squares().collect(),
            starts: BTreeMap::from([
                (Owner::Heart, layout.start(Owner::Heart)),
                (Owner::Spade, layout.start(Owner::Spade)),
            ]),
            finishes: BTreeMap::from([
                (Owner::Heart, layout.finish(Owner::Heart)),
                (Owner::Spade, layout.finish(Owner::Spade)),
            ]),
        }
    }

    pub fn empty(rank_count: u16) -> Self {
        Self {
            rank_count,
            squares: BTreeSet::new(),
            starts: BTreeMap::new(),
            finishes: BTreeMap::new(),
        }
    }

    pub fn build(&self) -> Result<Layout, CoreError> {
        Layout::new(
            self.rank_count,
            self.squares.clone(),
            self.starts.clone(),
            self.finishes.clone(),
        )
    }

    pub fn print(&self) {
        println!("layout draft: {} ranks", self.rank_count);
        println!(
            "squares: {}",
            self.squares
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
        for owner in [Owner::Heart, Owner::Spade] {
            println!(
                "{owner}: start {:?}, finish {:?}",
                self.starts.get(&owner),
                self.finishes.get(&owner)
            );
        }
    }
}
