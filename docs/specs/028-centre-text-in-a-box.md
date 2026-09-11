# Centre text in a box

As someone sketching a diagram, I want the text in every box centred, so my
diagram stays tidy when a neighbouring box grows wider.

## Acceptance Criteria

1. A box's text sits centred between its left and right borders.
2. When one box grows wide enough to widen its neighbours, the neighbours' text
   re-centres instead of staying against the left border.
3. When the leftover space can't be split evenly, the extra space goes on the
   left of the text.
4. In insert mode, the typing cursor sits immediately after the last character
   of the centred text, and moves as the text re-centres.
5. In a box with no text, the cursor sits in the middle of the box.

## Technical Design
