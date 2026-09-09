# 018 - Move the selection to the box below

## Story

Bob has a box with another box beside it on the right, and nothing below. He
presses `j` and the selection stays put — `j` no longer carries him sideways.
With a box stacked underneath, `j` takes him down to it.

## Acceptance Criteria

- `j` moves the selection to the box directly below it.
- `j` with no box directly below it leaves the selection where it is.
- `j` on a canvas with no boxes does nothing.

## Technical Design
