# The List:
- Should we allow start and finish on the same square?
- Disallow selecting active card once background is red.
- Make sure we do not use a Vec of Batch or Turn.
- ReplayBootstrap probably does not need sequence numbers.
- Consider moving over to UDP since we already have sequence numbers.
- Allow deltas to be multi-turn so that we can better group network stuff.
- Redesign "Game" struct to avoid `Vec<TurnRecord>`, make sure there is no `Vec<Batch>`, and audit other types too.
- Redundant: "An active-player-owned card may be intended only when it is the active card".
- When writing documents, I do not know how to make something plural but still signify that it is the same techical `term`.
- Get feedback on naming conventions.
- Allow proposing a take back.
- 

# Future directions:
- The most abstract board shape design consists of two fully customizable types: `square` and `adjacency`. Each `adjacency` has a source and target `square`. This theoretical allows for one way connections. Every `square` has a `pile`.
- Consider bringing back the "red background" idea and actually allowing the board to enter illegal state in order to better demonstrate why it is that the user cannot make that action.

Prompt:
