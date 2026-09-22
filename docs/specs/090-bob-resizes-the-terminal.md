# Bob resizes the terminal

Bob opened `dre` and drew a diagram. He resized his terminal window, and the
diagram redrew live to fit the new size — centered correctly, with no
corrupted or leftover content on screen.

## Acceptance Criteria

- When the terminal window is resized, `dre` detects the size change without
  requiring a keypress or other action.
- The diagram redraws live, using the new terminal dimensions, re-centered to
  fit the new space (in both growing and shrinking directions).
- The status line is redrawn at the correct row for the new terminal size.
- No stale/leftover content remains on screen outside the new bounds after a
  resize.

## Technical Design
