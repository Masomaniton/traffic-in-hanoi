At least for now, we should have each `square` be represented as a pair of natural numbers for the column and row. In this case, every `adjacency` would consist of a `square` and a `direction`. The minimum row and column should be 0.

The implementation should not make any assumptions about the shape of the board because I eventually want this to be customizable.

Note, the status of each square as planted or evicted does not have to be stored since it can always be calculated on the fly.

At least for now, when a player does something illegal, we should not change the state. Instead, we should briefly show a message explaining why that move was illegal.

The player should have to manually end the turn. The controls I imagine are backspace for undo and enter to end a turn. We do not need something for reset for now, but our design should not prohibit that possibility. It is ok if time complexity of the eventual reset grows linearly with the numbers of actions in the turn so far.

The active card for a turn can be held in in `Option<Rank>` because we already know what the owner has to be. We will know whether or not we are at the start of the turn by whether we see a `None` or `Some` variant in this type.
