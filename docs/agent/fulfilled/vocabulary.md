# Traffic in Hanoi: Vocabulary

This document defines informal vocabulary for conversation and UI copy. It
does not add rules beyond [rules.md](rules.md).

| Term | Meaning |
| --- | --- |
| **pile** | A bottom-to-top sequence of cards on one square. |
| **moving square** | A square in `Moving(direction)` state; its top card is the moving card. |
| **active card** | The positional card established by the first successful intention of a turn. |
| **planted** | A derived square status with neither rank nor owner eviction. |
| **evicted** | A derived status caused by rank eviction, owner eviction, or both. |
| **settled** | Every board square is idling and planted. |
| **step backward / forward** | Local client review navigation; neither is a server Undo. |
| **undo** | The authoritative, open-turn `ServerEvent::Undo`. |
| **take-back** | A future consensual mechanism to revisit a committed earlier turn; not Undo. |
