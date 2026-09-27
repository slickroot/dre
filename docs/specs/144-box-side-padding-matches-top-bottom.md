# Box side padding matches top/bottom padding

Doug draws a box with a label in dre. Looking at it, he notices the space
between the label and the box's left/right edges now visually matches the
space between the label and the top/bottom edges, instead of looking
cramped on the sides.

## Acceptance Criteria

- Every box (regardless of label length) gets one extra character cell of
  horizontal spacing added on the left and right of the label, on top of
  what's there today.
- Eyeballing any box in the terminal, the left/right padding now looks
  roughly even with the top/bottom padding, instead of visibly tighter.

## Technical Design
