# Double the horizontal gap

**Story:** As a user, I want more horizontal space between sibling boxes so my diagram is easier to read.

## Acceptance Criteria

- The horizontal gap between sibling boxes doubles from 4 columns to 8 columns.
- Arrows drawn between a parent and child still span the full gap width.
- Vertical gap and box dimensions are unchanged.

## Technical Design

- Change the `GAP_WIDTH` constant in `sketch/layout.py` from `4` to `8`.
- No other code changes are needed: `column_tracks` sizes the arrow-gap track from `GAP_WIDTH`, and `emit` sizes the arrow placement's width from `GAP_WIDTH`, so both the layout spacing and the arrow span scale together automatically.
- The gap stays a hardcoded constant rather than becoming configurable — no story requires per-diagram or per-theme control over it.
- Existing tests (`tests/test_layout.py`) already assert against the `GAP_WIDTH` constant rather than a literal `4`, so they validate the new value without modification.
