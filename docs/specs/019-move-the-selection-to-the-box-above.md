# 019 - Move the selection to the box above

## Story

Bob has a box with another box beside it on the left, and nothing above. He
presses `k` and the selection stays put. With a box stacked overhead, `k` takes
him up to it.

## Acceptance Criteria

- `k` moves the selection to the box directly above it.
- `k` with no box directly above it leaves the selection where it is.
- `k` on a canvas with no boxes does nothing.

## Technical Design
