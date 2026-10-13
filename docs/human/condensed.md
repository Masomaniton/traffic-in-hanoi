TODO

Heart plays the first turn.

An idle card becomes evicted when there is
- an evicition on the support of the card
- an intention to the support of the card
- a fulfilment of the card onto a card of the opposite player


- During a turn, every evicted card owned by the player is the active card


A player **can intend** an evicted card to a target support when
- the card does not have any occupants
- the target support is in an orthogonally adjacent square which is not the a special square of the opponent
- any occupant of the target support is of a lesser rank than the card
- the target support, if it is a card, is of equal-or-greater rank than the card

A player **can fulfil** an intended card when
- the target support does not have any occupants

TODO: vow-like representation

```
pile <- empty, stack
stack -> square, card
card -> rank, player, status
adjacency -> square, square
```
