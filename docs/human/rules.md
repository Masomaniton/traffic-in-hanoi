# Rules

## Background and Elements

Traffic in Hanoi is a two-player card game which thoughtfully combines Checkers and the Towers of Hanoi puzzle. This file contains the rules for the default match configuration.

The players are heart and spade. The main board is a three by three grid of squares. In addition, each player has special start and finish squares. The starts are below and above the left column for heart and spade, respectively. The finishes are above and below the right column for heart and spade, respectively. Each square has a pile, where a pile is defined recursively as empty or a pile with a card on top.

## State and Status

There is always an active player. This the player of the turn.

The possible states for a game are initial and subequent, where every subsequent game state has an active card.

The possible states for a square are idling and moving. A square in a moving state has the possible directions of up, down, left, and right one square. That is to say that a moving square indicates an orthogonally adjacent square as its target. This means that each special square connects only to a single corner.

The possible statuses for a square are planted and evicted. A square is evicted if and only if the rank or owner eviction conditions hold. The rank eviction condition applies to a non-empty target square of a moving square and the rank of the top card of the target square is less than the rank of the top card of the moving square. The owner eviction condition applies to a square when its pile is not empty and the owner of the top card is not the owner of every card it is stacked on.

*Note, states must be kept track of and only change due to player actions, while statuses are just needed to define the rules and update automatically.*

## Match and Turn

The match begins with two piles for the two players with cards of rank one through five, top to bottom, in the corresponding starts. Heart plays the first turn. The match ends when, at the start of a player's turn, each of that player's cards are in the player's finish.

To begin a turn, a player must intend a square. This transitions the game from initial to subsequent, where the top card of the square is the active card. To end a turn, all squares must be idling and planted. This transitions the game from subsequent to initial and switches the active player.

## Intend and Fulfil

*During a turn, the possible actions the active player can take are to intend and fulfil a square.*

A player can intend a square if and only if the initial or subsequent intention conditions hold. The initial intention condition applies if there have been no previous intentions in the turn, the square is not empty, and the top card of the square belongs to the player. The subsequent intention condition appiles if there have been previous intentions in the turn, the square is evicted, and the evicted card, which is the top card of the square, is the active card or does not belong to the player. To intend, the player must decide a direction for a square, where the target square must not be a moving square nor a special square of a player other than the owner of the top card of the moving square. All intentions transition the state of the square from idling to moving.

*Note, this means that if the top card of an evicted square belongs to the player and is not the active card, the turn is invalid.*

A player can only fulfil a square if and only if the square is moving, and the target square is empty or the top card of the target square is not of lesser rank than the top card of the moving square. To fulfil, the player must move the top card of the square according to the moving direction. This transitions the state of the square from moving to idling.

*Note, these rules means that only your own cards can block you. In particular, if player intends a square to a non-empty target square with none of that player's cards, one of two possibilities can hold. If the top card of the target square is not of lesser rank than the moving square, the move can be fulfilled without eviction. If the top card of the target square is of lesser rank than the moving square, it will be possible to evict the target square, which enables an intention and fulfilment to get the top card of the target square out of the way. This repeats until the first possibility can hold. The caveat is that there might not be anywhere for the top card of the target square to go, but again, this will only be the case due that card being blocked by the active player's cards.*

*Note, there are no restrictions on when a start or finish can be entered or exited, only which cards can enter and exit. This even means that these special squares can have cards going in or out even when the owner of those cards is not taking the current turn.*

*Note, these rules might very well let the player end up in a dead end if the player does not look ahead during the turn. In particular, if a moving square targets a square with a top card belonging to the active player of lesser rank than the top card of the moving square, there is nothing to be done, and the player must go back. However, it is to the total discretion of the player the order in which to resolve evicted and moving squares in order to complete the turn.*

*Note, while the terms moving and idling only apply to squares and not to cards, they should really be thought of as describing the top card of the square.*

*Note, all moving squares are not empty, and the top card of a moving square is the "moving card"*
