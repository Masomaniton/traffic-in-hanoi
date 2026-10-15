# UI

We should have clean minimalist graphics and intuitive controls which resemble the feel of editting a document.

## Borders and Corners

Everything should have a subtle border for now. The three layers elements are the board, squares, and cards. All corners should be rounded such that the corresponding circles for corners should all have the same center if the elements are "aligned". This should create a clean nesting feel, with each layer of elements having a constant increase in radii from the one in front.

## Board, Squares, and Piles

The board should be green. Idling squares should be a slightly darker color than the board. Moving squares should be a slightly darker color than idling squares. The squares should actually be square. Piles are rendered bottom to top. The piles show at most SANDWICH_CARDS + 2 cards, where SANDWICH_CARDS is an adjustable unsigned integer for the number of cards between the leftmost and rightmost cards. The cards are not squares. The topmost card should always be aligned with the right corners of the square. When a new card stacks on top, each card below slides leftward by a constant amount, until the card becomes aligned with the left corners of the square. If there is a card aligned with the left corners of the square, any card below is completely invisible. There will also be a fully fanned view for piles. In this view, the bottommost card is aligned with the left corners of the square, and the cards extend out as far right as needed, perhaps past the border of the square.

## Controls

- `backspace` - undo a move in an incomplete turn
- `enter` - confirm a turn is complete
- `left/right arrow` - step backward/forward an action in the history
- `up/down arrow` - step backward/forward a turn in the history

The mouse should handle the main board actions. To perform an intention, we click the source square and then click the target square. To perform a fulfilment, we click the moving square. In order to fan out a pile, we can can click and hold. The pile will remained fanned until there is another click, anywhere.

## Cards

Cards should display the rank and first letter of the player in the top right corner. The cards should also have 180 degree symmetry, as with real cards. We can worry about iconography and color concerns until later.
