# 017 - Move the selection to the box on the left

## Story

Bob has two boxes side by side, with the selection on the right one. He presses
`h` and the selection jumps to the box beside it. He presses `i` and types, and
it is that left-hand box that changes.

## Acceptance Criteria

- `h` moves the selection to the box directly beside it on the left.
- `h` with no box directly beside it on the left leaves the selection where it is.
- `h` on a canvas with no boxes does nothing.

## Technical Design
