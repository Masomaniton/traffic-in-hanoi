# Traffic in Hanoi: Rules Specification

**Status:** normative. This document defines the configurable board, the
state needed to apply the two board actions, the rules for those actions, and
their invariants and consequences. Turn advancement, Undo, networking, UI
feedback, and victory presentation are implementation concerns defined
elsewhere.

## 1. Design parameters

A game design supplies:

- an ordered list of two or more players, each with a unique arbitrary name;
- a finite, non-empty, normalized set `S` of squares, each with a unique
  non-negative coordinate `(column, row)`;
- one start and one finish square for each player; and
- one initial, rank-legal pile for each player.

Players take turns in the configured order. This specification calls the
player whose turn is in progress the **active player**; the mechanism that
starts, settles, and advances turns is outside this specification.

A design is normalized when the minimum column and minimum row in `S` are both
zero. `S` may have holes and need not be rectangular; it is otherwise a board
on the square lattice.

The directions and their coordinate offsets are:

```text
Up    = ( 0,  1)      Down  = ( 0, -1)
Left  = (-1,  0)      Right = ( 1,  0)
```

For a square `q` and direction `d`, `target(q, d)` is `q` plus the offset for
`d`. A direction from `q` is available exactly when `target(q, d)` belongs to
`S`. Thus every available direction leads to an existing orthogonally adjacent
square, and no arbitrary graph edges exist in version one of the game.

Each card has an owner and a natural-number rank. Cards do not have persistent
identities: their position in a pile is sufficient. Cards of equal owner and
rank are therefore interchangeable for these rules.

Let `special(p) = { Start(p), Finish(p) }`. A player's start and finish may
coincide. Special squares belonging to distinct players must not overlap:

```text
special(p) intersection special(r) = empty, for distinct players p and r
```

## 2. Standard design

The standard design has two players, `Heart` and `Spade`, in that turn order.
Each begins with ranks one through five, bottom-to-top in descending order:

```text
5, 4, 3, 2, 1
```

Its main board is:

```text
{0, 1, 2} x {1, 2, 3}
```

and its special squares are:

```text
Start(Heart)  = (0,0)     Finish(Spade) = (2,0)
Start(Spade)  = (0,4)     Finish(Heart) = (2,4)
```

These squares, together with the available directions defined above, completely
specify the standard board.

## 3. State and definitions

Every square has one pile: an ordered sequence of zero or more cards, listed
bottom-to-top. A card's **height** is its zero-based position in its pile: the
bottom card has height zero.

Each square is in exactly one stored square state:

- **idling**; or
- **moving(direction)**, where `direction` is an available direction from that
  square.

Every moving square is non-empty. Its top card is its **moving card**, and its
target is the square indicated by its stored direction.

A turn is either:

- **initial**, with no active-card location; or
- **subsequent**, with an active-card location `(square, height)`.

The active-card location identifies the card currently at that position. It is
established by the first successful intention of a turn. When that card is
fulfilled, its location is updated to its new square and height. It does not
otherwise change during the turn. Thus no global card identifier is needed.

Each square also has a derived status, **planted** or **evicted**. Status is
not stored; it is recalculated from the current piles and moving-square states.
A square is evicted if and only if either of these definitions holds:

- **rank eviction:** it is the non-empty target of at least one moving square,
  and its top card has lesser rank than the moving card of at least one such
  moving square;
- **owner eviction:** it is non-empty, and the owner of its top card is not the
  owner of every card below it in the same pile.

Otherwise the square is planted. In particular, an empty square is planted.

## 4. Rules

The only board actions are **intend** and **fulfil**. An attempted action that
does not meet its conditions does not change board or turn state.

### 4.1 Intend

To intend a source square `q`, the active player chooses an available direction
`d` from `q`. Let `t = target(q, d)` and let `c` be the top card of `q`.

The intention is valid only when all of these conditions hold:

- `q` is idling and non-empty;
- `t` is idling;
- `t` is not a special square of any player other than the owner of `c`;
- the turn is initial and `c` belongs to the active player, or the turn is
  subsequent and the subsequent-intention condition holds.

The subsequent-intention condition holds when `q` is evicted and either:

- `c` is the card at the active-card location; or
- `c` does not belong to the active player.

A successful initial intention records the active-card location as `(q, h)`,
where `h` is the height of `c`, and transitions the turn to subsequent. Every
successful intention transitions `q` from `idling` to `moving(d)`; it does not
move a card.

### 4.2 Fulfil

A square `q` may be fulfilled only when it is `moving(d)`. Let `t = target(q,
d)` and let `c` be the moving card of `q`.

The fulfilment is valid only when `t` is empty or the rank of its top card is
not less than the rank of `c`.

To fulfil, move `c` from the top of `q` onto the top of `t`, then transition
`q` from `moving(d)` to `idling`. The target square keeps whatever state it
already has, including a moving state. If `c` was the active card, update its
location to its new height at `t`.

## 5. Invariants

The following facts hold in every initial state and after every successful
action:

- Every pile is **rank-legal**: every card directly above another has rank less
  than or equal to the rank of the card below it.
- A card never occupies the start or finish square of a player other than its
  owner.
- Every moving square is non-empty, and its moving card is its top card.
- A subsequent turn has exactly one active-card location, and that location
  identifies a card owned by the active player.

Initially, every square is idling. Each player's initial pile occupies that
player's start square, and every other square is empty. A design's supplied
initial piles must satisfy the rank-legality invariant.

## 6. Consequences

These are deductions from the definitions and rules, not extra restrictions.

- A square may be both moving and evicted; state and status are independent.
- An empty square is planted.
- An intention's target need only be idling when the intention is made. That
  target may later become moving, so one moving square may target another.
- An evicted top card owned by the active player can be intended only when it
  is the active card.
- A fulfilment is the earliest point at which the destination's top-card rank
  can be checked. By contrast, the special-square restriction is enforced at
  intention time, before a moving state is created.
