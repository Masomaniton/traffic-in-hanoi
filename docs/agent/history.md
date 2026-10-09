# Local history

`HistoryCursor` is client-only presentation state. It does not replace the
append-only `GameEvent` log described in [network.md](network.md).

## Directional stacks

The cursor has a past stack and a future deque. Its position is implicit: the
top of `past` is the current state and the front of `future` is the next state.

```text
left:  pop past, unapply it, push its forward form to the front of future
right: pop front of future, apply it, push its inverse form to past
```

Normal left/right operations are O(1). Gameplay is allowed only when `future`
is empty. When an authoritative event arrives during review, `apply_event`
first validates the complete canonical log transactionally, reconstructs its
live tail, and restores the prior batch boundary; an invalid event leaves the
cursor unchanged.

## Direction-specific entries

Past entries contain only what reverse application needs:

```text
Root(card)
Eviction(card)
Intention(card)
Fulfilment { source, card }
End
```

For `Intention(card)`, the current `CardStatus::Intended(target)` supplies the
target while unapplying. For `Fulfilment { source, card }`, the card's current
support is the target; reversal restores it to `source` with
`Intended(target)` status.

Future entries contain only what forward application needs:

```text
Root(card)
Eviction(card)
Intention { card, target }
Fulfilment(card)
End
```

Automatic evictions appear in both directional forms. A batch is one player
operation followed by zero or more evictions. Left and right arrow controls
move a whole batch by applying or unapplying its entries one at a time;
trailing evictions are visited before the player operation when moving left.

`Undo` is not a history entry. It is an accepted canonical `GameDelta` which
removes the live final batch. The complete `GameEvent` log still records it.
