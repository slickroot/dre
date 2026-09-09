# 015 - Move the selection to the box on the right

## Story

Bob has two boxes side by side, with the selection on the left one. He presses
`l` and the selection jumps to the box beside it. He presses `i` and types, and
it is that right-hand box that changes.

## Acceptance Criteria

- `l` moves the selection to the box directly beside it on the right.
- `l` with no box directly beside it on the right leaves the selection where it is.
- `l` on a canvas with no boxes does nothing.

## Technical Design
