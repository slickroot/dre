# Arrows don't move boxes

## User Story

As a user, when I connect two boxes with an arrow, I want the arrow to only connect them — never move the boxes — and to always point in the correct direction, so that adding a connection never disrupts my layout.

## Acceptance Criteria

- Given boxes arranged in a horizontal row, when I connect two of them with an arrow, the boxes' positions in the row do not change.
- Given boxes arranged in a vertical column, when I connect two of them with an arrow, the boxes' positions do not change (existing behavior, no regression).
- The arrow always points from the source box to the target box in the correct direction, regardless of layout orientation.

## Technical Design
