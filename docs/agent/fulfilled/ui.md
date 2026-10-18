# Traffic in Hanoi: UI

This document specifies the graphical board and its interaction model. It
refines the human-facing [UI note](../human/ui.md) for implementation. The
current local demo implements the lattice, compact card stacks, pile fan,
pointer actions, and preliminary local review described here. It remains an
in-memory demo and lacks automatic reconnect/catch-up; see
[local-demo.md](local-demo.md) for its current behavior.

The browser submits source-oriented board actions only. It never names a card
to move, calculates eviction, or treats its local snapshot as authoritative.

## Board geometry and visual language

The board has three nested visual layers: board, square, and card. Each has a
subtle border. The board is green; idling squares are a distinct, slightly
darker green; moving squares are darker still. Colour is supplementary: state
and controls must also have text, shape, or accessible descriptions.

Squares are square. Cards are rounded rectangles with a non-square aspect
ratio. The renderer owns responsive pixel measurements, but must preserve these
relationships for a base radius `r` and a nesting increment `g`:

```text
card radius   = r
square radius = r + g
board radius  = r + 2g
```

Two corresponding corners are *aligned* when their quarter-circle centers
coincide, not when their outside edges merely line up. Thus a card aligned to
a square corner is inset from that edge by `g`; a square aligned to a board
corner is inset by `g`. This produces concentric nesting at the outer board
corners and at the corners of each card-containing square. Grid gaps, border
weights, and `g` should be derived from the same responsive scale rather than
chosen independently.

The standard design is a three-by-three central grid with the four special
squares in their configured positions. Render configured squares by their
`Square` coordinates; do not encode the standard arrangement into interaction
logic. The board must allow pile fans to paint outside their originating
square and, where necessary, outside the board; it must not clip them.

## Compact piles and pile fans

Piles are rendered bottom-to-top. Let `SANDWICH_CARDS` be a non-negative,
client presentation setting. A compact pile displays at most
`SANDWICH_CARDS + 2` cards: the top card is at the right aligned corners of
the square; the oldest displayed card is at its left aligned corners; and up
to `SANDWICH_CARDS` cards occupy evenly spaced slots between them. Cards below
that leftmost displayed card are omitted completely. This is visual clipping
only: it must not alter the pile, its count, or the accessible pile
description.

A fanned pile displays every card in the same bottom-to-top order. Its
bottommost card uses the square's left aligned corners. Successive cards move
rightward by a constant fan offset and may extend past the square or board.
The fan is above ordinary board content and remains legible rather than being
clipped or compressed to fit the lattice. It is an inspector, not a second
board state and not a source of moves.

Within every card, display the rank and first letter of the owner at the top
right, and repeat the marking upside down at the bottom left to preserve
180-degree rotational symmetry. Card identity must also be available as an
accessible name (for example, “Heart 3”). Iconography and player colours are
deliberately deferred.

## Pointer interaction

Main board actions use pointer input:

- A normal click on an idling source square selects it for an intention. A
  normal click on an orthogonally adjacent target then submits
  `Intend(Adjacency)`.
- A normal click on a moving square submits `Fulfil(source)`.
- Pressing and holding on a pile for a short deliberate hold opens its fully
  fanned inspector. Releasing after it opens does not close it.
- The next pointer click anywhere dismisses an open fan and is consumed; it
  does not also select a square or submit an action.

The selected source and open fan are transient client presentation state. A
successful server event is the only change to `current`; a rejection leaves
the authoritative snapshot untouched and is visibly explained. The client may
preview the selected adjacency and target, but server validation remains
authoritative. A normal click must not accidentally open a fan, and opening a
fan must not submit a board action.

Before release, provide a keyboard-accessible equivalent for selection,
fulfilment, fan inspection, and dismissal. It must not depend on pointer hover
or take away the documented history shortcuts. Focus indicators must remain
visible despite the minimalist styling.

## Live commands and local review

- `Backspace` attempts authoritative `Undo` for the current unended turn.
- `Enter` attempts authoritative `EndTurn`.
- Left/right arrows step one action backward/forward through local review.
- Up/down arrows step one turn backward/forward through local review.

`Undo` and local review are distinct. `Undo` is an authoritative server event;
the arrow controls modify `viewed`, never `current`, and send no event. The
client may submit gameplay only while `viewed` is current. A key handler must
not consume these shortcuts while focus is in editable text, and should not
prevent normal assistive-technology interaction. The review model and inferred
turn boundaries are specified in [history.md](history.md).

Render the current player, active-card location, settled status, moving
direction, derived eviction status, and server rejection in a form that does
not rely only on colour. Empty squares are planted. End turn is an attempt;
the server determines whether the authoritative snapshot is settled. Once it
accepts `EndTurn`, that turn is committed.
