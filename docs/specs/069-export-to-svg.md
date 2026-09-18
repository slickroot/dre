# 069: Export to SVG

## User Story

As a user, I can run `dre --svg diagram.dre` and get a crisp, vector `diagram.svg` that looks
just like my diagram in the terminal — same boxes, labels, colours, fills, rounded corners,
and arrows — without ever opening the editor.

## Acceptance Criteria

- Running `dre --svg diagram.dre` creates `diagram.svg` next to the `.dre` file and exits
- The SVG matches the terminal view: boxes, labels, colours, fills, rounded corners, and arrows
- The SVG has no cursor or selection overlay — it's the saved document, not the editing session
- A missing file prints `no such file: <path>` and exits with a failure code
- A corrupted file prints `<path>: not a valid diagram` and exits with a failure code

## Technical Design