# 023 - Grow a child box with an arrow

## Story

Bob has a box selected. He presses `bj` and a new box appears below it, already
joined to it by an arrow pointing down — no separate `a` step to connect them.
`bl` does the same to the right.

## Acceptance Criteria

- `bj` on a selected box creates a new box below it, connected by an arrow
  pointing down.
- `bl` on a selected box creates a new box to the right, connected by an arrow
  pointing right.
- `b` on an empty canvas still creates the first box with no arrow.
- The new box becomes the selection and the app enters insert mode, as today.

## Technical Design
