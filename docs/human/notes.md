Should we allow start and finish on the same square?
Disallow selecting active card once background is red.
Investigate use of "occupants", which creates a Vec.
Make sure we do not use a Vec of Batch.
ReplyBootstrap probably does not need sequence numbers.
Consider moving over to UDP since we already have sequence numbers.
Allow deltas to be multi-turn so that we can better group network stuff.
How do we display an invalid fulfilment where two cards have the same support?
Redesign "Game" struct to avoid `Vec<TurnRecord>`, and audit other types too.
Redundant: "An active-player-owned card may be intended only when it is the active card".

Prompt:
