# Status bar shows mode

## User Story

Bob opens dre and always sees a status bar at the bottom of the screen, visually separated from the canvas by a different background color, showing whether he's currently EDITING or COMMANDING.

## Acceptance Criteria

- The status bar is visible at all times, not just during the Save-as prompt.
- The status bar's background color is visibly different from the terminal's default background, separating it from the canvas.
- When in Insert mode, the status bar shows "EDITING".
- When in Command mode, the status bar shows "COMMANDING".
- When in the Save-as prompt, the status bar shows "Save as: ..." as it does today (unchanged).

## Technical Design

