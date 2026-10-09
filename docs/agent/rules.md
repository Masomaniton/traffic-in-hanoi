# Traffic in Hanoi: Rules Specification

**Status:** normative. This document defines the game’s valid positions and
valid turn traces. History storage, Undo, networking, and UI feedback are
implementation concerns defined elsewhere.

## 1. Design parameters

A game design supplies:

- an integer `n ≥ 1`, the number of ranks per player;
- a finite normalized set of squares, each with a unique non-negative
  coordinate `(column, row)`;
- one start and one finish square for each player.

A design is normalized when its minimum column and minimum row are both zero.
Two squares are adjacent exactly when their Manhattan distance is one. No
arbitrary graph edges exist in version one of the game.

The players are `Heart` and `Spade`; `other(p)` denotes the other player.
Each player owns one card of every internal rank in `{0, …, n - 1}`. A rank is
displayed to players as its internal value plus one, so rank `0` is Ace.

## 2. Standard design

The standard design has `n = 5`. Its main board is:

```text
{0, 1, 2} × {1, 2, 3}
```

and its special squares are:

```text
Start(Heart)  = (0,0)     Finish(Spade) = (2,0)
Start(Spade)  = (0,4)     Finish(Heart) = (2,4)
```

These coordinates, together with orthogonal adjacency, completely specify the
standard board.

## 3. Piles and supports

Each square contains one ordered pile, listed bottom-to-top. A pile is
rank-legal when a card directly above another has rank less than or equal to
the lower card's rank.

A **support** is either a square base or a card:

```text
Support = Base(square) | Card(card)
```

For a pile `[c1, …, ck]` on square `q`, `Base(q)` supports `c1`, and `Card(ci)`
supports `c(i+1)` when it exists. The **occupant** of a support is the card
directly on it, if one exists.

Cards target supports, never merely squares. A card placed on `Card(c)` rests
directly on `c`; a card placed on `Base(q)` becomes the bottom card of `q`.

## 4. Initial position and permanent restrictions

Initially, each start square contains its owner’s cards bottom-to-top in
strictly descending rank order:

```text
n - 1, n - 2, …, 1, 0
```

Heart takes the first turn. A card may never enter the other owner’s start or
finish square. Cards may otherwise enter or leave their own start and finish
squares until victory.

At every turn boundary, each pile has cards from at most one owner. Mixed piles
are permitted during a turn.

## 5. Turn trace

Each card is `Idle`, `Evicted`, or `Intended(target support)`. A turn is a
finite trace beginning when the active player selects one owned card as the
**active card** and root-evicts it.

The trace then consists of player intentions and fulfilments interleaved with
automatic evictions. The active player may choose their order, subject to the
validity conditions below. A turn ends only with a valid end action.

### 5.1 Automatic evictions

An automatic eviction occurs when:

- a card directly covers an evicted card (**cover eviction**);
- a card occupies the target support of a new intention (**target eviction**);
- a fulfilled card lands directly on an opponent-owned card (**mix eviction**).

Already evicted or intended cards do not receive another eviction.

## 6. Validity conditions

Every state in a valid turn trace satisfies the following conditions.

### 6.1 Ownership and source condition

- Every evicted card owned by the active player is the active card.
- An intention may be made only with an evicted card that has no occupant; that
  card is therefore topmost in its pile.
- An active-player-owned card may be intended only when it is the active card.

### 6.2 Target condition

For an intended card `c` targeting support `s`:

- `square(c)` and `square(s)` are adjacent;
- `square(s)` is not an opponent-owned special square;
- if `s = Card(b)`, then `rank(c) ≤ rank(b)`;
- if `occupant(s) = o`, then `rank(o) < rank(c)`;
- no other intended card targets `s` (**target collision**);
- if `s = Card(b)`, then `b` is idle (**support collision**).

Thus a support collision includes both forms of non-idle target support: an
evicted card and an intended card.

### 6.3 Fulfilment condition

An intended card may fulfil exactly when its target support has no occupant. It
moves from the top of its pile directly onto that support and becomes idle.

The target condition prevents any valid later intention from covering an
intended mover. Therefore an intended card remains topmost until fulfilment;
topmost status need not be repeated as a fulfilment condition.

### 6.4 End condition

A turn may end only when every card is idle. Pile rank, special-square access,
and single-owner turn-boundary conditions are invariants of valid traces, not
additional player-facing end conditions.

## 7. Victory

After a valid turn ends, pass the turn to the other player `p`. Before `p`
starts a turn, `p` wins if every card owned by `p` occupies `Finish(p)`.

If both finish piles are complete, the player whose turn begins wins.

