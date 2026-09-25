## Story

Doug runs `dre docs/plans.dre`. He sees his diagram, and in the bottom-right corner of the screen a small label saying `plans`. The big coloured bar is gone. He always knows which diagram he's working on, and the diagram has more room.

## Acceptance Criteria

- The 3-row bottom bar is gone.
- When a file is open, its name appears in the bottom-right corner on a single row. The label takes only the width of the name, not the whole width of the window.
- The name is the file name without the folder and without `.dre`. For `docs/plans.dre` it shows `plans`.
- When no file is open, the corner stays empty.
- The bottom row is reserved for the name. The diagram is centred in the rows above it and cropped there, so it never draws into that row.
- This applies to the terminal only. The web view and SVG export are unchanged for now.

## Technical Design
