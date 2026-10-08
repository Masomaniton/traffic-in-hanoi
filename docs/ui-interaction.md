# Traffic in Hanoi: UI Interaction

## 1. Mouse controls

The live board uses direct clicks rather than command text fields.

- Click an idle card to attempt the turn's root eviction.
- Click an evicted card to select it for an intention.
- With an evicted card selected, click an empty square to target its base, or
  click a card to target that card as support. The click submits an intention.
- Click an intended card to fulfil its already-declared intention.

Stacks fan horizontally so the suit/rank corner and clickable area of every
card remain visible. A selected evicted card is visibly highlighted.

## 2. Dead ends

Players may attempt actions that violate the declarative rules. The current
batch then becomes blocked and the interface freezes it with a red-tinted
explanation visible to both players. Show every violated predicate at the first
invalid history position. Only Undo is available while blocked.

## 3. Keyboard controls

Scope keyboard controls to the focused game board and prevent browser-default
navigation where necessary.

| Key | Action |
| --- | --- |
| Backspace | Undo the final batch. |
| Enter | Attempt `End`. |
| Left / Right | Move one entry backward / forward in the read-only replay cursor. |
| Up / Down | Move one turn backward / forward in the read-only replay cursor. |

The replay cursor never changes the live partial turn. The player may make a
new operation only while viewing the current live history position.
